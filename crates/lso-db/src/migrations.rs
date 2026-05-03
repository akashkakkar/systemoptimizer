//! Embedded migration runner.

use chrono::Utc;
use rusqlite::Connection;

use crate::error::{DbError, DbResult};

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../migrations/001_initial.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../migrations/002_recommendation_lifecycle.sql"),
    },
];

/// Ensure the schema_version table exists, then apply any pending migrations.
pub fn run(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    let current_version: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .map_err(DbError::from)?;

    for migration in MIGRATIONS {
        if migration.version <= current_version {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![migration.version, Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
        tracing::info!(version = migration.version, "applied migration");
    }

    Ok(())
}
