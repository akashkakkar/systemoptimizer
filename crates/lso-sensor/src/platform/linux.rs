//! Linux disk usage collection via /proc/mounts + statvfs.

use std::fs;

use nix::sys::statvfs::statvfs;

use super::{is_pseudo_fs, MountStats};
use lso_core::SensorError;

pub fn collect() -> Result<Vec<MountStats>, SensorError> {
    let mounts_content = fs::read_to_string("/proc/mounts").map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            SensorError::PermissionRequired("cannot read /proc/mounts".to_string())
        } else {
            SensorError::Io(e)
        }
    })?;

    let mut results = Vec::new();

    for line in mounts_content.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }

        let mount_point = fields[1];
        let fs_type = fields[2];

        if is_pseudo_fs(fs_type) {
            continue;
        }

        match statvfs(mount_point) {
            Ok(stat) => {
                let block_size = stat.block_size() as u64;
                let total_bytes = stat.blocks() * block_size;
                let available_bytes = stat.blocks_available() * block_size;
                let free_bytes = stat.blocks_free() * block_size;
                let used_bytes = total_bytes.saturating_sub(free_bytes);

                let usage_percent = if total_bytes == 0 {
                    0.0
                } else {
                    (used_bytes as f64 / total_bytes as f64) * 100.0
                };

                results.push(MountStats {
                    mount_point: mount_point.to_string(),
                    fs_type: fs_type.to_string(),
                    total_bytes,
                    used_bytes,
                    available_bytes,
                    usage_percent,
                });
            }
            Err(nix::errno::Errno::EACCES | nix::errno::Errno::EPERM) => {
                tracing::warn!(mount_point, "permission denied, skipping");
            }
            Err(_) => {
                tracing::debug!(mount_point, "statvfs failed, skipping");
            }
        }
    }

    Ok(results)
}
