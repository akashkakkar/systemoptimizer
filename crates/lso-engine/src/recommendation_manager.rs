//! Recommendation lifecycle manager — scan, deduplicate, expire, approve, reject.

use lso_core::{
    ApprovalResponse, ProbeResult, Recommendation, RecommendationStatus, RiskLevel,
};
use lso_db::Database;
use tracing::info;

use crate::evaluator::RuleEngine;
use crate::EngineError;

/// Manages the full recommendation lifecycle on top of RuleEngine + Database.
pub struct RecommendationManager {
    engine: RuleEngine,
    db: Database,
}

impl RecommendationManager {
    pub fn new(engine: RuleEngine, db: Database) -> Self {
        Self { engine, db }
    }

    /// Store probe metrics in the database.
    pub fn store_metrics(&self, metrics: &[lso_core::SystemMetric]) -> Result<(), EngineError> {
        self.db
            .store_metrics(metrics)
            .map_err(|e| EngineError::Storage(e.to_string()))
    }

    /// Run all rules against probe data, deduplicate, and store new recommendations.
    pub fn scan(&self, probes: &[ProbeResult]) -> Result<Vec<Recommendation>, EngineError> {
        let mut raw_recs = self.engine.evaluate(probes);

        // Apply risk modifiers based on target context
        for rec in &mut raw_recs {
            self.apply_risk_modifiers(rec, probes);
        }

        let deduped = self.deduplicate(raw_recs)?;

        for rec in &deduped {
            self.db
                .store_recommendation(rec)
                .map_err(|e| EngineError::Storage(e.to_string()))?;
        }

        info!(count = deduped.len(), "scan produced new recommendations");
        Ok(deduped)
    }

    /// Deduplicate: if a pending recommendation with the same rule+target exists, skip it.
    fn deduplicate(
        &self,
        candidates: Vec<Recommendation>,
    ) -> Result<Vec<Recommendation>, EngineError> {
        let mut results = Vec::new();
        for candidate in candidates {
            let existing = self
                .db
                .find_by_dedup_key(&candidate.rule_id, &candidate.target)
                .map_err(|e| EngineError::Storage(e.to_string()))?;

            if existing.is_none() {
                results.push(candidate);
            }
        }
        Ok(results)
    }

    /// Expire recommendations whose underlying condition no longer holds.
    pub fn expire_resolved(
        &self,
        current_probes: &[ProbeResult],
    ) -> Result<usize, EngineError> {
        let pending = self
            .db
            .get_pending_recommendations()
            .map_err(|e| EngineError::Storage(e.to_string()))?;

        let current_recs = self.engine.evaluate(current_probes);
        let mut expired_count = 0;

        for mut rec in pending {
            let still_relevant = current_recs
                .iter()
                .any(|r| r.rule_id == rec.rule_id && r.target == rec.target);

            if !still_relevant && rec.expire() {
                self.db
                    .store_recommendation(&rec)
                    .map_err(|e| EngineError::Storage(e.to_string()))?;
                expired_count += 1;
            }
        }

        info!(count = expired_count, "expired resolved recommendations");
        Ok(expired_count)
    }

    /// Get all recommendations, optionally filtered by status.
    pub fn list(
        &self,
        status: Option<RecommendationStatus>,
    ) -> Result<Vec<Recommendation>, EngineError> {
        self.db
            .get_recommendations(status)
            .map_err(|e| EngineError::Storage(e.to_string()))
    }

    /// Approve a recommendation. Returns approval info with double-confirm flag.
    pub fn approve(&self, id: &str) -> Result<ApprovalResponse, EngineError> {
        let mut rec = self
            .db
            .get_recommendation(id)
            .map_err(|e| EngineError::Storage(e.to_string()))?
            .ok_or_else(|| EngineError::Storage(format!("recommendation not found: {id}")))?;

        let requires_double_confirm = rec.risk_level == RiskLevel::High;

        if !rec.approve() {
            return Err(EngineError::Storage(format!(
                "cannot approve recommendation {id} (status={:?}, risk={:?})",
                rec.status, rec.risk_level
            )));
        }

        self.db
            .store_recommendation(&rec)
            .map_err(|e| EngineError::Storage(e.to_string()))?;

        Ok(ApprovalResponse {
            id: rec.id,
            status: rec.status,
            requires_double_confirm,
        })
    }

