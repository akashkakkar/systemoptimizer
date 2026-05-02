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
            "INSERT OR REPLACE INTO recommendations (id, title, description, risk_level, category, status, created_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                rec.id.to_string(),
                rec.title,
                rec.description,
                rec.risk_level.as_str(),
                rec.category,
                rec.status.as_str(),
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

    /// Get all recommendations with status 'pending'.
    pub fn get_pending_recommendations(&self) -> DbResult<Vec<Recommendation>> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, title, description, risk_level, category, status, created_at, resolved_at
             FROM recommendations
             WHERE status = 'pending'
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let resolved_str: Option<String> = row.get(7)?;
            Ok(Recommendation {
                id: row
                    .get::<_, String>(0)?
                    .parse::<Uuid>()
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?,
                title: row.get(1)?,
                description: row.get(2)?,
                risk_level: parse_enum::<RiskLevel>(&row.get::<_, String>(3)?, 3)?,
                category: row.get(4)?,
                target: String::new(),
                rollback_plan: None,
                status: parse_enum::<RecommendationStatus>(&row.get::<_, String>(5)?, 5)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e)))?
                    .with_timezone(&Utc),
                resolved_at: resolved_str
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
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
            title: "Clean temp files".into(),
            description: "Remove stale temp files to reclaim 2 GB".into(),
            risk_level: RiskLevel::Low,
            category: "disk".into(),
            target: "/tmp".into(),
            rollback_plan: None,
            status: RecommendationStatus::Pending,
            created_at: Utc::now(),
            resolved_at: None,
        };
        db.store_recommendation(&rec).unwrap();

        let pending = db.get_pending_recommendations().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Clean temp files");
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
