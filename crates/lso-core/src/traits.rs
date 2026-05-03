/// Trait definitions for system probes and actuators.
use async_trait::async_trait;

use crate::error::{ActuatorError, SensorError};
use crate::types::{CleanupProgress, PreflightReport, PrivilegeLevel, ProbeResult, RiskLevel};

/// Persistent cache for LLM-generated explanations. Keyed by the
/// recommendation id (stringified UUID) and a hash of the underlying
/// probe data, so an explanation is reused only while its inputs are
/// unchanged. Backed by SQLCipher in production; in-memory and JSON
/// implementations exist for tests.
#[async_trait]
pub trait ExplanationCache: Send + Sync {
    async fn get(&self, rec_id: &str, data_hash: &str) -> Option<String>;
    async fn put(&self, rec_id: &str, data_hash: &str, explanation: &str);
    async fn invalidate(&self, rec_id: &str);
}

/// A system probe that collects metrics from the local machine.
#[async_trait]
pub trait SystemProbe: Send + Sync {
    /// Unique identifier for this probe (e.g. "disk_usage").
    fn probe_id(&self) -> &str;

    /// Human-readable description of what this probe measures.
    fn description(&self) -> &str;

    /// Minimum privilege level needed to run this probe.
    fn required_privilege(&self) -> PrivilegeLevel;

    /// Collect metrics from the system.
    async fn collect(&self) -> Result<ProbeResult, SensorError>;
}

/// An executor that can perform a specific system action.
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Unique identifier for this action (e.g. "cleanup.temp_files").
    fn action_id(&self) -> &str;

    /// Risk level of this action.
    fn risk_level(&self) -> RiskLevel;

    /// Generate a preflight report without making changes.
    async fn preflight(&self) -> Result<PreflightReport, ActuatorError>;

    /// Execute the action, reporting progress via the callback.
    async fn execute(
        &self,
        on_progress: Box<dyn Fn(CleanupProgress) + Send>,
    ) -> Result<crate::types::CleanupResult, ActuatorError>;
}
