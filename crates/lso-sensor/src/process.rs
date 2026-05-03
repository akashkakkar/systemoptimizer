//! Process list probe — collects per-process CPU%, memory%, status.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use lso_core::{Platform, PrivilegeLevel, ProbeResult, SensorError, SystemMetric, SystemProbe};

pub const PROBE_ID: &str = "sensor.process.list";

/// A single process entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub status: String,
}

/// Cross-platform process list probe.
pub struct ProcessListProbe {
    platform: Platform,
}

impl ProcessListProbe {
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
impl SystemProbe for ProcessListProbe {
    fn probe_id(&self) -> &str {
        PROBE_ID
    }

    fn description(&self) -> &str {
        "Per-process CPU and memory usage with status"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::Unprivileged
    }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        let processes = collect_processes(self.platform)?;
        let now = Utc::now();
        let platform_label = self.platform.to_string();

        let metrics = processes
            .iter()
            .flat_map(|p| {
                let prefix = format!("{}:{}", p.pid, p.name);
                vec![
                    SystemMetric {
                        id: Uuid::new_v4(),
                        probe_id: PROBE_ID.to_string(),
                        name: format!("{prefix}::cpu_percent"),
                        value: p.cpu_percent,
                        unit: Some("%".to_string()),
                        collected_at: now,
                        platform: platform_label.clone(),
                    },
                    SystemMetric {
                        id: Uuid::new_v4(),
                        probe_id: PROBE_ID.to_string(),
                        name: format!("{prefix}::memory_percent"),
                        value: p.memory_percent,
                        unit: Some("%".to_string()),
                        collected_at: now,
                        platform: platform_label.clone(),
                    },
                ]
            })
            .collect();

        Ok(ProbeResult {
            probe_id: PROBE_ID.to_string(),
            metrics,
            collected_at: now,
            platform: self.platform,
        })
    }
}

fn collect_processes(platform: Platform) -> Result<Vec<ProcessInfo>, SensorError> {
    match platform {
        Platform::Linux => collect_processes_linux(),
        _ => Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: format!("{platform} process probe not yet implemented"),
        }),
    }
}

#[cfg(target_os = "linux")]
fn collect_processes_linux() -> Result<Vec<ProcessInfo>, SensorError> {
    use std::fs;

    let total_memory_kb = get_total_memory_kb()?;
    let clock_ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    let uptime = get_system_uptime()?;

    let mut processes = Vec::new();

    let proc_dir = fs::read_dir("/proc").map_err(SensorError::Io)?;
    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let stat_path = entry.path().join("stat");
        let stat_content = match fs::read_to_string(&stat_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Some(info) = parse_proc_stat(&stat_content, pid, total_memory_kb, clock_ticks, uptime) {
            processes.push(info);
        }
    }

    Ok(processes)
}

#[cfg(target_os = "linux")]
fn parse_proc_stat(
    content: &str,
    pid: u32,
    total_memory_kb: u64,
    clock_ticks: f64,
    uptime: f64,
) -> Option<ProcessInfo> {
    // Format: pid (comm) state ppid ... utime stime ... starttime ... rss ...
    // Fields after (comm) are space-separated. comm can contain spaces/parens.
    let comm_start = content.find('(')?;
    let comm_end = content.rfind(')')?;
    let name = content[comm_start + 1..comm_end].to_string();
    let rest = &content[comm_end + 2..];
    let fields: Vec<&str> = rest.split_whitespace().collect();

    if fields.len() < 22 {
        return None;
    }

    let state_char = fields[0];
    let utime: u64 = fields[11].parse().ok()?;
    let stime: u64 = fields[12].parse().ok()?;
    let starttime: u64 = fields[19].parse().ok()?;
    let rss_pages: u64 = fields[21].parse().ok()?;

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let rss_kb = rss_pages * page_size / 1024;

    let memory_percent = if total_memory_kb > 0 {
        (rss_kb as f64 / total_memory_kb as f64) * 100.0
    } else {
        0.0
    };

    let total_time = utime + stime;
    let seconds_running = uptime - (starttime as f64 / clock_ticks);
    let cpu_percent = if seconds_running > 0.0 {
        ((total_time as f64 / clock_ticks) / seconds_running) * 100.0
    } else {
        0.0
    };

    let status = match state_char {
        "R" => "running",
        "S" => "sleeping",
        "D" => "disk_sleep",
        "Z" => "zombie",
        "T" => "stopped",
        "t" => "tracing_stop",
        "X" | "x" => "dead",
        _ => "unknown",
    }
    .to_string();

    Some(ProcessInfo {
        pid,
        name,
        cpu_percent,
        memory_percent,
        status,
    })
}

#[cfg(target_os = "linux")]
fn get_total_memory_kb() -> Result<u64, SensorError> {
    let content = std::fs::read_to_string("/proc/meminfo").map_err(SensorError::Io)?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb_str = rest.trim().trim_end_matches(" kB").trim();
            return kb_str.parse().map_err(|_| SensorError::ProbeFailed {
                probe: PROBE_ID.to_string(),
                reason: "failed to parse MemTotal".to_string(),
            });
        }
    }
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "MemTotal not found in /proc/meminfo".to_string(),
    })
}

#[cfg(target_os = "linux")]
fn get_system_uptime() -> Result<f64, SensorError> {
    let content = std::fs::read_to_string("/proc/uptime").map_err(SensorError::Io)?;
    let uptime_str = content.split_whitespace().next().ok_or_else(|| {
        SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: "empty /proc/uptime".to_string(),
        }
    })?;
    uptime_str.parse().map_err(|_| SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "failed to parse uptime".to_string(),
    })
}

#[cfg(not(target_os = "linux"))]
fn collect_processes_linux() -> Result<Vec<ProcessInfo>, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "Linux process collection not available on this platform".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_has_correct_id() {
        let probe = ProcessListProbe::new(Platform::Linux);
        assert_eq!(probe.probe_id(), PROBE_ID);
        assert_eq!(probe.required_privilege(), PrivilegeLevel::Unprivileged);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn linux_probe_returns_processes() {
        let probe = ProcessListProbe::for_host().unwrap();
        let result = probe.collect().await.unwrap();
        assert_eq!(result.probe_id, PROBE_ID);
        assert!(!result.metrics.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parse_proc_stat_valid() {
        let content = "1 (systemd) S 0 1 1 0 -1 4194560 100 200 0 0 50 30 0 0 20 0 1 0 1 100000 500 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0 0 0 0 0 0 0 0 0";
        let info = parse_proc_stat(content, 1, 16_000_000, 100.0, 10000.0).unwrap();
        assert_eq!(info.pid, 1);
        assert_eq!(info.name, "systemd");
        assert_eq!(info.status, "sleeping");
        assert!(info.cpu_percent >= 0.0);
        assert!(info.memory_percent >= 0.0);
    }
}
