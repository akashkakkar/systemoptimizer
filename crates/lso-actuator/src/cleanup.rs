//! Temp file cleanup executor — scans and removes stale temporary files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lso_core::{
    ActionExecutor, ActuatorError, CleanupProgress, CleanupResult, CleanupTarget, PreflightReport,
    RiskLevel, SkippedFile,
};
use tracing::{debug, warn};

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
        let open_files = get_open_files();

        for dir in &self.dirs {
            if !dir.exists() {
                continue;
            }
            self.scan_dir(dir, &mut deletable, &mut skipped, now, &open_files);
        }

        (deletable, skipped)
    }

    fn scan_dir(
        &self,
        dir: &Path,
        deletable: &mut Vec<FileEntry>,
        skipped: &mut Vec<SkippedFile>,
        now: SystemTime,
        open_files: &[PathBuf],
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

            if path.is_dir() {
                self.scan_dir(&path, deletable, skipped, now, open_files);
                continue;
            }

            let meta = match fs::metadata(&path) {
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

            if is_file_open(&path, open_files) {
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
                if cfg!(target_os = "macos") {
                    dirs.push(home.join("Library/Caches"));
                }
                dirs.push(home.join(".cache"));
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

/// Get list of open files via lsof (macOS/Linux) or handle.exe (Windows).
fn get_open_files() -> Vec<PathBuf> {
    if cfg!(target_os = "windows") {
        return Vec::new();
    }

    let output = Command::new("lsof")
        .args(["-F", "n"])
        .output();

    match output {
        Ok(out) => {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.strip_prefix('n'))
                .filter(|p| p.starts_with('/'))
                .map(PathBuf::from)
                .collect()
        }
        Err(e) => {
            warn!(error = %e, "lsof unavailable, skipping open-file check");
            Vec::new()
        }
    }
}

/// Check if a file is currently open by another process.
fn is_file_open(path: &Path, open_files: &[PathBuf]) -> bool {
    open_files.iter().any(|p| p == path)
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
        filetime::set_file_mtime(
            &old_file,
            filetime::FileTime::from_system_time(old_time),
        )
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

        let result = executor
            .execute(Box::new(|_| {}))
            .await
            .unwrap();
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
        filetime::set_file_mtime(
            &file,
            filetime::FileTime::from_system_time(old_time),
        )
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
}
