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
        Platform::MacOS => collect_cpu_macos(),
        Platform::Windows => collect_cpu_windows(),
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

// ---------------------------------------------------------------------------
// macOS — host_processor_info(PROCESSOR_CPU_LOAD_INFO) + getloadavg
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn collect_cpu_macos() -> Result<CpuInfo, SensorError> {
    let mut load: [f64; 3] = [0.0; 3];
    unsafe { libc::getloadavg(load.as_mut_ptr(), 3) };

    let per_core_percent = macos_per_core_usage()?;
    let core_count = per_core_percent.len() as u32;

    Ok(CpuInfo {
        core_count,
        per_core_percent,
        load_avg_1: load[0],
        load_avg_5: load[1],
        load_avg_15: load[2],
    })
}

#[cfg(target_os = "macos")]
fn macos_per_core_usage() -> Result<Vec<f64>, SensorError> {
    extern "C" {
        fn host_processor_info(
            host: libc::mach_port_t,
            flavor: libc::c_int,
            out_count: *mut u32,
            out_info: *mut *mut i32,
            out_info_cnt: *mut u32,
        ) -> libc::c_int;
        fn vm_deallocate(
            target: libc::mach_port_t,
            address: libc::vm_address_t,
            size: libc::vm_size_t,
        ) -> libc::c_int;
    }

    const PROCESSOR_CPU_LOAD_INFO: libc::c_int = 2;
    const CPU_STATE_USER: usize = 0;
    const CPU_STATE_SYSTEM: usize = 1;
    const CPU_STATE_IDLE: usize = 2;
    const CPU_STATE_NICE: usize = 3;
    const CPU_STATE_MAX: usize = 4;

    #[allow(deprecated)]
    let host = unsafe { libc::mach_host_self() };
    let mut num_cpus: u32 = 0;
    let mut cpu_info: *mut i32 = std::ptr::null_mut();
    let mut num_info: u32 = 0;

    let ret = unsafe {
        host_processor_info(host, PROCESSOR_CPU_LOAD_INFO, &mut num_cpus, &mut cpu_info, &mut num_info)
    };
    if ret != 0 {
        return Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: format!("host_processor_info failed: {ret}"),
        });
    }

    let mut per_core = Vec::with_capacity(num_cpus as usize);
    for i in 0..num_cpus as usize {
        let base = i * CPU_STATE_MAX;
        let user = unsafe { *cpu_info.add(base + CPU_STATE_USER) } as u64;
        let system = unsafe { *cpu_info.add(base + CPU_STATE_SYSTEM) } as u64;
        let idle = unsafe { *cpu_info.add(base + CPU_STATE_IDLE) } as u64;
        let nice = unsafe { *cpu_info.add(base + CPU_STATE_NICE) } as u64;

        let total = user + system + idle + nice;
        let active = user + system + nice;

        per_core.push(if total == 0 {
            0.0
        } else {
            (active as f64 / total as f64) * 100.0
        });
    }

    if !cpu_info.is_null() {
        #[allow(deprecated)]
        let task = unsafe { libc::mach_task_self() };
        unsafe {
            vm_deallocate(
                task,
                cpu_info as libc::vm_address_t,
                num_info as libc::vm_size_t * std::mem::size_of::<i32>() as libc::vm_size_t,
            );
        }
    }

    Ok(per_core)
}

#[cfg(not(target_os = "macos"))]
fn collect_cpu_macos() -> Result<CpuInfo, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "macOS CPU collection not available on this platform".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Windows — GetSystemTimes + GetSystemInfo
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn collect_cpu_windows() -> Result<CpuInfo, SensorError> {
    use windows_sys::Win32::System::SystemInformation::*;
    use windows_sys::Win32::System::Threading::GetSystemTimes;

    let mut sys_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    unsafe { GetSystemInfo(&mut sys_info) };
    let core_count = sys_info.dwNumberOfProcessors;

    let mut idle: i64 = 0;
    let mut kernel: i64 = 0;
    let mut user: i64 = 0;
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: "GetSystemTimes failed".to_string(),
        });
    }

    let total = kernel + user;
    let active = total - idle;
    let avg_usage = if total > 0 {
        (active as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(CpuInfo {
        core_count,
        per_core_percent: vec![avg_usage; core_count as usize],
        load_avg_1: avg_usage / 100.0 * core_count as f64,
        load_avg_5: 0.0,
        load_avg_15: 0.0,
    })
}

#[cfg(not(target_os = "windows"))]
fn collect_cpu_windows() -> Result<CpuInfo, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "Windows CPU collection not available on this platform".to_string(),
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
