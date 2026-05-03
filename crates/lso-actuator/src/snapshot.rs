//! Snapshot providers for rollback safety.

use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use lso_core::{ActuatorError, SnapshotId, SnapshotInfo, SnapshotProvider};

struct SnapshotState {
    infos: Vec<SnapshotInfo>,
    archive_paths: HashMap<SnapshotId, PathBuf>,
}

/// Tar/gzip-based snapshot provider. Works on all platforms.
pub struct TarSnapshotProvider {
    target_dir: PathBuf,
    snapshot_dir: PathBuf,
    state: Mutex<SnapshotState>,
}

impl TarSnapshotProvider {
    /// Create a new provider that snapshots `target_dir` into `snapshot_dir`.
    pub fn new(target_dir: PathBuf, snapshot_dir: PathBuf) -> Result<Self, ActuatorError> {
        fs::create_dir_all(&snapshot_dir)?;
        Ok(Self {
            target_dir,
            snapshot_dir,
            state: Mutex::new(SnapshotState {
                infos: Vec::new(),
                archive_paths: HashMap::new(),
            }),
        })
    }
}

#[async_trait]
impl SnapshotProvider for TarSnapshotProvider {
    async fn create(&self, label: &str) -> Result<SnapshotId, ActuatorError> {
        let id = uuid::Uuid::new_v4().to_string();
        let archive_path = self.snapshot_dir.join(format!("{id}.tar.gz"));
        let target = self.target_dir.clone();
        let archive = archive_path.clone();

        tokio::task::spawn_blocking(move || -> Result<(), ActuatorError> {
            let file = File::create(&archive)
                .map_err(|e| ActuatorError::SnapshotFailed(format!("create archive: {e}")))?;
            let enc = GzEncoder::new(file, Compression::default());
            let mut builder = tar::Builder::new(enc);
            builder
                .append_dir_all(".", &target)
                .map_err(|e| ActuatorError::SnapshotFailed(format!("archive directory: {e}")))?;
            builder
                .into_inner()
                .map_err(|e| ActuatorError::SnapshotFailed(format!("finalize tar: {e}")))?
                .finish()
                .map_err(|e| ActuatorError::SnapshotFailed(format!("finalize gzip: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| ActuatorError::SnapshotFailed(format!("task join: {e}")))??;

        let size = fs::metadata(&archive_path).ok().map(|m| m.len());
        let info = SnapshotInfo {
            id: id.clone(),
            label: label.to_string(),
            created_at: Utc::now(),
            size_estimate: size,
            platform_detail: "tar.gz archive".to_string(),
        };

        let mut state = self.state.lock().expect("snapshot state lock poisoned");
        state.infos.push(info);
        state.archive_paths.insert(id.clone(), archive_path);

        Ok(id)
    }

    async fn list(&self) -> Result<Vec<SnapshotInfo>, ActuatorError> {
        let state = self.state.lock().expect("snapshot state lock poisoned");
        Ok(state.infos.clone())
    }

    async fn restore(&self, id: &SnapshotId) -> Result<(), ActuatorError> {
        let archive_path = {
            let state = self.state.lock().expect("snapshot state lock poisoned");
            state
                .archive_paths
                .get(id)
                .cloned()
                .ok_or_else(|| ActuatorError::SnapshotFailed(format!("snapshot not found: {id}")))?
        };

        let target = self.target_dir.clone();

        tokio::task::spawn_blocking(move || -> Result<(), ActuatorError> {
            clear_directory(&target)?;

            let file = File::open(&archive_path)
                .map_err(|e| ActuatorError::SnapshotFailed(format!("open archive: {e}")))?;
            let dec = GzDecoder::new(file);
            let mut archive = tar::Archive::new(dec);
            archive
                .unpack(&target)
                .map_err(|e| ActuatorError::SnapshotFailed(format!("extract archive: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| ActuatorError::SnapshotFailed(format!("task join: {e}")))?
    }

    async fn delete(&self, id: &SnapshotId) -> Result<(), ActuatorError> {
        let archive_path = {
            let mut state = self.state.lock().expect("snapshot state lock poisoned");
            let path = state
                .archive_paths
                .remove(id)
                .ok_or_else(|| ActuatorError::SnapshotFailed(format!("snapshot not found: {id}")))?;
            state.infos.retain(|s| s.id != *id);
            path
        };

        fs::remove_file(&archive_path)
            .map_err(|e| ActuatorError::SnapshotFailed(format!("delete archive: {e}")))?;

        Ok(())
    }

    fn supports_platform(&self) -> bool {
        true
    }
}

fn clear_directory(dir: &Path) -> Result<(), ActuatorError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn create_and_list_snapshots() {
        let target = TempDir::new().unwrap();
        let snap_dir = TempDir::new().unwrap();

        fs::write(target.path().join("file1.txt"), "hello").unwrap();
        fs::write(target.path().join("file2.txt"), "world").unwrap();

        let provider =
            TarSnapshotProvider::new(target.path().to_path_buf(), snap_dir.path().to_path_buf())
                .unwrap();

        let id = provider.create("test-snapshot").await.unwrap();

        let snapshots = provider.list().await.unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, id);
        assert_eq!(snapshots[0].label, "test-snapshot");
        assert!(snapshots[0].size_estimate.is_some());
    }

    #[tokio::test]
    async fn create_and_restore_snapshot() {
        let target = TempDir::new().unwrap();
        let snap_dir = TempDir::new().unwrap();

        fs::write(target.path().join("file1.txt"), "original").unwrap();
        fs::create_dir(target.path().join("subdir")).unwrap();
        fs::write(target.path().join("subdir/file2.txt"), "nested").unwrap();

        let provider =
            TarSnapshotProvider::new(target.path().to_path_buf(), snap_dir.path().to_path_buf())
                .unwrap();

        let id = provider.create("before-change").await.unwrap();

        fs::write(target.path().join("file1.txt"), "modified").unwrap();
        fs::remove_dir_all(target.path().join("subdir")).unwrap();
        fs::write(target.path().join("new_file.txt"), "added").unwrap();

        provider.restore(&id).await.unwrap();

        assert_eq!(
            fs::read_to_string(target.path().join("file1.txt")).unwrap(),
            "original"
        );
        assert_eq!(
            fs::read_to_string(target.path().join("subdir/file2.txt")).unwrap(),
            "nested"
        );
        assert!(!target.path().join("new_file.txt").exists());
    }

    #[tokio::test]
    async fn delete_snapshot() {
        let target = TempDir::new().unwrap();
        let snap_dir = TempDir::new().unwrap();

        fs::write(target.path().join("file.txt"), "data").unwrap();

        let provider =
            TarSnapshotProvider::new(target.path().to_path_buf(), snap_dir.path().to_path_buf())
                .unwrap();

        let id = provider.create("to-delete").await.unwrap();
        assert_eq!(provider.list().await.unwrap().len(), 1);

        provider.delete(&id).await.unwrap();
        assert_eq!(provider.list().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn restore_nonexistent_snapshot_fails() {
        let target = TempDir::new().unwrap();
        let snap_dir = TempDir::new().unwrap();

        let provider =
            TarSnapshotProvider::new(target.path().to_path_buf(), snap_dir.path().to_path_buf())
                .unwrap();

        let result = provider.restore(&"nonexistent".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn supports_platform_returns_true() {
        let target = TempDir::new().unwrap();
        let snap_dir = TempDir::new().unwrap();

        let provider =
            TarSnapshotProvider::new(target.path().to_path_buf(), snap_dir.path().to_path_buf())
                .unwrap();

        assert!(provider.supports_platform());
    }
}
