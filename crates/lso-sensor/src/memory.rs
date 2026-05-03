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
        Platform::MacOS => collect_memory_macos(),
        Platform::Windows => collect_memory_windows(),
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

// ---------------------------------------------------------------------------
// macOS — host_statistics64(HOST_VM_INFO64) + sysctl hw.memsize / vm.swapusage
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn collect_memory_macos() -> Result<MemoryInfo, SensorError> {
    let total_bytes = macos_sysctl_u64(c"hw.memsize")?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;

    let vm = macos_vm_stats()?;
    let used_pages = vm.active_count as u64 + vm.wire_count as u64 + vm.compressor_page_count as u64;
    let used_bytes = used_pages * page_size;
    let available_bytes = total_bytes.saturating_sub(used_bytes);

    let (swap_total, swap_used) = macos_swap_usage();

    let usage_percent = if total_bytes > 0 {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    } else {
        0.0
    };
    let swap_percent = if swap_total > 0 {
        (swap_used as f64 / swap_total as f64) * 100.0
    } else {
        0.0
    };

    Ok(MemoryInfo {
        total_bytes,
        used_bytes,
        available_bytes,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
        usage_percent,
        swap_percent,
    })
}

#[cfg(target_os = "macos")]
fn macos_sysctl_u64(name: &std::ffi::CStr) -> Result<u64, SensorError> {
    let mut val: u64 = 0;
    let mut size = std::mem::size_of::<u64>();
    if unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            &mut val as *mut _ as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(SensorError::Io(std::io::Error::last_os_error()));
    }
    Ok(val)
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct VmStatistics64 {
    free_count: u32,
    active_count: u32,
    inactive_count: u32,
    wire_count: u32,
    zero_fill_count: u64,
    reactivations: u64,
    pageins: u64,
    pageouts: u64,
    faults: u64,
    cow_faults: u64,
    lookups: u64,
    hits: u64,
    purges: u64,
    purgeable_count: u32,
    speculative_count: u32,
    decompressions: u64,
    compressions: u64,
    swapins: u64,
    swapouts: u64,
    compressor_page_count: u32,
    throttled_count: u32,
    external_page_count: u32,
    internal_page_count: u32,
    total_uncompressed_pages_in_compressor: u64,
}

#[cfg(target_os = "macos")]
fn macos_vm_stats() -> Result<VmStatistics64, SensorError> {
    const HOST_VM_INFO64: libc::c_int = 4;
    #[allow(deprecated)]
    let host = unsafe { libc::mach_host_self() };
    let mut stats: VmStatistics64 = unsafe { std::mem::zeroed() };
    let mut count = (std::mem::size_of::<VmStatistics64>() / std::mem::size_of::<i32>()) as u32;

    let ret = unsafe {
        libc::host_statistics64(
            host,
            HOST_VM_INFO64,
            &mut stats as *mut _ as *mut i32,
            &mut count,
        )
    };

    if ret != 0 {
        return Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: format!("host_statistics64 failed: {ret}"),
        });
    }

    Ok(stats)
}

#[cfg(target_os = "macos")]
fn macos_swap_usage() -> (u64, u64) {
    #[repr(C)]
    struct XswUsage {
        xsu_total: u64,
        xsu_avail: u64,
        xsu_used: u64,
        xsu_pagesize: u32,
        xsu_encrypted: i32,
    }

    let mut usage: XswUsage = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of::<XswUsage>();

    let ret = unsafe {
        libc::sysctlbyname(
            c"vm.swapusage".as_ptr(),
            &mut usage as *mut _ as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };

    if ret != 0 {
        return (0, 0);
    }

    (usage.xsu_total, usage.xsu_used)
}

#[cfg(not(target_os = "macos"))]
fn collect_memory_macos() -> Result<MemoryInfo, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "macOS memory collection not available on this platform".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Windows — GlobalMemoryStatusEx
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn collect_memory_windows() -> Result<MemoryInfo, SensorError> {
    use windows_sys::Win32::System::SystemInformation::*;

    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: "GlobalMemoryStatusEx failed".to_string(),
        });
    }

    let total = status.ullTotalPhys;
    let available = status.ullAvailPhys;
    let used = total.saturating_sub(available);
    let swap_total = status.ullTotalPageFile;
    let swap_used = swap_total.saturating_sub(status.ullAvailPageFile);

    Ok(MemoryInfo {
        total_bytes: total,
        used_bytes: used,
        available_bytes: available,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
        usage_percent: if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        },
        swap_percent: if swap_total > 0 {
            (swap_used as f64 / swap_total as f64) * 100.0
        } else {
            0.0
        },
    })
}

#[cfg(not(target_os = "windows"))]
fn collect_memory_windows() -> Result<MemoryInfo, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "Windows memory collection not available on this platform".to_string(),
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
