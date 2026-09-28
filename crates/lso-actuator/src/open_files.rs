//! Enumerate files currently held open by any process.
//!
//! Used by cleanup executors to avoid deleting in-use files. Implemented with
//! native APIs (libproc on macOS, `/proc` on Linux) instead of shelling out to
//! `lsof`, which took ~10 s on a typical macOS desktop. Processes we are not
//! permitted to inspect are silently skipped.

use std::collections::HashSet;
use std::path::PathBuf;

/// Return the set of absolute paths currently open by any inspectable process.
///
/// Returns an empty set on platforms without an implementation.
pub fn open_file_paths() -> HashSet<PathBuf> {
    imp::open_file_paths()
}

#[cfg(target_os = "macos")]
mod imp {
    use std::collections::HashSet;
    use std::ffi::CStr;
    use std::mem::size_of;
    use std::path::PathBuf;

    const PROC_PIDFDVNODEPATHINFO: libc::c_int = 2;

    /// Mirrors `struct proc_fileinfo` from `<sys/proc_info.h>`.
    #[repr(C)]
    struct ProcFileInfo {
        fi_openflags: u32,
        fi_status: u32,
        fi_offset: libc::off_t,
        fi_type: i32,
        fi_guardflags: u32,
    }

    /// Mirrors `struct vnode_fdinfowithpath` from `<sys/proc_info.h>`.
    #[repr(C)]
    struct VnodeFdInfoWithPath {
        pfi: ProcFileInfo,
        pvip: libc::vnode_info_path,
    }

    // Kernel ABI size; a mismatch would make proc_pidfdinfo reject the buffer.
    const _: () = assert!(size_of::<VnodeFdInfoWithPath>() == 1200);

    pub fn open_file_paths() -> HashSet<PathBuf> {
        let mut paths = HashSet::new();
        for pid in list_pids() {
            for fd in list_vnode_fds(pid) {
                if let Some(path) = fd_path(pid, fd) {
                    paths.insert(path);
                }
            }
        }
        paths
    }

    fn list_pids() -> Vec<libc::pid_t> {
        // SAFETY: a null buffer asks libproc for the current pid count.
        let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
        if count <= 0 {
            return Vec::new();
        }
        // Headroom for processes spawned between the two calls.
        let mut pids = vec![0 as libc::pid_t; count as usize + 64];
        let buf_bytes = (pids.len() * size_of::<libc::pid_t>()) as libc::c_int;
        // SAFETY: buffer is valid for `buf_bytes` bytes.
        let n = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), buf_bytes) };
        pids.truncate(n.max(0) as usize);
        pids
    }

    fn list_vnode_fds(pid: libc::pid_t) -> Vec<i32> {
        // SAFETY: a null buffer asks for the required size in bytes.
        let bytes =
            unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
        if bytes <= 0 {
            return Vec::new();
        }
        let cap = bytes as usize / size_of::<libc::proc_fdinfo>() + 16;
        let mut fds: Vec<libc::proc_fdinfo> = Vec::with_capacity(cap);
        let buf_bytes = (cap * size_of::<libc::proc_fdinfo>()) as libc::c_int;
        // SAFETY: buffer has capacity for `cap` entries; the kernel reports bytes written.
        let written = unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDLISTFDS,
                0,
                fds.as_mut_ptr().cast(),
                buf_bytes,
            )
        };
        if written <= 0 {
            return Vec::new();
        }
        // SAFETY: the kernel initialised `written` bytes of whole proc_fdinfo entries.
        unsafe { fds.set_len(written as usize / size_of::<libc::proc_fdinfo>()) };
        fds.into_iter()
            .filter(|f| f.proc_fdtype == libc::PROX_FDTYPE_VNODE as u32)
            .map(|f| f.proc_fd)
            .collect()
    }

    fn fd_path(pid: libc::pid_t, fd: i32) -> Option<PathBuf> {
        let mut info = std::mem::MaybeUninit::<VnodeFdInfoWithPath>::zeroed();
        let size = size_of::<VnodeFdInfoWithPath>() as libc::c_int;
        // SAFETY: buffer is exactly one VnodeFdInfoWithPath.
        let ret = unsafe {
            libc::proc_pidfdinfo(
                pid,
                fd,
                PROC_PIDFDVNODEPATHINFO,
                info.as_mut_ptr().cast(),
                size,
            )
        };
        if ret != size {
            return None;
        }
        // SAFETY: the kernel filled the struct; zeroed memory is a valid bit pattern.
        let info = unsafe { info.assume_init() };
        // vip_path is a NUL-terminated MAXPATHLEN buffer split into 32x32 chunks.
        // SAFETY: the array is contiguous, 1024 bytes, and zero-initialised.
        let raw = unsafe {
            std::slice::from_raw_parts(info.pvip.vip_path.as_ptr().cast::<u8>(), 32 * 32)
        };
        let path = CStr::from_bytes_until_nul(raw).ok()?.to_str().ok()?;
        if path.starts_with('/') {
            Some(PathBuf::from(path))
        } else {
            None
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    pub fn open_file_paths() -> HashSet<PathBuf> {
        let mut paths = HashSet::new();
        let Ok(procs) = fs::read_dir("/proc") else {
            return paths;
        };
        for proc_entry in procs.flatten() {
            let is_pid = proc_entry
                .file_name()
                .to_str()
                .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()));
            if !is_pid {
                continue;
            }
            // Permission denied for other users' processes is expected.
            let Ok(fds) = fs::read_dir(proc_entry.path().join("fd")) else {
                continue;
            };
            for fd in fds.flatten() {
                if let Ok(target) = fs::read_link(fd.path()) {
                    if target.is_absolute() {
                        paths.insert(target);
                    }
                }
            }
        }
        paths
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod imp {
    use std::collections::HashSet;
    use std::path::PathBuf;

    pub fn open_file_paths() -> HashSet<PathBuf> {
        HashSet::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn detects_file_held_open_by_this_process() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("held.txt");
        let _handle = File::create(&path).unwrap();

        let open = open_file_paths();
        let canonical = path.canonicalize().unwrap();
        assert!(
            open.contains(&canonical) || open.contains(&path),
            "expected {} in open-file set ({} entries)",
            path.display(),
            open.len()
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn closed_file_is_not_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("closed.txt");
        drop(File::create(&path).unwrap());

        let open = open_file_paths();
        let canonical = path.canonicalize().unwrap();
        assert!(!open.contains(&canonical) && !open.contains(&path));
    }
}
