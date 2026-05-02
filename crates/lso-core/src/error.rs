//! Sensor error types.

use thiserror::Error;

/// Errors that can occur during sensor operations.
#[derive(Debug, Error)]
pub enum SensorError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("storage error: {0}")]
    Storage(String),
}
