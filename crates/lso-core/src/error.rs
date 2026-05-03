//! Error types for the actuator subsystem.

use crate::types::PrivilegeLevel;

/// Errors that can occur during actuator operations.
#[derive(Debug, thiserror::Error)]
pub enum ActuatorError {
    #[error("action not approved: {0}")]
    NotApproved(String),

    #[error("preflight failed: {0}")]
    PreflightFailed(String),

    #[error("snapshot failed: {0}")]
    SnapshotFailed(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error("verification failed: {0}")]
    VerificationFailed(String),

    #[error("rollback failed: {0}")]
    RollbackFailed(String),

    #[error("action not found: {action_id}")]
    ActionNotFound { action_id: String },

    #[error("insufficient privilege: requires {required:?}")]
    InsufficientPrivilege { required: PrivilegeLevel },

    #[error("platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
