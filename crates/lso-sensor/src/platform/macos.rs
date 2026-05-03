//! macOS disk usage collection via getfsstat + statvfs.

use std::ffi::CStr;
use std::mem;

use nix::sys::statvfs::statvfs;

use super::{is_pseudo_fs, MountStats};
use lso_core::SensorError;

/// Collect disk stats for all mounted volumes on macOS using getfsstat.
pub fn collect() -> Result<Vec<MountStats>, SensorError> {
    let mounts = get_mount_points()?;
    let mut results = Vec::new();

    for (mount_point, fs_type) in &mounts {
        if is_pseudo_fs(fs_type) {
            continue;
        }

        match statvfs(mount_point.as_str()) {
            Ok(stat) => {
                // POSIX: total bytes = f_frsize * f_blocks. f_bsize is the I/O hint
                // and on APFS is ~256× larger than f_frsize, which would massively
                // overstate capacity.
                let frag_size = stat.fragment_size();
                let total_bytes = u64::from(stat.blocks()) * frag_size;
                let available_bytes = u64::from(stat.blocks_available()) * frag_size;
                let free_bytes = u64::from(stat.blocks_free()) * frag_size;
                let used_bytes = total_bytes.saturating_sub(free_bytes);

                let usage_percent = if total_bytes == 0 {
                    0.0
                } else {
                    (used_bytes as f64 / total_bytes as f64) * 100.0
                };

                results.push(MountStats {
                    mount_point: mount_point.clone(),
                    fs_type: fs_type.clone(),
                    total_bytes,
                    used_bytes,
                    available_bytes,
                    usage_percent,
                });
            }
            Err(nix::errno::Errno::EACCES | nix::errno::Errno::EPERM) => {
                tracing::warn!(mount_point = mount_point.as_str(), "permission denied, skipping");
            }
            Err(_) => {
                tracing::debug!(mount_point = mount_point.as_str(), "statvfs failed, skipping");
            }
        }
    }

    Ok(results)
}

fn get_mount_points() -> Result<Vec<(String, String)>, SensorError> {
    let count = unsafe { libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT) };
    if count < 0 {
        return Err(SensorError::Io(std::io::Error::last_os_error()));
    }

    let buf_size = count as usize * mem::size_of::<libc::statfs>();
    let mut buf: Vec<libc::statfs> = Vec::with_capacity(count as usize);

    let actual_count = unsafe {
        libc::getfsstat(
            buf.as_mut_ptr(),
            buf_size as libc::c_int,
            libc::MNT_NOWAIT,
        )
    };

    if actual_count < 0 {
        return Err(SensorError::Io(std::io::Error::last_os_error()));
    }

    unsafe { buf.set_len(actual_count as usize) };

    let mut mounts = Vec::new();
    for entry in &buf {
        let mount_point = unsafe { CStr::from_ptr(entry.f_mntonname.as_ptr()) }
            .to_string_lossy()
            .into_owned();

        let fs_type = unsafe { CStr::from_ptr(entry.f_fstypename.as_ptr()) }
            .to_string_lossy()
            .into_owned();

        mounts.push((mount_point, fs_type));
    }

    Ok(mounts)
}
