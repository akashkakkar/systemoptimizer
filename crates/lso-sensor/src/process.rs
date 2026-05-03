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
        Platform::MacOS => collect_processes_macos(),
        Platform::Windows => collect_processes_windows(),
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

// ---------------------------------------------------------------------------
// macOS — sysctl KERN_PROC_ALL + proc_pidinfo for per-process stats
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn collect_processes_macos() -> Result<Vec<ProcessInfo>, SensorError> {
    extern "C" {
        fn proc_listallpids(
            buffer: *mut libc::c_void,
            buffersize: libc::c_int,
        ) -> libc::c_int;
        fn proc_name(
            pid: libc::c_int,
            buffer: *mut libc::c_void,
            buffersize: u32,
        ) -> libc::c_int;
    }

    let total_memory = macos_sysctl_u64(c"hw.memsize")?;

    let count = unsafe { proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: "proc_listallpids returned 0".to_string(),
        });
    }

    let alloc = (count as usize) + 64;
    let mut pids: Vec<i32> = vec![0; alloc];
    let buf_bytes = (alloc * std::mem::size_of::<i32>()) as libc::c_int;
    let actual = unsafe { proc_listallpids(pids.as_mut_ptr() as *mut libc::c_void, buf_bytes) };
    if actual <= 0 {
        return Err(SensorError::Io(std::io::Error::last_os_error()));
    }
    pids.truncate(actual as usize);

    let boot_time = macos_boot_time().unwrap_or(0.0);
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let system_uptime = (now_secs - boot_time).max(1.0);

    let mut processes = Vec::with_capacity(pids.len());
    for &pid in &pids {
        if pid <= 0 {
            continue;
        }

        let mut name_buf = [0u8; 256];
        let name_len =
            unsafe { proc_name(pid, name_buf.as_mut_ptr() as *mut libc::c_void, 256) };
        let name = if name_len > 0 {
            String::from_utf8_lossy(&name_buf[..name_len as usize]).to_string()
        } else {
            continue;
        };

        let (cpu_percent, memory_percent) = macos_task_info(pid, total_memory, system_uptime);

        processes.push(ProcessInfo {
            pid: pid as u32,
            name,
            cpu_percent,
            memory_percent,
            status: "running".to_string(),
        });
    }

    Ok(processes)
}

#[cfg(target_os = "macos")]
fn macos_boot_time() -> Option<f64> {
    let mut tv: libc::timeval = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of::<libc::timeval>();
    let mut mib = [libc::CTL_KERN, libc::KERN_BOOTTIME];
    let ret = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            2,
            &mut tv as *mut _ as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if ret == 0 {
        Some(tv.tv_sec as f64 + tv.tv_usec as f64 / 1_000_000.0)
    } else {
        None
    }
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
fn macos_task_info(pid: libc::c_int, total_memory: u64, uptime_secs: f64) -> (f64, f64) {
    extern "C" {
        fn proc_pidinfo(
            pid: libc::c_int,
            flavor: libc::c_int,
            arg: u64,
            buffer: *mut libc::c_void,
            buffersize: libc::c_int,
        ) -> libc::c_int;
    }

    const PROC_PIDTASKINFO: libc::c_int = 4;

    #[repr(C)]
    struct ProcTaskInfo {
        pti_virtual_size: u64,
        pti_resident_size: u64,
        pti_total_user: u64,
        pti_total_system: u64,
        pti_threads_user: u64,
        pti_threads_system: u64,
        pti_policy: i32,
        pti_faults: i32,
        pti_pageins: i32,
        pti_cow_faults: i32,
        pti_messages_sent: i32,
        pti_messages_received: i32,
        pti_syscalls_mach: i32,
        pti_syscalls_unix: i32,
        pti_csw: i32,
        pti_threadnum: i32,
        pti_numrunning: i32,
        pti_priority: i32,
    }

    let mut ti: ProcTaskInfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<ProcTaskInfo>() as libc::c_int;

    let ret = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDTASKINFO,
            0,
            &mut ti as *mut _ as *mut libc::c_void,
            size,
        )
    };
    if ret <= 0 {
        return (0.0, 0.0);
    }

    let memory_percent = if total_memory > 0 {
        (ti.pti_resident_size as f64 / total_memory as f64) * 100.0
    } else {
        0.0
    };

    let total_cpu_secs = (ti.pti_total_user + ti.pti_total_system) as f64 / 1_000_000_000.0;
    let cpu_percent = if uptime_secs > 0.0 {
        (total_cpu_secs / uptime_secs) * 100.0
    } else {
        0.0
    };

    (cpu_percent, memory_percent)
}

#[cfg(not(target_os = "macos"))]
fn collect_processes_macos() -> Result<Vec<ProcessInfo>, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "macOS process collection not available on this platform".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Windows — stubbed, requires windows-sys crate on Windows builds
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn collect_processes_windows() -> Result<Vec<ProcessInfo>, SensorError> {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    use windows_sys::Win32::Foundation::CloseHandle;

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        return Err(SensorError::ProbeFailed {
            probe: PROBE_ID.to_string(),
            reason: "CreateToolhelp32Snapshot failed".to_string(),
        });
    }

    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

    let mut processes = Vec::new();
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };

    while ok != 0 {
        let name_len = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);

        processes.push(ProcessInfo {
            pid: entry.th32ProcessID,
            name,
            cpu_percent: 0.0,
            memory_percent: 0.0,
            status: "running".to_string(),
        });

        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }

    unsafe { CloseHandle(snapshot) };
    Ok(processes)
}

#[cfg(not(target_os = "windows"))]
fn collect_processes_windows() -> Result<Vec<ProcessInfo>, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: PROBE_ID.to_string(),
        reason: "Windows process collection not available on this platform".to_string(),
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
