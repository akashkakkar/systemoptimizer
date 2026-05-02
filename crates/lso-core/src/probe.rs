//! System probe trait and result types.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::SensorError;

/// Privilege level required to run a probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeLevel {
    None,
    Elevated,
}

/// A single metric value collected by a probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Uint(u64),
    Float(f64),
    Text(String),
}

/// Result of a probe execution containing collected metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub probe_id: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: Vec<HashMap<String, MetricValue>>,
}

/// Trait that all system probes must implement.
pub trait SystemProbe: Send + Sync {
    /// Unique identifier for this probe.
    fn probe_id(&self) -> &str;

    /// Privilege level required to execute this probe.
    fn required_privilege(&self) -> PrivilegeLevel;

    /// Collect system metrics. Returns structured results or an error.
    fn collect(&self) -> Pin<Box<dyn Future<Output = Result<ProbeResult, SensorError>> + Send + '_>>;
}
