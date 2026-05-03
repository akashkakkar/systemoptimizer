//! LSO DB — SQLCipher storage layer for persistent data.

mod error;
mod explanation_cache;
mod migrations;

pub use error::{DbError, DbResult};
pub use explanation_cache::SqlCipherExplanationCache;

use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use lso_core::{
    ActionResult, AuditEntry, AuditFilter, AuditPage, ExportFormat, Recommendation,
    RecommendationStatus, RiskLevel, SystemMetric,
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
            "INSERT INTO audit_log (id, timestamp, action, target, category, risk_level, user_approved, snapshot_id, result, details, rollback_available)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                entry.id.to_string(),
                entry.timestamp.to_rfc3339(),
                entry.action,
                entry.target,
                entry.category,
                entry.risk_level.as_str(),
                entry.user_approved as i32,
                entry.snapshot_id,
                entry.result.as_str(),
                entry.details,
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

    /// Query audit log with filters and pagination.
    pub fn get_audit_log(&self, filter: &AuditFilter) -> DbResult<AuditPage> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;

        let mut where_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(ref status) = filter.status {
            where_clauses.push(format!("result = ?{idx}"));
            params.push(Box::new(status.as_str().to_string()));
            idx += 1;
        }
        if let Some(ref risk) = filter.risk_level {
            where_clauses.push(format!("risk_level = ?{idx}"));
            params.push(Box::new(risk.as_str().to_string()));
            idx += 1;
        }
        if let Some(ref cat) = filter.category {
            where_clauses.push(format!("category = ?{idx}"));
            params.push(Box::new(cat.clone()));
            idx += 1;
        }
        if let Some(ref from) = filter.date_from {
            where_clauses.push(format!("timestamp >= ?{idx}"));
            params.push(Box::new(from.to_rfc3339()));
            idx += 1;
        }
        if let Some(ref to) = filter.date_to {
            where_clauses.push(format!("timestamp <= ?{idx}"));
            params.push(Box::new(to.to_rfc3339()));
            idx += 1;
        }
        if let Some(ref search) = filter.search {
            where_clauses.push(format!("(target LIKE ?{idx} OR action LIKE ?{idx})"));
            params.push(Box::new(format!("%{search}%")));
            idx += 1;
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM audit_log {where_sql}");
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let total_count: u64 = conn
            .query_row(&count_sql, param_refs.as_slice(), |row| row.get(0))
            .map_err(DbError::from)?;

        let per_page = filter.per_page.unwrap_or(50);
        let page = filter.page.unwrap_or(1).max(1);
        let total_pages = if total_count == 0 {
            1
        } else {
            ((total_count as f64) / (per_page as f64)).ceil() as u32
        };
        let offset = (page - 1) * per_page;

        let query_sql = format!(
            "SELECT id, timestamp, action, target, category, risk_level, user_approved, snapshot_id, result, details, rollback_available
             FROM audit_log {where_sql}
             ORDER BY timestamp DESC
             LIMIT ?{idx} OFFSET ?{}",
            idx + 1
        );
        params.push(Box::new(per_page));
        params.push(Box::new(offset));

        let param_refs2: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&query_sql)?;
        let rows = stmt.query_map(param_refs2.as_slice(), Self::row_to_audit_entry)?;
        let entries: Vec<AuditEntry> =
            rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)?;

        Ok(AuditPage {
            entries,
            total_count,
            page,
            per_page,
            total_pages,
        })
    }

    /// Get a single audit entry by ID.
    pub fn get_audit_entry(&self, id: &str) -> DbResult<Option<AuditEntry>> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, action, target, category, risk_level, user_approved, snapshot_id, result, details, rollback_available
             FROM audit_log WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], Self::row_to_audit_entry)?;
        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(DbError::from(e)),
            None => Ok(None),
        }
    }

    /// Mark an audit entry as no longer rollback-eligible.
    pub fn mark_rolled_back(&self, id: &str) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        conn.execute(
            "UPDATE audit_log SET rollback_available = 0 WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    /// Export audit entries matching a date range to JSON or CSV.
    pub fn export_audit_log(
        &self,
        format: ExportFormat,
        filter: &AuditFilter,
        dest: &Path,
    ) -> DbResult<()> {
        let full_filter = AuditFilter {
            per_page: Some(u32::MAX),
            page: Some(1),
            ..filter.clone()
        };
        let page = self.get_audit_log(&full_filter)?;

        match format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&page.entries)
                    .map_err(|e| DbError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e))))?;
                std::fs::write(dest, json)?;
            }
            ExportFormat::Csv => {
                let file = std::fs::File::create(dest)?;
                let mut wtr = csv::Writer::from_writer(file);
                wtr.write_record([
                    "id", "timestamp", "action", "target", "category", "risk_level",
                    "user_approved", "snapshot_id", "result", "details", "rollback_available",
                ]).map_err(|e| DbError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e))))?;
                for entry in &page.entries {
                    wtr.write_record([
                        &entry.id.to_string(),
                        &entry.timestamp.to_rfc3339(),
                        &entry.action,
                        &entry.target,
                        &entry.category,
                        entry.risk_level.as_str(),
                        &entry.user_approved.to_string(),
                        entry.snapshot_id.as_deref().unwrap_or(""),
                        entry.result.as_str(),
                        entry.details.as_deref().unwrap_or(""),
                        &entry.rollback_available.to_string(),
                    ]).map_err(|e| DbError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e))))?;
                }
                wtr.flush().map_err(|e| DbError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e))))?;
            }
        }
        Ok(())
    }

    /// Look up a cached explanation for `rec_id`. Returns `None` if no
    /// row exists or if the stored `data_hash` differs from `data_hash`
    /// (the cached entry is stale and should be regenerated).
    pub fn get_explanation(&self, rec_id: &str, data_hash: &str) -> DbResult<Option<String>> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT data_hash, explanation FROM explanation_cache WHERE rec_id = ?1",
                rusqlite::params![rec_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        Ok(row
            .filter(|(h, _)| h == data_hash)
            .map(|(_, e)| e))
    }

    /// Insert or replace the cached explanation for `rec_id`.
    pub fn put_explanation(
        &self,
        rec_id: &str,
        data_hash: &str,
        explanation: &str,
    ) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO explanation_cache (rec_id, data_hash, explanation, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(rec_id) DO UPDATE SET
                data_hash = excluded.data_hash,
                explanation = excluded.explanation,
                updated_at = excluded.updated_at",
            rusqlite::params![rec_id, data_hash, explanation, now],
        )?;
        Ok(())
    }

    /// Drop any cached explanation for `rec_id`.
    pub fn invalidate_explanation(&self, rec_id: &str) -> DbResult<()> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        conn.execute(
            "DELETE FROM explanation_cache WHERE rec_id = ?1",
            rusqlite::params![rec_id],
        )?;
        Ok(())
    }

    fn row_to_audit_entry(row: &rusqlite::Row<'_>) -> Result<AuditEntry, rusqlite::Error> {
        Ok(AuditEntry {
            id: row
                .get::<_, String>(0)?
                .parse::<Uuid>()
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
            timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&Utc),
            action: row.get(2)?,
            target: row.get(3)?,
            category: row.get(4)?,
            risk_level: parse_enum::<RiskLevel>(&row.get::<_, String>(5)?, 5)?,
            user_approved: row.get::<_, i32>(6)? != 0,
            snapshot_id: row.get(7)?,
            result: parse_enum::<ActionResult>(&row.get::<_, String>(8)?, 8)?,
            details: row.get(9)?,
            rollback_available: row.get::<_, i32>(10)? != 0,
        })
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

    fn make_entry(action: &str, result: ActionResult, category: &str) -> AuditEntry {
        AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: action.into(),
            target: "/tmp/stale".into(),
            category: category.into(),
            risk_level: RiskLevel::Medium,
            user_approved: true,
            snapshot_id: Some("snap-001".into()),
            result,
            details: Some("test details".into()),
            rollback_available: result == ActionResult::Success,
        }
    }

    #[test]
    fn audit_entry_insert_and_query() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        let entry = make_entry("delete_temp_files", ActionResult::Success, "cleanup");
        db.store_audit_entry(&entry).unwrap();

        let page = db.get_audit_log(&AuditFilter::default()).unwrap();
        assert_eq!(page.total_count, 1);
        assert_eq!(page.entries[0].action, "delete_temp_files");
        assert_eq!(page.entries[0].category, "cleanup");
        assert_eq!(page.entries[0].details.as_deref(), Some("test details"));
    }

    #[test]
    fn audit_log_filter_by_status() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        db.store_audit_entry(&make_entry("a", ActionResult::Success, "cleanup")).unwrap();
        db.store_audit_entry(&make_entry("b", ActionResult::Failed, "cleanup")).unwrap();

        let filter = AuditFilter {
            status: Some(ActionResult::Success),
            ..Default::default()
        };
        let page = db.get_audit_log(&filter).unwrap();
        assert_eq!(page.total_count, 1);
        assert_eq!(page.entries[0].action, "a");
    }

    #[test]
    fn audit_log_pagination() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        for i in 0..5 {
            db.store_audit_entry(&make_entry(&format!("action_{i}"), ActionResult::Success, "cleanup")).unwrap();
        }

        let filter = AuditFilter {
            per_page: Some(2),
            page: Some(1),
            ..Default::default()
        };
        let page = db.get_audit_log(&filter).unwrap();
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.total_count, 5);
        assert_eq!(page.total_pages, 3);
    }

    #[test]
    fn audit_log_search() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        db.store_audit_entry(&make_entry("cleanup_cache", ActionResult::Success, "cleanup")).unwrap();
        db.store_audit_entry(&make_entry("rotate_logs", ActionResult::Success, "logs")).unwrap();

        let filter = AuditFilter {
            search: Some("cache".into()),
            ..Default::default()
        };
        let page = db.get_audit_log(&filter).unwrap();
        assert_eq!(page.total_count, 1);
    }

    #[test]
    fn mark_rolled_back() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        let entry = make_entry("cleanup", ActionResult::Success, "cleanup");
        let id = entry.id.to_string();
        db.store_audit_entry(&entry).unwrap();

        db.mark_rolled_back(&id).unwrap();
        let fetched = db.get_audit_entry(&id).unwrap().unwrap();
        assert!(!fetched.rollback_available);
    }

    #[test]
    fn export_json() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        db.store_audit_entry(&make_entry("test", ActionResult::Success, "cleanup")).unwrap();

        let out = dir.path().join("export.json");
        db.export_audit_log(ExportFormat::Json, &AuditFilter::default(), &out).unwrap();
        let contents = std::fs::read_to_string(&out).unwrap();
        assert!(contents.contains("\"action\": \"test\""));
    }

    #[test]
    fn export_csv() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db"), "key").unwrap();
        db.migrate().unwrap();

        db.store_audit_entry(&make_entry("test", ActionResult::Success, "cleanup")).unwrap();

        let out = dir.path().join("export.csv");
        db.export_audit_log(ExportFormat::Csv, &AuditFilter::default(), &out).unwrap();
        let contents = std::fs::read_to_string(&out).unwrap();
        assert!(contents.contains("id,timestamp,action"));
        assert!(contents.contains("test"));
    }
}
