//! Temp file cleanup executor — scans and removes stale temporary files.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lso_core::{
    ActionExecutor, ActuatorError, CleanupProgress, CleanupResult, CleanupTarget, PreflightReport,
    RiskLevel, SkippedFile,
};
use tracing::{debug, warn};

use crate::open_files::open_file_paths;
use crate::snapshot::SnapshotManager;

const MIN_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const LOG_MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Executor for cleaning temporary files from well-known directories.
pub struct TempCleanupExecutor {
    target: CleanupTarget,
    dirs: Vec<PathBuf>,
    snapshot_mgr: SnapshotManager,
}

impl TempCleanupExecutor {
    /// Create a new executor for the given cleanup target.
    pub fn new(
        target: CleanupTarget,
        snapshot_mgr: SnapshotManager,
    ) -> Result<Self, ActuatorError> {
        let dirs = resolve_dirs(target);
        Ok(Self {
            target,
            dirs,
            snapshot_mgr,
        })
    }

    /// Scan directories and classify files as deletable or skipped.
    fn scan(&self) -> (Vec<FileEntry>, Vec<SkippedFile>) {
        let mut deletable = Vec::new();
        let mut skipped = Vec::new();
        let now = SystemTime::now();
        let open_files = open_file_paths();

        for dir in &self.dirs {
            // Resolve the root once (e.g. macOS /var -> /private/var) so child
            // paths match the kernel's real paths in the open-file set.
            let Ok(root) = fs::canonicalize(dir) else {
                continue;
            };
            self.scan_dir(&root, &mut deletable, &mut skipped, now, &open_files);
        }

        (deletable, skipped)
    }

    fn scan_dir(
        &self,
        dir: &Path,
        deletable: &mut Vec<FileEntry>,
        skipped: &mut Vec<SkippedFile>,
        now: SystemTime,
        open_files: &HashSet<PathBuf>,
    ) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                debug!(dir = %dir.display(), error = %e, "cannot read directory");
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // DirEntry::file_type does not follow symlinks. Never traverse or
            // delete through a link: it could point outside the cleanup root.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "symbolic link".into(),
                });
                continue;
            }
            if file_type.is_dir() {
                self.scan_dir(&path, deletable, skipped, now, open_files);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let meta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let modified = meta.modified().unwrap_or(now);
            let age = now.duration_since(modified).unwrap_or_default();

            if age < MIN_AGE {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "modified within last 24 hours".into(),
                });
                continue;
            }

            if self.target == CleanupTarget::AppLogs && age < LOG_MAX_AGE {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "log file younger than 30 days".into(),
                });
                continue;
            }

            if open_files.contains(&path) {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "file is in use by another process".into(),
                });
                continue;
            }

            deletable.push(FileEntry {
                path,
                size: meta.len(),
                modified,
            });
        }
    }
}

