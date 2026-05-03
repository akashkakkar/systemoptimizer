//! Windows disk usage via GetLogicalDriveStringsW + GetDiskFreeSpaceExW.

use super::MountStats;
use lso_core::SensorError;

#[cfg(target_os = "windows")]
pub fn collect() -> Result<Vec<MountStats>, SensorError> {
    use windows_sys::Win32::Storage::FileSystem::*;

    let mut drive_buf = [0u16; 256];
    let len = unsafe { GetLogicalDriveStringsW(drive_buf.len() as u32, drive_buf.as_mut_ptr()) };
    if len == 0 {
        return Err(SensorError::ProbeFailed {
            probe: "disk.usage".to_string(),
            reason: "GetLogicalDriveStringsW failed".to_string(),
        });
    }

    let mut results = Vec::new();
    let mut offset = 0usize;

    while offset < len as usize {
        let end = drive_buf[offset..]
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(0);
        if end == 0 {
            break;
        }

        let drive_path = &drive_buf[offset..offset + end + 1];
        let drive_name = String::from_utf16_lossy(&drive_buf[offset..offset + end]);
        offset += end + 1;

        let drive_type = unsafe { GetDriveTypeW(drive_path.as_ptr()) };
        if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE {
            continue;
        }

        let mut free_caller: u64 = 0;
        let mut total: u64 = 0;
        let mut free_total: u64 = 0;

        let ok = unsafe {
            GetDiskFreeSpaceExW(
                drive_path.as_ptr(),
                &mut free_caller,
                &mut total,
                &mut free_total,
            )
        };

        if ok == 0 {
            continue;
        }

        let used = total.saturating_sub(free_total);
        let usage_percent = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        results.push(MountStats {
            mount_point: drive_name,
            fs_type: "NTFS".to_string(),
            total_bytes: total,
            used_bytes: used,
            available_bytes: free_caller,
            usage_percent,
        });
    }

    Ok(results)
}

#[cfg(not(target_os = "windows"))]
pub fn collect() -> Result<Vec<MountStats>, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: "disk.usage".to_string(),
        reason: "Windows disk probe not available on this platform".to_string(),
    })
}
