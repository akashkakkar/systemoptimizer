//! Error types for the database layer.

/// Database-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database connection mutex poisoned")]
    LockPoisoned,
}

pub type DbResult<T> = Result<T, DbError>;
