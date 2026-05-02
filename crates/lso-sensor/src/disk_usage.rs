//! Disk usage probe — collects per-mount statistics across platforms.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use lso_core::{
    DiskUsageReport, Platform, PrivilegeLevel, ProbeResult, SensorError, SystemMetric, SystemProbe,
};

use crate::platform::{collect_mount_stats, MountStats};

pub const PROBE_ID: &str = "disk.usage";

/// Cross-platform disk usage probe.
pub struct DiskUsageProbe {
    platform: Platform,
}

impl DiskUsageProbe {
    /// Create a probe for the given platform.
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    /// Create a probe for the host platform, returning an error if unsupported.
    pub fn for_host() -> Result<Self, SensorError> {
        let platform = Platform::detect().ok_or_else(|| SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: format!("unknown host platform: {}", std::env::consts::OS),
        })?;
        Ok(Self::new(platform))
    }
}

#[async_trait]
impl SystemProbe for DiskUsageProbe {
    fn probe_id(&self) -> &str {
        PROBE_ID
    }

    fn description(&self) -> &str {
        "Per-mount disk usage statistics (total, used, available, percent)"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::Unprivileged
    }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        let mounts = collect_mount_stats(self.platform)?;
        let now = Utc::now();
        let platform_label = self.platform.to_string();

        let metrics = mounts
            .iter()
            .flat_map(|m| metric_rows_for(m, &platform_label, now))
            .collect();

        Ok(ProbeResult {
            probe_id: PROBE_ID.to_string(),
            metrics,
            collected_at: now,
            platform: self.platform,
        })
    }
}

fn metric_rows_for(
    m: &MountStats,
    platform_label: &str,
    collected_at: chrono::DateTime<Utc>,
) -> Vec<SystemMetric> {
    let row = |name: &str, value: f64, unit: Option<&str>| SystemMetric {
        id: Uuid::new_v4(),
        probe_id: PROBE_ID.to_string(),
        name: format!("{}::{}", m.mount_point, name),
        value,
        unit: unit.map(str::to_string),
        collected_at,
        platform: platform_label.to_string(),
    };

    vec![
        row("total_bytes", m.total_bytes as f64, Some("B")),
        row("used_bytes", m.used_bytes as f64, Some("B")),
        row("available_bytes", m.available_bytes as f64, Some("B")),
        row("usage_percent", m.usage_percent, Some("%")),
    ]
}

/// IPC-friendly: collect per-mount disk usage as `DiskUsageReport` records.
pub fn get_disk_reports() -> Result<Vec<DiskUsageReport>, SensorError> {
    let platform = Platform::detect().ok_or_else(|| SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: format!("unknown host platform: {}", std::env::consts::OS),
    })?;
    let mounts = collect_mount_stats(platform)?;
    Ok(mounts.into_iter().map(into_report).collect())
}

fn into_report(m: MountStats) -> DiskUsageReport {
    DiskUsageReport {
        mount_point: m.mount_point,
        fs_type: m.fs_type,
        total_bytes: m.total_bytes,
        used_bytes: m.used_bytes,
        available_bytes: m.available_bytes,
        usage_percent: m.usage_percent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn host_probe_returns_at_least_one_mount() {
        let probe = DiskUsageProbe::for_host().expect("supported host platform");
        let result = probe.collect().await.expect("collect");
        assert_eq!(result.probe_id, PROBE_ID);
        assert!(!result.metrics.is_empty(), "expected at least one mount");
    }

    #[test]
    fn get_disk_reports_returns_data_or_empty() {
        // On the host platform this should succeed; we just assert it doesn't panic and
        // every report is internally consistent.
        let reports = get_disk_reports().expect("collect reports");
        for r in &reports {
            assert!(r.total_bytes > 0);
            assert!(r.usage_percent >= 0.0 && r.usage_percent <= 100.0);
        }
    }
}