    /// Reject a recommendation with optional reason.
    pub fn reject(&self, id: &str, reason: Option<String>) -> Result<(), EngineError> {
        let mut rec = self
            .db
            .get_recommendation(id)
            .map_err(|e| EngineError::Storage(e.to_string()))?
            .ok_or_else(|| EngineError::Storage(format!("recommendation not found: {id}")))?;

        if !rec.reject(reason) {
            return Err(EngineError::Storage(format!(
                "cannot reject recommendation {id} (status={:?})",
                rec.status
            )));
        }

        self.db
            .store_recommendation(&rec)
            .map_err(|e| EngineError::Storage(e.to_string()))
    }

    /// Dismiss a recommendation (shorthand for reject with "dismissed" reason).
    pub fn dismiss(&self, id: &str) -> Result<(), EngineError> {
        self.reject(id, Some("dismissed by user".into()))
    }

    /// Apply risk modifiers: system drive → +1 level, boot partition → Critical.
    fn apply_risk_modifiers(&self, rec: &mut Recommendation, probes: &[ProbeResult]) {
        let target = &rec.target;
        let is_system_drive = self.is_system_drive(target, probes);
        let is_boot_partition = self.is_boot_partition(target, probes);

        if is_boot_partition {
            rec.risk_level = RiskLevel::Critical;
        } else if is_system_drive {
            rec.risk_level = rec.risk_level.elevate();
        }
    }

    fn is_system_drive(&self, target: &str, _probes: &[ProbeResult]) -> bool {
        let t = target.to_lowercase();
        t == "/" || t.starts_with("/system") || t.starts_with("c:\\")
    }

    fn is_boot_partition(&self, target: &str, _probes: &[ProbeResult]) -> bool {
        let t = target.to_lowercase();
        t.starts_with("/boot") || t.contains("efi")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::{Condition, Operator, Rule, RuleMetadata, RuleRecommendation};
    use chrono::Utc;
    use lso_core::{Platform, SystemMetric};
    use uuid::Uuid;

    struct TestDb {
        db: Database,
        _dir: tempfile::TempDir,
    }

    fn test_db() -> TestDb {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "test-key").unwrap();
        db.migrate().unwrap();
        TestDb { db, _dir: dir }
    }

    fn test_rules() -> Vec<Rule> {
        vec![Rule {
            id: "disk.usage_high".into(),
            name: "High disk usage".into(),
            description: "Disk exceeds threshold".into(),
            category: "disk".into(),
            enabled: true,
            condition: Condition {
                probe: "disk.usage".into(),
                metric: "usage_percent".into(),
                operator: Operator::GreaterThan,
                threshold: 80.0,
            },
            recommendation: RuleRecommendation {
                title: "Clean up {target}".into(),
                description: "{target} at {value}%".into(),
                risk_level: RiskLevel::Low,
                category: "cleanup".into(),
            },
            metadata: RuleMetadata::default(),
        }]
    }

    fn make_probes(target: &str, usage: f64) -> Vec<ProbeResult> {
        vec![ProbeResult {
            probe_id: "disk.usage".into(),
            metrics: vec![SystemMetric {
                id: Uuid::new_v4(),
                probe_id: "disk.usage".into(),
                name: format!("{target}::usage_percent"),
                value: usage,
                unit: Some("%".into()),
                collected_at: Utc::now(),
                platform: "macos".into(),
            }],
            collected_at: Utc::now(),
            platform: Platform::MacOS,
        }]
    }