#[async_trait]
impl ActionExecutor for TempCleanupExecutor {
    fn action_id(&self) -> &str {
        "cleanup.temp_files"
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    async fn preflight(&self) -> Result<PreflightReport, ActuatorError> {
        let (deletable, skipped) = self.scan();

        let total_bytes: u64 = deletable.iter().map(|f| f.size).sum();
        let oldest = deletable
            .iter()
            .map(|f| f.modified)
            .min()
            .and_then(|t| DateTime::<Utc>::from(t).into());
        let newest = deletable
            .iter()
            .map(|f| f.modified)
            .max()
            .and_then(|t| DateTime::<Utc>::from(t).into());

        Ok(PreflightReport {
            target: self.target,
            file_count: deletable.len() as u64,
            total_bytes,
            oldest_modified: oldest,
            newest_modified: newest,
            skipped,
        })
    }

    async fn execute(
        &self,
        on_progress: Box<dyn Fn(CleanupProgress) + Send>,
    ) -> Result<CleanupResult, ActuatorError> {
        let start = std::time::Instant::now();
        let (deletable, skipped) = self.scan();
        let total = deletable.len() as u64;

        let file_paths: Vec<PathBuf> = deletable.iter().map(|f| f.path.clone()).collect();
        let snapshot_id = self.snapshot_mgr.create(&file_paths)?;

        let mut files_deleted = 0u64;
        let mut bytes_reclaimed = 0u64;
        let mut errors = Vec::new();

        for (i, entry) in deletable.iter().enumerate() {
            on_progress(CleanupProgress {
                processed: i as u64,
                total,
                current_file: entry.path.to_string_lossy().to_string(),
                bytes_so_far: bytes_reclaimed,
            });

            match fs::remove_file(&entry.path) {
                Ok(()) => {
                    files_deleted += 1;
                    bytes_reclaimed += entry.size;
                }
                Err(e) => {
                    warn!(path = %entry.path.display(), error = %e, "failed to delete file");
                    errors.push(format!("{}: {e}", entry.path.display()));
                }
            }
        }

        on_progress(CleanupProgress {
            processed: total,
            total,
            current_file: String::new(),
            bytes_so_far: bytes_reclaimed,
        });

        Ok(CleanupResult {
            files_deleted,
            bytes_reclaimed,
            errors,
            skipped,
            snapshot_id,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

struct FileEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

/// Resolve directories for a cleanup target using platform-appropriate paths.
fn resolve_dirs(target: CleanupTarget) -> Vec<PathBuf> {
    match target {
        CleanupTarget::SystemTemp => {
            vec![std::env::temp_dir()]
        }
        CleanupTarget::UserCache => {
            let mut dirs = Vec::new();
            if let Some(home) = dirs::home_dir() {
                // On macOS only ~/Library/Caches is the OS-designated cache.
                // ~/.cache there is used by CLI tools for costly data such as
                // LLM models (lm-studio) and Python environments (uv).
                if cfg!(target_os = "macos") {
                    dirs.push(home.join("Library/Caches"));
                } else {
                    dirs.push(home.join(".cache"));
                }
            }
            dirs
        }
        CleanupTarget::AppLogs => {
            let mut dirs = Vec::new();
            if let Some(home) = dirs::home_dir() {
                if cfg!(target_os = "macos") {
                    dirs.push(home.join("Library/Logs"));
                }
                dirs.push(home.join(".local/share/lso/logs"));
            }
            if cfg!(target_os = "linux") {
                dirs.push(PathBuf::from("/var/log"));
            }
            dirs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_dirs_system_temp_is_nonempty() {
        let dirs = resolve_dirs(CleanupTarget::SystemTemp);
        assert!(!dirs.is_empty());
    }

    #[test]
    fn resolve_dirs_user_cache_is_nonempty() {
        let dirs = resolve_dirs(CleanupTarget::UserCache);
        assert!(!dirs.is_empty());
    }

    #[tokio::test]
    async fn preflight_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let snap_dir = dir.path().join("snaps");
        let snap_mgr = SnapshotManager::new(snap_dir).unwrap();

        let mut executor = TempCleanupExecutor::new(CleanupTarget::SystemTemp, snap_mgr).unwrap();
        executor.dirs = vec![dir.path().join("empty")];
        fs::create_dir_all(&executor.dirs[0]).unwrap();

        let report = executor.preflight().await.unwrap();
        assert_eq!(report.file_count, 0);
        assert_eq!(report.total_bytes, 0);
    }

    #[tokio::test]
    async fn execute_deletes_old_files_and_skips_recent() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fs::create_dir_all(&source).unwrap();

        let old_file = source.join("old.tmp");
        fs::write(&old_file, "old content").unwrap();
        let old_time = SystemTime::now() - Duration::from_secs(48 * 3600);
        filetime::set_file_mtime(&old_file, filetime::FileTime::from_system_time(old_time))
            .unwrap_or_default();

        let new_file = source.join("new.tmp");
        fs::write(&new_file, "new content").unwrap();

        let snap_dir = dir.path().join("snaps");
        let snap_mgr = SnapshotManager::new(snap_dir).unwrap();
        let mut executor = TempCleanupExecutor::new(CleanupTarget::SystemTemp, snap_mgr).unwrap();
        executor.dirs = vec![source.clone()];

        let report = executor.preflight().await.unwrap();
        assert_eq!(report.file_count, 1);
        assert_eq!(report.skipped.len(), 1);

        let result = executor.execute(Box::new(|_| {})).await.unwrap();
        assert_eq!(result.files_deleted, 1);
        assert!(!old_file.exists());
        assert!(new_file.exists());
    }

    #[tokio::test]
    async fn rollback_restores_deleted_files() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        fs::create_dir_all(&source).unwrap();

        let file = source.join("deleteme.tmp");
        fs::write(&file, "precious data").unwrap();
        let old_time = SystemTime::now() - Duration::from_secs(48 * 3600);
        filetime::set_file_mtime(&file, filetime::FileTime::from_system_time(old_time))
            .unwrap_or_default();

        let snap_dir = dir.path().join("snaps");
        let snap_mgr = SnapshotManager::new(snap_dir.clone()).unwrap();
        let mut executor = TempCleanupExecutor::new(CleanupTarget::SystemTemp, snap_mgr).unwrap();
        executor.dirs = vec![source];

        let result = executor.execute(Box::new(|_| {})).await.unwrap();
        assert_eq!(result.files_deleted, 1);
        assert!(!file.exists());

        let restore_mgr = SnapshotManager::new(snap_dir).unwrap();
        let restored = restore_mgr.restore(&result.snapshot_id).unwrap();
        assert_eq!(restored, 1);
        assert_eq!(fs::read_to_string(&file).unwrap(), "precious data");
    }

    fn make_old(path: &Path) {
        let old_time = SystemTime::now() - Duration::from_secs(48 * 3600);
        filetime::set_file_mtime(path, filetime::FileTime::from_system_time(old_time)).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlinks_are_never_followed_or_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let precious = outside.join("precious.txt");
        fs::write(&precious, "keep me").unwrap();
        make_old(&precious);

        let root = dir.path().join("cache");
        fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("dir_link")).unwrap();
        std::os::unix::fs::symlink(&precious, root.join("file_link")).unwrap();

        let snap_mgr = SnapshotManager::new(dir.path().join("snaps")).unwrap();
        let mut executor = TempCleanupExecutor::new(CleanupTarget::UserCache, snap_mgr).unwrap();
        executor.dirs = vec![root.clone()];

        let report = executor.preflight().await.unwrap();
        assert_eq!(report.file_count, 0);
        assert_eq!(report.skipped.len(), 2);

        let result = executor.execute(Box::new(|_| {})).await.unwrap();
        assert_eq!(result.files_deleted, 0);
        assert!(precious.exists());
        assert!(root.join("dir_link").symlink_metadata().is_ok());
        assert!(root.join("file_link").symlink_metadata().is_ok());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[tokio::test]
    async fn open_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("cache");
        fs::create_dir_all(&root).unwrap();
        let held = root.join("held.tmp");
        fs::write(&held, "busy").unwrap();
        make_old(&held);
        let _handle = fs::File::open(&held).unwrap();

        let snap_mgr = SnapshotManager::new(dir.path().join("snaps")).unwrap();
        let mut executor = TempCleanupExecutor::new(CleanupTarget::UserCache, snap_mgr).unwrap();
        executor.dirs = vec![root];

        let report = executor.preflight().await.unwrap();
        assert_eq!(report.file_count, 0);
        assert_eq!(
            report.skipped[0].reason,
            "file is in use by another process"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_user_cache_excludes_dot_cache() {
        let dirs = resolve_dirs(CleanupTarget::UserCache);
        assert!(dirs.iter().all(|d| !d.ends_with(".cache")));
        assert!(dirs.iter().any(|d| d.ends_with("Library/Caches")));
    }
}
