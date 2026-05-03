//! LSO DB — SQLCipher storage layer for persistent data.

mod error;
mod migrations;

pub use error::{DbError, DbResult};

use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use lso_core::{
    AuditEntry, Recommendation, RecommendationStatus, RiskLevel, SystemMetric,
};
use rusqlite::Connection;
use uuid::Uuid;

fn parse_enum<T: std::str::FromStr<Err = String>>(
    s: &str,
    col: usize,
) -> Result<T, rusqlite::Error> {
    s.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            col,
            rusqlite::types::Type::Text,
            Box::<dyn std::error::Error + Send + Sync>::from(e),
        )
    })
}

/// Encrypted SQLite database backed by SQLCipher.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) an encrypted database at `path` using `key`.
    pub fn open(path: &Path, key: &str) -> DbResult<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "key", key)?;
        // Force a read to verify the key is correct.
        conn.pragma_query_value(None, "schema_version", |_row| Ok(()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Apply all pending migrations.
    pub fn migrate(&self) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        migrations::run(&conn)
    }

    /// Batch-insert system metrics.
    pub fn store_metrics(&self, metrics: &[SystemMetric]) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO metrics (id, probe_id, name, value, unit, collected_at, platform)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for m in metrics {
                stmt.execute(rusqlite::params![
                    m.id.to_string(),
                    m.probe_id,
                    m.name,
                    m.value,
                    m.unit,
                    m.collected_at.to_rfc3339(),
                    m.platform,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Insert or update a recommendation.
    pub fn store_recommendation(&self, rec: &Recommendation) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        conn.execute(
            "INSERT OR REPLACE INTO recommendations (id, rule_id, title, description, risk_level, category, target, rollback_plan, status, rejection_reason, created_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                rec.id.to_string(),
                rec.rule_id,
                rec.title,
                rec.description,
                rec.risk_level.as_str(),
                rec.category,
                rec.target,
                rec.rollback_plan,
                rec.status.as_str(),
                rec.rejection_reason,
                rec.created_at.to_rfc3339(),
                rec.resolved_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Append an audit log entry (immutable once written).
    pub fn store_audit_entry(&self, entry: &AuditEntry) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        conn.execute(
            "INSERT INTO audit_log (id, timestamp, action, target, risk_level, user_approved, snapshot_id, result, rollback_available)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                entry.id.to_string(),
                entry.timestamp.to_rfc3339(),
                entry.action,
                entry.target,
                entry.risk_level.as_str(),
                entry.user_approved as i32,
                entry.snapshot_id,
                entry.result.as_str(),
                entry.rollback_available as i32,
            ],
        )?;
        Ok(())
    }

    /// Get recent metrics for a given probe, ordered newest-first.
    pub fn get_recent_metrics(&self, probe_id: &str, limit: u32) -> DbResult<Vec<SystemMetric>> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, probe_id, name, value, unit, collected_at, platform
             FROM metrics
             WHERE probe_id = ?1
             ORDER BY collected_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![probe_id, limit], |row| {
            Ok(SystemMetric {
                id: row
                    .get::<_, String>(0)?
                    .parse::<Uuid>()
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?,
                probe_id: row.get(1)?,
                name: row.get(2)?,
                value: row.get(3)?,
                unit: row.get(4)?,
                collected_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e)))?
                    .with_timezone(&Utc),
                platform: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get all recommendations, optionally filtered by status.
    pub fn get_recommendations(
        &self,
        status_filter: Option<RecommendationStatus>,
    ) -> DbResult<Vec<Recommendation>> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match status_filter {
            Some(status) => (
                "SELECT id, rule_id, title, description, risk_level, category, target, rollback_plan, status, rejection_reason, created_at, resolved_at
                 FROM recommendations WHERE status = ?1 ORDER BY created_at DESC",
                vec![Box::new(status.as_str().to_string())],
            ),
            None => (
                "SELECT id, rule_id, title, description, risk_level, category, target, rollback_plan, status, rejection_reason, created_at, resolved_at
                 FROM recommendations ORDER BY created_at DESC",
                vec![],
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_recommendation)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Get all recommendations with status 'pending' (convenience method).
    pub fn get_pending_recommendations(&self) -> DbResult<Vec<Recommendation>> {
        self.get_recommendations(Some(RecommendationStatus::Pending))
    }

    /// Find a recommendation by its dedup key (rule_id + target).
    pub fn find_by_dedup_key(&self, rule_id: &str, target: &str) -> DbResult<Option<Recommendation>> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, rule_id, title, description, risk_level, category, target, rollback_plan, status, rejection_reason, created_at, resolved_at
             FROM recommendations WHERE rule_id = ?1 AND target = ?2 AND status = 'pending' LIMIT 1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![rule_id, target], |row| {
            Self::row_to_recommendation(row)
        })?;
        match rows.next() {
            Some(Ok(rec)) => Ok(Some(rec)),
            Some(Err(e)) => Err(DbError::from(e)),
            None => Ok(None),
        }
    }

    /// Get a single recommendation by ID.
    pub fn get_recommendation(&self, id: &str) -> DbResult<Option<Recommendation>> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, rule_id, title, description, risk_level, category, target, rollback_plan, status, rejection_reason, created_at, resolved_at
             FROM recommendations WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], |row| {
            Self::row_to_recommendation(row)
        })?;
        match rows.next() {
            Some(Ok(rec)) => Ok(Some(rec)),
            Some(Err(e)) => Err(DbError::from(e)),
            None => Ok(None),
        }
    }

    fn row_to_recommendation(row: &rusqlite::Row<'_>) -> Result<Recommendation, rusqlite::Error> {
        let resolved_str: Option<String> = row.get(11)?;
        Ok(Recommendation {
            id: row
                .get::<_, String>(0)?
                .parse::<Uuid>()
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?,
            rule_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            risk_level: parse_enum::<RiskLevel>(&row.get::<_, String>(4)?, 4)?,
            category: row.get(5)?,
            target: row.get(6)?,
            rollback_plan: row.get(7)?,
            status: parse_enum::<RecommendationStatus>(&row.get::<_, String>(8)?, 8)?,
            rejection_reason: row.get(9)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e)))?
                .with_timezone(&Utc),
            resolved_at: resolved_str
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lso_core::ActionResult;

    #[test]
    fn open_and_migrate() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path, "test-key").unwrap();
        db.migrate().unwrap();
        // Migrating again should be idempotent.
        db.migrate().unwrap();
    }

    #[test]
    fn wrong_key_fails() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let db = Database::open(&db_path, "correct-key").unwrap();
        db.migrate().unwrap();
        drop(db);

        let result = Database::open(&db_path, "wrong-key");
        assert!(result.is_err(), "opening with wrong key should fail");
    }

    #[test]
    fn metrics_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        let metric = SystemMetric {
            id: Uuid::new_v4(),
            probe_id: "disk.usage".into(),
            name: "disk_used_bytes".into(),
            value: 1024.0,
            unit: Some("bytes".into()),
            collected_at: Utc::now(),
            platform: "macos".into(),
        };
        db.store_metrics(std::slice::from_ref(&metric)).unwrap();

        let fetched = db.get_recent_metrics("disk.usage", 10).unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].id, metric.id);
        assert!((fetched[0].value - 1024.0).abs() < f64::EPSILON);
    }

    #[test]
    fn recommendation_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        let rec = Recommendation {
            id: Uuid::new_v4(),
            rule_id: "disk.cleanup".into(),
            title: "Clean temp files".into(),
            description: "Remove stale temp files to reclaim 2 GB".into(),
            risk_level: RiskLevel::Low,
            category: "disk".into(),
            target: "/tmp".into(),
            rollback_plan: Some("Restore from snapshot".into()),
            status: RecommendationStatus::Pending,
            rejection_reason: None,
            created_at: Utc::now(),
            resolved_at: None,
        };
        db.store_recommendation(&rec).unwrap();

        let pending = db.get_pending_recommendations().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Clean temp files");
        assert_eq!(pending[0].rule_id, "disk.cleanup");
        assert_eq!(pending[0].target, "/tmp");
    }

    #[test]
    fn audit_entry_insert() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: "delete_temp_files".into(),
            target: "/tmp/stale".into(),
            risk_level: RiskLevel::Medium,
            user_approved: true,
            snapshot_id: Some("snap-001".into()),
            result: ActionResult::Success,
            rollback_available: true,
        };
        db.store_audit_entry(&entry).unwrap();
    }
}
