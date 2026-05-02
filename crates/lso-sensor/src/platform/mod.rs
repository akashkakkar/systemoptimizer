//! Platform-specific mount and disk stat collection.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use lso_core::{Platform, SensorError};

/// Raw stats for a single mount point.
#[derive(Debug, Clone)]
pub struct MountStats {
    pub mount_point: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f64,
}

/// Pseudo-filesystem types to exclude.
const PSEUDO_FS: &[&str] = &[
    "proc", "sysfs", "devfs", "devtmpfs", "tmpfs", "securityfs", "debugfs", "cgroup",
    "cgroup2", "pstore", "bpf", "tracefs", "hugetlbfs", "mqueue", "fusectl", "configfs",
    "autofs", "devpts", "ramfs", "overlay",
];

/// Returns true if this filesystem type is a pseudo-filesystem that should be excluded.
pub fn is_pseudo_fs(fs_type: &str) -> bool {
    PSEUDO_FS.contains(&fs_type)
}

/// Collect mount statistics for all real filesystems on the given platform.
pub fn collect_mount_stats(platform: Platform) -> Result<Vec<MountStats>, SensorError> {
    match platform {
        #[cfg(target_os = "linux")]
        Platform::Linux => linux::collect(),
        #[cfg(target_os = "macos")]
        Platform::MacOS => macos::collect(),
        #[cfg(target_os = "windows")]
        Platform::Windows => windows::collect(),

        _ => Err(SensorError::PlatformNotSupported(format!(
            "{platform:?} is not supported on this build"
        ))),
    }
}
