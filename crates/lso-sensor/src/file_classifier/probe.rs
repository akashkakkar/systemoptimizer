//! FileClassificationProbe — directory walker with progress and optional SHA-256 duplicate detection.

use super::classify::classify_by_extension;
use chrono::{DateTime, Utc};
use lso_core::{
    CategoryStats, ClassificationResult, DuplicateCluster, FileCategory, FileInfo, ScanConfig,
    ScanProgress, SensorError,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Read-only probe that classifies files in user-approved directories.
pub struct FileClassificationProbe;

impl FileClassificationProbe {
    /// Run a classification scan over the configured directories.
    ///
    /// Sends `ScanProgress` updates through `progress_tx` if provided.
    /// Respects `cancel` — checks between files.
    pub async fn scan(
        config: &ScanConfig,
        progress_tx: Option<mpsc::Sender<ScanProgress>>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<ClassificationResult, SensorError> {
        let scan_id = Uuid::new_v4();
        let started_at = Utc::now();

        for dir in &config.directories {
            if !dir.exists() {
                return Err(SensorError::ProbeFailed {
                    probe: "file_classifier".into(),
                    reason: format!("directory not found: {}", dir.display()),
                });
            }
        }

        let mut files: Vec<FileInfo> = Vec::new();
        let mut scanned: u64 = 0;
        let mut bytes_scanned: u64 = 0;

        for dir in &config.directories {
            walk_directory(
                dir,
                config.max_depth.unwrap_or(usize::MAX),
                0,
                &cancel,
                &mut |path, metadata| {
                    if cancel.is_cancelled() {
                        return Err(SensorError::ProbeFailed {
                            probe: "file_classifier".into(),
                            reason: "scan cancelled by user".into(),
                        });
                    }

                    let info = build_file_info(&path, &metadata);
                    scanned += 1;
                    bytes_scanned += info.size_bytes;

                    if let Some(tx) = &progress_tx {
                        let progress = ScanProgress {
                            files_scanned: scanned,
                            files_total_estimate: None,
                            current_directory: path
                                .parent()
                                .unwrap_or_else(|| Path::new(""))
                                .to_path_buf(),
                            bytes_scanned,
                        };
                        let _ = tx.try_send(progress);
                    }

                    files.push(info);
                    Ok(())
                },
            )?;
        }

        if config.detect_duplicates {
            compute_hashes(&mut files, &cancel)?;
        }

        let category_summary = build_category_summary(&files);
        let duplicate_clusters = find_duplicate_clusters(&files);
        let completed_at = Utc::now();

        info!(
            scan_id = %scan_id,
            files = files.len(),
            duplicates = duplicate_clusters.len(),
            "file classification scan complete"
        );

        Ok(ClassificationResult {
            scan_id,
            started_at,
            completed_at,
            files,
            category_summary,
            duplicate_clusters,
        })
    }
}

fn walk_directory(
    dir: &Path,
    max_depth: usize,
    current_depth: usize,
    cancel: &tokio_util::sync::CancellationToken,
    visitor: &mut dyn FnMut(PathBuf, fs::Metadata) -> Result<(), SensorError>,
) -> Result<(), SensorError> {
    if cancel.is_cancelled() {
        return Err(SensorError::ProbeFailed {
            probe: "file_classifier".into(),
            reason: "scan cancelled by user".into(),
        });
    }
    if current_depth > max_depth {
        return Ok(());
    }

    let entries = fs::read_dir(dir).map_err(SensorError::Io)?;

    for entry in entries {
        if cancel.is_cancelled() {
            return Err(SensorError::ProbeFailed {
                probe: "file_classifier".into(),
                reason: "scan cancelled by user".into(),
            });
        }

        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!(path = %dir.display(), error = %e, "skipping unreadable entry");
                continue;
            }
        };

        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                debug!(path = %path.display(), error = %e, "skipping inaccessible file");
                continue;
            }
        };

        if metadata.is_symlink() {
            continue;
        }

        if metadata.is_dir() {
            walk_directory(&path, max_depth, current_depth + 1, cancel, visitor)?;
        } else if metadata.is_file() {
            visitor(path, metadata)?;
        }
    }

    Ok(())
}

