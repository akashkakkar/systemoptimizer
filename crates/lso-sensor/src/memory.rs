//! Memory probe — collects total, used, available, and swap usage.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use lso_core::{Platform, PrivilegeLevel, ProbeResult, SensorError, SystemMetric, SystemProbe};

pub const PROBE_ID: &str = "sensor.memory.usage";

/// Memory usage snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub usage_percent: f64,
    pub swap_percent: f64,
}

/// Cross-platform memory probe.
pub struct MemoryProbe {
    platform: Platform,
}

impl MemoryProbe {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn for_host() -> Result<Self, SensorError> {
        let platform = Platform::detect().ok_or_else(|| SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: format!("unknown host platform: {}", std::env::consts::OS),
        })?;
        Ok(Self::new(platform))
    }
}

#[async_trait]
impl SystemProbe for MemoryProbe {
    fn probe_id(&self) -> &str {
        PROBE_ID
    }

    fn description(&self) -> &str {
        "System memory and swap usage"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::Unprivileged
    }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        let info = collect_memory(self.platform)?;
        let now = Utc::now();
        let platform_label = self.platform.to_string();

        let metric = |name: &str, value: f64, unit: &str| SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.to_string(),
            name: format!("system::{name}"),
            value,
            unit: Some(unit.to_string()),
            collected_at: now,
            platform: platform_label.clone(),
        };

        let metrics = vec![
            metric("total_bytes", info.total_bytes as f64, "B"),
            metric("used_bytes", info.used_bytes as f64, "B"),
            metric("available_bytes", info.available_bytes as f64, "B"),
            metric("swap_total_bytes", info.swap_total_bytes as f64, "B"),
            metric("swap_used_bytes", info.swap_used_bytes as f64, "B"),
            metric("usage_percent", info.usage_percent, "%"),
            metric("swap_percent", info.swap_percent, "%"),
        ];

        Ok(ProbeResult {
            probe_id: PROBE_ID.to_string(),
            metrics,
            collected_at: now,
            platform: self.platform,
        })
    }
}

fn collect_memory(platform: Platform) -> Result<MemoryInfo, SensorError> {
    match platform {
        Platform::Linux => collect_memory_linux(),
        _ => Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: format!("{platform} memory probe not yet implemented"),
        }),
    }
}

#[cfg(target_os = "linux")]
fn collect_memory_linux() -> Result<MemoryInfo, SensorError> {
    let content = std::fs::read_to_string("/proc/meminfo").map_err(SensorError::Io)?;

    let mut mem_total: u64 = 0;
    let mut mem_available: u64 = 0;
    let mut swap_total: u64 = 0;
    let mut swap_free: u64 = 0;

    for line in content.lines() {
        if let Some((key, value_kb)) = parse_meminfo_line(line) {
            match key {
                "MemTotal" => mem_total = value_kb * 1024,
                "MemAvailable" => mem_available = value_kb * 1024,
                "SwapTotal" => swap_total = value_kb * 1024,
                "SwapFree" => swap_free = value_kb * 1024,
                _ => {}
            }
        }
    }

    let used = mem_total.saturating_sub(mem_available);
    let swap_used = swap_total.saturating_sub(swap_free);

    let usage_percent = if mem_total > 0 {
        (used as f64 / mem_total as f64) * 100.0
    } else {
        0.0
    };

    let swap_percent = if swap_total > 0 {
        (swap_used as f64 / swap_total as f64) * 100.0
    } else {
        0.0
    };

    Ok(MemoryInfo {
        total_bytes: mem_total,
        used_bytes: used,
        available_bytes: mem_available,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
        usage_percent,
        swap_percent,
    })
}

#[cfg(target_os = "linux")]
fn parse_meminfo_line(line: &str) -> Option<(&str, u64)> {
    let (key, rest) = line.split_once(':')?;
    let value_str = rest.trim().trim_end_matches(" kB").trim();
    let value: u64 = value_str.parse().ok()?;
    Some((key, value))
}

#[cfg(not(target_os = "linux"))]
fn collect_memory_linux() -> Result<MemoryInfo, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "Linux memory collection not available on this platform".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_has_correct_id() {
        let probe = MemoryProbe::new(Platform::Linux);
        assert_eq!(probe.probe_id(), PROBE_ID);
        assert_eq!(probe.required_privilege(), PrivilegeLevel::Unprivileged);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn linux_probe_returns_memory_data() {
        let probe = MemoryProbe::for_host().unwrap();
        let result = probe.collect().await.unwrap();
        assert_eq!(result.probe_id, PROBE_ID);
        assert_eq!(result.metrics.len(), 7);

        let usage = result
            .metrics
            .iter()
            .find(|m| m.name == "system::usage_percent")
            .unwrap();
        assert!(usage.value >= 0.0 && usage.value <= 100.0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parse_meminfo_line_valid() {
        let (key, val) = parse_meminfo_line("MemTotal:       16384000 kB").unwrap();
        assert_eq!(key, "MemTotal");
        assert_eq!(val, 16384000);
    }
}
