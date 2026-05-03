//! LSO Core — shared types, traits, and error definitions.

pub mod error;
pub mod mock;
pub mod platform;
pub mod traits;
pub mod types;

pub use error::*;
pub use mock::MockPlatform;
pub use platform::*;
pub use traits::*;
pub use types::*;

/// Initialize the tracing subscriber with structured logging.
///
/// Uses `RUST_LOG` env var if set, otherwise defaults to `info`.
/// Safe to call once at application startup.
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexports_accessible() {
        let _level = RiskLevel::Low;
        let _status = ApprovalStatus::Pending;
        let _priv = PrivilegeLevel::Unprivileged;
        let _platform = Platform::detect();
    }

    #[test]
    fn tracing_subscriber_initializes() {
        use tracing_subscriber::EnvFilter;

        let result = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new("debug"))
            .with_test_writer()
            .try_init();

        // Either Ok (first init) or Err (already initialized by another test) is fine.
        let _ = result;
    }

    #[test]
    fn error_converts_to_lso_error() {
        let sensor_err = SensorError::PermissionRequired("root needed".into());
        let lso_err: LsoError = sensor_err.into();
        assert!(matches!(lso_err, LsoError::Sensor(_)));
    }

    #[tokio::test]
    async fn system_probe_trait_is_object_safe() {
        fn _accepts_probe(_probe: &dyn SystemProbe) {}
    }
}