fn build_file_info(path: &Path, metadata: &fs::Metadata) -> FileInfo {
    let category = classify_by_extension(path);
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());

    let to_utc = |t: std::io::Result<std::time::SystemTime>| -> Option<DateTime<Utc>> {
        t.ok().map(DateTime::<Utc>::from)
    };

    FileInfo {
        path: path.to_path_buf(),
        category,
        extension,
        size_bytes: metadata.len(),
        created: to_utc(metadata.created()),
        modified: to_utc(metadata.modified()),
        accessed: to_utc(metadata.accessed()),
        content_hash: None,
    }
}

fn compute_hashes(
    files: &mut [FileInfo],
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<(), SensorError> {
    for file in files.iter_mut() {
        if cancel.is_cancelled() {
            return Err(SensorError::ProbeFailed {
                probe: "file_classifier".into(),
                reason: "scan cancelled by user".into(),
            });
        }

        match hash_file(&file.path) {
            Ok(hash) => file.content_hash = Some(hash),
            Err(e) => {
                debug!(path = %file.path.display(), error = %e, "skipping hash for inaccessible file");
            }
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn build_category_summary(files: &[FileInfo]) -> HashMap<FileCategory, CategoryStats> {
    let mut summary = HashMap::new();
    for file in files {
        let stats = summary
            .entry(file.category)
            .or_insert_with(CategoryStats::default);
        stats.count += 1;
        stats.total_bytes += file.size_bytes;
    }
    summary
}

fn find_duplicate_clusters(files: &[FileInfo]) -> Vec<DuplicateCluster> {
    let mut hash_groups: HashMap<&str, Vec<&FileInfo>> = HashMap::new();

    for file in files {
        if let Some(ref hash) = file.content_hash {
            hash_groups.entry(hash.as_str()).or_default().push(file);
        }
    }

    hash_groups
        .into_iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(hash, group)| {
            let file_size = group[0].size_bytes;
            let count = group.len() as u64;
            DuplicateCluster {
                hash: hash.to_string(),
                files: group.iter().map(|f| f.path.clone()).collect(),
                file_size,
                wasted_bytes: file_size * (count - 1),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_tree(dir: &Path) {
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::create_dir_all(dir.join("code")).unwrap();
        fs::create_dir_all(dir.join("media")).unwrap();

        fs::write(dir.join("docs/report.pdf"), b"fake pdf").unwrap();
        fs::write(dir.join("docs/notes.txt"), b"some notes").unwrap();
        fs::write(dir.join("code/main.rs"), b"fn main() {}").unwrap();
        fs::write(dir.join("code/Cargo.toml"), b"[package]").unwrap();
        fs::write(dir.join("media/photo.jpg"), b"fake jpeg").unwrap();
        fs::write(dir.join("data.csv"), b"a,b,c").unwrap();
        fs::write(dir.join("archive.zip"), b"PK fake").unwrap();
        fs::write(dir.join("temp.tmp"), b"garbage").unwrap();
        fs::write(dir.join("mystery.xyz"), b"???").unwrap();
    }

    #[tokio::test]
    async fn scan_classifies_files() {
        let tmp = TempDir::new().unwrap();
        create_test_tree(tmp.path());

        let config = ScanConfig {
            directories: vec![tmp.path().to_path_buf()],
            detect_duplicates: false,
            max_depth: None,
        };

        let cancel = tokio_util::sync::CancellationToken::new();
        let result = FileClassificationProbe::scan(&config, None, cancel).await.unwrap();

        assert_eq!(result.files.len(), 9);

        let cats: HashMap<FileCategory, usize> = {
            let mut m = HashMap::new();
            for f in &result.files {
                *m.entry(f.category).or_default() += 1;
            }
            m
        };

        assert_eq!(cats[&FileCategory::Documents], 2);
        assert_eq!(cats[&FileCategory::Code], 2);
        assert_eq!(cats[&FileCategory::Media], 1);
        assert_eq!(cats[&FileCategory::Data], 1);
        assert_eq!(cats[&FileCategory::Archives], 1);
        assert_eq!(cats[&FileCategory::Temporary], 1);
        assert_eq!(cats[&FileCategory::Unknown], 1);
    }

    #[tokio::test]
    async fn scan_detects_duplicates() {
        let tmp = TempDir::new().unwrap();
        let content = b"identical content here";
        fs::write(tmp.path().join("a.txt"), content).unwrap();
        fs::write(tmp.path().join("b.txt"), content).unwrap();
        fs::write(tmp.path().join("c.txt"), b"different").unwrap();

        let config = ScanConfig {
            directories: vec![tmp.path().to_path_buf()],
            detect_duplicates: true,
            max_depth: None,
        };

        let cancel = tokio_util::sync::CancellationToken::new();
        let result = FileClassificationProbe::scan(&config, None, cancel).await.unwrap();

        assert_eq!(result.duplicate_clusters.len(), 1);
        assert_eq!(result.duplicate_clusters[0].files.len(), 2);
        assert_eq!(result.duplicate_clusters[0].wasted_bytes, content.len() as u64);
    }

    #[tokio::test]
    async fn scan_reports_progress() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), b"hello").unwrap();
        fs::write(tmp.path().join("b.rs"), b"fn main(){}").unwrap();

        let config = ScanConfig {
            directories: vec![tmp.path().to_path_buf()],
            detect_duplicates: false,
            max_depth: None,
        };

        let (tx, mut rx) = mpsc::channel(64);
        let cancel = tokio_util::sync::CancellationToken::new();

        FileClassificationProbe::scan(&config, Some(tx), cancel).await.unwrap();

        let mut count = 0u64;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn scan_respects_cancellation() {
        let tmp = TempDir::new().unwrap();
        for i in 0..100 {
            fs::write(tmp.path().join(format!("file{i}.txt")), b"data").unwrap();
        }

        let config = ScanConfig {
            directories: vec![tmp.path().to_path_buf()],
            detect_duplicates: false,
            max_depth: None,
        };

        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();

        let result = FileClassificationProbe::scan(&config, None, cancel).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn scan_rejects_nonexistent_directory() {
        let config = ScanConfig {
            directories: vec![PathBuf::from("/nonexistent/path/unlikely")],
            detect_duplicates: false,
            max_depth: None,
        };

        let cancel = tokio_util::sync::CancellationToken::new();
        let result = FileClassificationProbe::scan(&config, None, cancel).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn scan_respects_max_depth() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        fs::write(tmp.path().join("root.txt"), b"root").unwrap();
        fs::write(tmp.path().join("a/level1.txt"), b"l1").unwrap();
        fs::write(tmp.path().join("a/b/level2.txt"), b"l2").unwrap();
        fs::write(tmp.path().join("a/b/c/level3.txt"), b"l3").unwrap();

        let config = ScanConfig {
            directories: vec![tmp.path().to_path_buf()],
            detect_duplicates: false,
            max_depth: Some(1),
        };

        let cancel = tokio_util::sync::CancellationToken::new();
        let result = FileClassificationProbe::scan(&config, None, cancel).await.unwrap();

        assert_eq!(result.files.len(), 2);
    }

    #[test]
    fn hash_file_produces_consistent_sha256() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.bin");
        {
            let mut f = fs::File::create(&path).unwrap();
            f.write_all(b"hello world").unwrap();
        }

        let h1 = hash_file(&path).unwrap();
        let h2 = hash_file(&path).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}
