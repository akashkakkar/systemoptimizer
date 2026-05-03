//! CPU probe — collects per-core usage and load averages.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use lso_core::{Platform, PrivilegeLevel, ProbeResult, SensorError, SystemMetric, SystemProbe};

pub const PROBE_ID: &str = "sensor.cpu.load";

/// CPU usage snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub core_count: u32,
    pub per_core_percent: Vec<f64>,
    pub load_avg_1: f64,
    pub load_avg_5: f64,
    pub load_avg_15: f64,
}

/// Cross-platform CPU probe.
pub struct CpuProbe {
    platform: Platform,
}

impl CpuProbe {
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
impl SystemProbe for CpuProbe {
    fn probe_id(&self) -> &str {
        PROBE_ID
    }

    fn description(&self) -> &str {
        "Per-core CPU usage and system load averages"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::Unprivileged
    }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        let info = collect_cpu(self.platform)?;
        let now = Utc::now();
        let platform_label = self.platform.to_string();

        let mut metrics = Vec::new();

        let metric = |name: &str, value: f64, unit: Option<&str>| SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.to_string(),
            name: format!("system::{name}"),
            value,
            unit: unit.map(str::to_string),
            collected_at: now,
            platform: platform_label.clone(),
        };

        metrics.push(metric("core_count", info.core_count as f64, None));
        metrics.push(metric("load_avg_1", info.load_avg_1, None));
        metrics.push(metric("load_avg_5", info.load_avg_5, None));
        metrics.push(metric("load_avg_15", info.load_avg_15, None));

        for (i, pct) in info.per_core_percent.iter().enumerate() {
            metrics.push(SystemMetric {
                id: Uuid::new_v4(),
                probe_id: PROBE_ID.to_string(),
                name: format!("core_{i}::usage_percent"),
                value: *pct,
                unit: Some("%".to_string()),
                collected_at: now,
                platform: platform_label.clone(),
            });
        }

        Ok(ProbeResult {
            probe_id: PROBE_ID.to_string(),
            metrics,
            collected_at: now,
            platform: self.platform,
        })
    }
}

fn collect_cpu(platform: Platform) -> Result<CpuInfo, SensorError> {
    match platform {
        Platform::Linux => collect_cpu_linux(),
        _ => Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: format!("{platform} CPU probe not yet implemented"),
        }),
    }
}

#[cfg(target_os = "linux")]
fn collect_cpu_linux() -> Result<CpuInfo, SensorError> {
    let stat_content = std::fs::read_to_string("/proc/stat").map_err(SensorError::Io)?;

    let mut per_core_percent = Vec::new();

    for line in stat_content.lines() {
        if line.starts_with("cpu") && !line.starts_with("cpu ") {
            if let Some(pct) = parse_cpu_line(line) {
                per_core_percent.push(pct);
            }
        }
    }

    let core_count = per_core_percent.len() as u32;

    let (load_avg_1, load_avg_5, load_avg_15) = get_load_averages()?;

    Ok(CpuInfo {
        core_count,
        per_core_percent,
        load_avg_1,
        load_avg_5,
        load_avg_15,
    })
}

#[cfg(target_os = "linux")]
fn parse_cpu_line(line: &str) -> Option<f64> {
    // cpuN user nice system idle iowait irq softirq steal guest guest_nice
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 5 {
        return None;
    }

    let user: u64 = fields[1].parse().ok()?;
    let nice: u64 = fields[2].parse().ok()?;
    let system: u64 = fields[3].parse().ok()?;
    let idle: u64 = fields[4].parse().ok()?;
    let iowait: u64 = fields.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let irq: u64 = fields.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
    let softirq: u64 = fields.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
    let steal: u64 = fields.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);

    let total = user + nice + system + idle + iowait + irq + softirq + steal;
    let active = total - idle - iowait;

    if total == 0 {
        return Some(0.0);
    }

    Some((active as f64 / total as f64) * 100.0)
}

#[cfg(target_os = "linux")]
fn get_load_averages() -> Result<(f64, f64, f64), SensorError> {
    let content = std::fs::read_to_string("/proc/loadavg").map_err(SensorError::Io)?;
    let fields: Vec<&str> = content.split_whitespace().collect();
    if fields.len() < 3 {
        return Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: "unexpected /proc/loadavg format".to_string(),
        });
    }

    let parse = |s: &str| -> Result<f64, SensorError> {
        s.parse().map_err(|_| SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: format!("failed to parse load average: {s}"),
        })
    };

    Ok((parse(fields[0])?, parse(fields[1])?, parse(fields[2])?))
}

#[cfg(not(target_os = "linux"))]
fn collect_cpu_linux() -> Result<CpuInfo, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "Linux CPU collection not available on this platform".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_has_correct_id() {
        let probe = CpuProbe::new(Platform::Linux);
        assert_eq!(probe.probe_id(), PROBE_ID);
        assert_eq!(probe.required_privilege(), PrivilegeLevel::Unprivileged);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn linux_probe_returns_cpu_data() {
        let probe = CpuProbe::for_host().unwrap();
        let result = probe.collect().await.unwrap();
        assert_eq!(result.probe_id, PROBE_ID);
        assert!(!result.metrics.is_empty());

        let core_count = result
            .metrics
            .iter()
            .find(|m| m.name == "system::core_count")
            .unwrap();
        assert!(core_count.value >= 1.0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parse_cpu_line_valid() {
        let line = "cpu0 100 20 30 800 10 5 3 2 0 0";
        let pct = parse_cpu_line(line).unwrap();
        // active = 100+20+30+5+3+2 = 160, idle+iowait = 800+10 = 810, total = 970
        let expected = (160.0 / 970.0) * 100.0;
        assert!((pct - expected).abs() < 0.01);
    }
}