    #[test]
    fn scan_produces_and_stores_recommendations() {
        let tdb = test_db();
        let engine = RuleEngine::from_rules(test_rules(), Platform::MacOS);
        let mgr = RecommendationManager::new(engine, tdb.db);

        let probes = make_probes("/home", 92.0);
        let recs = mgr.scan(&probes).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].rule_id, "disk.usage_high");

        let stored = mgr.list(None).unwrap();
        assert_eq!(stored.len(), 1);
    }

    #[test]
    fn scan_deduplicates_pending() {
        let tdb = test_db();
        let engine = RuleEngine::from_rules(test_rules(), Platform::MacOS);
        let mgr = RecommendationManager::new(engine, tdb.db);

        let probes = make_probes("/home", 92.0);
        mgr.scan(&probes).unwrap();
        let second = mgr.scan(&probes).unwrap();
        assert_eq!(second.len(), 0);

        let all = mgr.list(None).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn expire_resolved_works() {
        let tdb = test_db();
        let engine = RuleEngine::from_rules(test_rules(), Platform::MacOS);
        let mgr = RecommendationManager::new(engine, tdb.db);

        let probes = make_probes("/home", 92.0);
        mgr.scan(&probes).unwrap();

        // Usage dropped below threshold
        let resolved = make_probes("/home", 50.0);
        let expired = mgr.expire_resolved(&resolved).unwrap();
        assert_eq!(expired, 1);

        let pending = mgr.list(Some(RecommendationStatus::Pending)).unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn system_drive_elevates_risk() {
        let tdb = test_db();
        let engine = RuleEngine::from_rules(test_rules(), Platform::MacOS);
        let mgr = RecommendationManager::new(engine, tdb.db);

        let probes = make_probes("/", 92.0);
        let recs = mgr.scan(&probes).unwrap();
        assert_eq!(recs[0].risk_level, RiskLevel::Medium); // Low → Medium
    }

    #[test]
    fn boot_partition_becomes_critical() {
        let tdb = test_db();
        let engine = RuleEngine::from_rules(test_rules(), Platform::MacOS);
        let mgr = RecommendationManager::new(engine, tdb.db);

        let probes = make_probes("/boot", 92.0);
        let recs = mgr.scan(&probes).unwrap();
        assert_eq!(recs[0].risk_level, RiskLevel::Critical);
    }

    #[test]
    fn approve_and_reject() {
        let tdb = test_db();
        let engine = RuleEngine::from_rules(test_rules(), Platform::MacOS);
        let mgr = RecommendationManager::new(engine, tdb.db);

        let probes = make_probes("/home", 92.0);
        let recs = mgr.scan(&probes).unwrap();
        let id = recs[0].id.to_string();

        let result = mgr.approve(&id).unwrap();
        assert_eq!(result.status, RecommendationStatus::Approved);
        assert!(!result.requires_double_confirm);
    }

    #[test]
    fn critical_cannot_be_approved() {
        let tdb = test_db();
        let engine = RuleEngine::from_rules(test_rules(), Platform::MacOS);
        let mgr = RecommendationManager::new(engine, tdb.db);

        let probes = make_probes("/boot", 92.0);
        let recs = mgr.scan(&probes).unwrap();
        let id = recs[0].id.to_string();

        let result = mgr.approve(&id);
        assert!(result.is_err());
    }

    #[test]
    fn reject_with_reason() {
        let tdb = test_db();
        let engine = RuleEngine::from_rules(test_rules(), Platform::MacOS);
        let mgr = RecommendationManager::new(engine, tdb.db);

        let probes = make_probes("/home", 92.0);
        let recs = mgr.scan(&probes).unwrap();
        let id = recs[0].id.to_string();

        mgr.reject(&id, Some("not now".into())).unwrap();
        let all = mgr.list(None).unwrap();
        assert_eq!(all[0].status, RecommendationStatus::Rejected);
        assert_eq!(all[0].rejection_reason.as_deref(), Some("not now"));
    }
}
