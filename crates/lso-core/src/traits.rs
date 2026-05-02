/// Trait definitions for system probes.
use async_trait::async_trait;

use crate::error::SensorError;
use crate::types::{PrivilegeLevel, ProbeResult};

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
