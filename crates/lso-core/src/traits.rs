//! Core traits for the snapshot and actuator subsystems.

use async_trait::async_trait;

use crate::error::ActuatorError;
use crate::types::{
    ActionResult, PreflightReport, PrivilegeLevel, RiskLevel, SnapshotId, SnapshotInfo,
    VerifyResult,
};

/// Provides snapshot creation and restoration for rollback safety.
#[async_trait]
pub trait SnapshotProvider: Send + Sync {
    /// Create a snapshot with the given label.
    async fn create(&self, label: &str) -> Result<SnapshotId, ActuatorError>;

    /// List all available snapshots.
    async fn list(&self) -> Result<Vec<SnapshotInfo>, ActuatorError>;

    /// Restore the system state from a snapshot.
    async fn restore(&self, id: &SnapshotId) -> Result<(), ActuatorError>;

    /// Delete a snapshot.
    async fn delete(&self, id: &SnapshotId) -> Result<(), ActuatorError>;

    /// Whether this provider is supported on the current platform.
    fn supports_platform(&self) -> bool;
}

/// Executes a specific type of system action.
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Unique identifier for this action type.
    fn action_id(&self) -> &str;

    /// Human-readable description of what this action does.
    fn description(&self) -> &str;

    /// Risk level of this action.
    fn risk_level(&self) -> RiskLevel;

    /// Required privilege level.
    fn required_privilege(&self) -> PrivilegeLevel;

    /// Validate preconditions before execution.
    async fn preflight(&self, target: &str) -> Result<PreflightReport, ActuatorError>;

    /// Execute the action (only called after approval + snapshot).
    async fn execute(&self, target: &str) -> Result<ActionResult, ActuatorError>;

    /// Verify the action succeeded.
    async fn verify(&self, target: &str) -> Result<VerifyResult, ActuatorError>;
}
