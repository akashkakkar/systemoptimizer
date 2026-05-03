//! Snapshot manager — copies files before deletion so they can be restored.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use lso_core::ActuatorError;
use tracing::info;

/// Manages file snapshots for rollback support.
pub struct SnapshotManager {
    base_dir: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager that stores snapshots under `base_dir`.
    pub fn new(base_dir: PathBuf) -> Result<Self, ActuatorError> {
        fs::create_dir_all(&base_dir).map_err(|e| {
            ActuatorError::SnapshotFailed(format!(
                "cannot create snapshot directory {}: {e}",
                base_dir.display()
            ))
        })?;
        Ok(Self { base_dir })
    }

    /// Create a snapshot of the given files, returning the snapshot ID.
    pub fn create(&self, files: &[PathBuf]) -> Result<String, ActuatorError> {
        let snapshot_id = format!("snap-{}", Utc::now().format("%Y%m%d-%H%M%S-%3f"));
        let snapshot_dir = self.base_dir.join(&snapshot_id);
        fs::create_dir_all(&snapshot_dir).map_err(|e| {
            ActuatorError::SnapshotFailed(format!("cannot create {}: {e}", snapshot_dir.display()))
        })?;

        let mut manifest = Vec::new();
        for src in files {
            if !src.exists() {
                continue;
            }
            let relative = self.relative_key(src);
            let dest = snapshot_dir.join(&relative);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    ActuatorError::SnapshotFailed(format!(
                        "cannot create parent dir for {}: {e}",
                        dest.display()
                    ))
                })?;
            }
            fs::copy(src, &dest).map_err(|e| {
                ActuatorError::SnapshotFailed(format!(
                    "cannot copy {} → {}: {e}",
                    src.display(),
                    dest.display()
                ))
            })?;
            manifest.push((src.to_string_lossy().to_string(), relative));
        }

        let manifest_path = snapshot_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| {
            ActuatorError::SnapshotFailed(format!("cannot serialize manifest: {e}"))
        })?;
        fs::write(&manifest_path, manifest_json).map_err(|e| {
            ActuatorError::SnapshotFailed(format!(
                "cannot write manifest {}: {e}",
                manifest_path.display()
            ))
        })?;

        info!(snapshot_id = %snapshot_id, file_count = manifest.len(), "snapshot created");
        Ok(snapshot_id)
    }

    /// Restore all files from a snapshot to their original locations.
    pub fn restore(&self, snapshot_id: &str) -> Result<u64, ActuatorError> {
        let snapshot_dir = self.base_dir.join(snapshot_id);
        let manifest_path = snapshot_dir.join("manifest.json");

        let manifest_json = fs::read_to_string(&manifest_path).map_err(|e| {
            ActuatorError::RollbackFailed(format!(
                "cannot read manifest {}: {e}",
                manifest_path.display()
            ))
        })?;
        let manifest: Vec<(String, String)> = serde_json::from_str(&manifest_json).map_err(|e| {
            ActuatorError::RollbackFailed(format!("cannot parse manifest: {e}"))
        })?;

        let mut restored = 0u64;
        for (original_path, relative) in &manifest {
            let src = snapshot_dir.join(relative);
            let dest = Path::new(original_path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    ActuatorError::RollbackFailed(format!(
                        "cannot create parent dir for {}: {e}",
                        dest.display()
                    ))
                })?;
            }
            fs::copy(&src, dest).map_err(|e| {
                ActuatorError::RollbackFailed(format!(
                    "cannot restore {} → {}: {e}",
                    src.display(),
                    dest.display()
                ))
            })?;
            restored += 1;
        }

        info!(snapshot_id = %snapshot_id, restored_count = restored, "snapshot restored");
        Ok(restored)
    }

    /// Convert an absolute path to a safe relative key for storage.
    fn relative_key(&self, path: &Path) -> String {
        let s = path.to_string_lossy();
        let stripped = s
            .strip_prefix('/')
            .or_else(|| s.strip_prefix("\\\\"))
            .unwrap_or(&s);
        stripped.replace(['/', '\\'], "_SEP_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_and_restore_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let snapshot_base = dir.path().join("snapshots");
        let mgr = SnapshotManager::new(snapshot_base).unwrap();

        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();
        let file_a = source_dir.join("a.txt");
        let file_b = source_dir.join("b.txt");
        fs::write(&file_a, "hello").unwrap();
        fs::write(&file_b, "world").unwrap();

        let snap_id = mgr.create(&[file_a.clone(), file_b.clone()]).unwrap();
        assert!(snap_id.starts_with("snap-"));

        fs::remove_file(&file_a).unwrap();
        fs::remove_file(&file_b).unwrap();
        assert!(!file_a.exists());

        let restored = mgr.restore(&snap_id).unwrap();
        assert_eq!(restored, 2);
        assert_eq!(fs::read_to_string(&file_a).unwrap(), "hello");
        assert_eq!(fs::read_to_string(&file_b).unwrap(), "world");
    }

    #[test]
    fn snapshot_skips_nonexistent_files() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SnapshotManager::new(dir.path().join("snaps")).unwrap();

        let snap_id = mgr
            .create(&[PathBuf::from("/nonexistent/file.txt")])
            .unwrap();

        let restored = mgr.restore(&snap_id).unwrap();
        assert_eq!(restored, 0);
    }

    #[test]
    fn restore_nonexistent_snapshot_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SnapshotManager::new(dir.path().join("snaps")).unwrap();
        let result = mgr.restore("snap-nonexistent");
        assert!(result.is_err());
    }
}
