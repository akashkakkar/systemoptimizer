/// Error types for all LSO subsystems.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LsoError {
    #[error("Sensor error: {0}")]
    Sensor(#[from] SensorError),
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("Engine error: {0}")]
    Engine(#[from] EngineError),
    #[error("Actuator error: {0}")]
    Actuator(#[from] ActuatorError),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}

#[derive(Error, Debug)]
pub enum SensorError {
    #[error("Probe failed: {probe} — {reason}")]
    ProbeFailed { probe: String, reason: String },
    #[error("Permission required: {0}")]
    PermissionRequired(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Query failed: {0}")]
    QueryFailed(String),
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Rule parse error: {0}")]
    RuleParseFailed(String),
    #[error("Evaluation failed: {rule} — {reason}")]
    EvaluationFailed { rule: String, reason: String },
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

#[derive(Error, Debug)]
pub enum ActuatorError {
    #[error("Execution failed: {action} — {reason}")]
    ExecutionFailed { action: String, reason: String },
    #[error("Snapshot failed: {0}")]
    SnapshotFailed(String),
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
    #[error("Approval required for: {0}")]
    ApprovalRequired(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor_io_fails() -> Result<(), SensorError> {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        Err(io_err)?
    }

    fn calls_sensor() -> Result<(), LsoError> {
        sensor_io_fails()?;
        Ok(())
    }

    #[test]
    fn error_chaining_sensor_io_to_lso() {
        let err = calls_sensor().unwrap_err();
        assert!(matches!(err, LsoError::Sensor(SensorError::Io(_))));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn sensor_probe_failed_display() {
        let err = SensorError::ProbeFailed {
            probe: "disk_usage".into(),
            reason: "timeout".into(),
        };
        assert_eq!(err.to_string(), "Probe failed: disk_usage — timeout");
    }

    #[test]
    fn actuator_error_chains_to_lso() {
        let err: LsoError = ActuatorError::SnapshotFailed("disk full".into()).into();
        assert!(matches!(err, LsoError::Actuator(ActuatorError::SnapshotFailed(_))));
    }

    #[test]
    fn engine_error_chains_to_lso() {
        let err: LsoError = EngineError::InvalidConfig("bad toml".into()).into();
        assert!(matches!(err, LsoError::Engine(EngineError::InvalidConfig(_))));
    }

    #[test]
    fn database_error_chains_to_lso() {
        let err: LsoError = DatabaseError::QueryFailed("syntax".into()).into();
        assert!(matches!(err, LsoError::Database(DatabaseError::QueryFailed(_))));
    }
}
