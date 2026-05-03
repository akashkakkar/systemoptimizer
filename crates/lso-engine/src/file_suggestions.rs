//! Generates organization recommendations from file classification scan results.
//!
//! All suggestions are `Pending` — they must pass through the approval gate
//! before any action is taken.

use chrono::Utc;
use lso_core::{
    ClassificationResult, FileCategory, FileInfo, Recommendation, RecommendationStatus, RiskLevel,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

/// Configurable thresholds loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct FileThresholds {
    pub large_file_bytes: u64,
    pub archive_after_days: u64,
    pub scattered_group_min: usize,
    pub scattered_dir_min: usize,
}

impl Default for FileThresholds {
    fn default() -> Self {
        Self {
            large_file_bytes: 100 * 1024 * 1024,
            archive_after_days: 365,
            scattered_group_min: 3,
            scattered_dir_min: 3,
        }
    }
}

/// Analyzes file classification results and produces recommendations.
pub struct FileSuggestionGenerator {
    thresholds: FileThresholds,
}

impl FileSuggestionGenerator {
    pub fn new(thresholds: FileThresholds) -> Self {
        Self { thresholds }
    }

    pub fn with_defaults() -> Self {
        Self {
            thresholds: FileThresholds::default(),
        }
    }

    /// Analyze scan results and produce all applicable recommendations.
    pub fn analyze(&self, result: &ClassificationResult) -> Vec<Recommendation> {
        let mut recs = Vec::new();

        recs.extend(self.find_large_unused_files(&result.files));
        recs.extend(self.find_archive_candidates(&result.files));
        recs.extend(self.find_scattered_groups(&result.files));
        recs.extend(self.find_duplicate_recommendations(result));

        info!(recommendations = recs.len(), "file suggestion analysis complete");
        recs
    }

    fn find_large_unused_files(&self, files: &[FileInfo]) -> Vec<Recommendation> {
        let now = Utc::now();
        let threshold_days = self.thresholds.archive_after_days as i64;

        files
            .iter()
            .filter(|f| {
                f.size_bytes >= self.thresholds.large_file_bytes
                    && f.accessed
                        .map(|a| (now - a).num_days() > threshold_days)
                        .unwrap_or(true)
            })
            .map(|f| {
                let mb = f.size_bytes as f64 / (1024.0 * 1024.0);
                Recommendation {
                    id: Uuid::new_v4(),
                    rule_id: "file.large_unused".into(),
                    title: format!("Large unused file ({mb:.1} MB)"),
                    description: format!(
                        "{} file, {mb:.1} MB, not accessed in over {threshold_days} days",
                        f.category,
                    ),
                    risk_level: RiskLevel::Low,
                    category: "file_organization".into(),
                    target: f.path.display().to_string(),
                    rollback_plan: None,
                    status: RecommendationStatus::Pending,
                    rejection_reason: None,
                    created_at: now,
                    resolved_at: None,
                }
            })
            .collect()
    }

    fn find_archive_candidates(&self, files: &[FileInfo]) -> Vec<Recommendation> {
        let now = Utc::now();
        let threshold_days = self.thresholds.archive_after_days as i64;

        let candidates: Vec<&FileInfo> = files
            .iter()
            .filter(|f| {
                f.size_bytes < self.thresholds.large_file_bytes
                    && f.category != FileCategory::Temporary
                    && f.modified
                        .map(|m| (now - m).num_days() > threshold_days)
                        .unwrap_or(false)
            })
            .collect();

        if candidates.is_empty() {
            return Vec::new();
        }

        let total_bytes: u64 = candidates.iter().map(|f| f.size_bytes).sum();
        let mb = total_bytes as f64 / (1024.0 * 1024.0);
        let target = candidates
            .first()
            .and_then(|f| f.path.parent())
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        vec![Recommendation {
            id: Uuid::new_v4(),
            rule_id: "file.archive_candidates".into(),
            title: format!(
                "{} files untouched for over {threshold_days} days",
                candidates.len(),
            ),
            description: format!(
                "{} files ({mb:.1} MB total) have not been modified in over {threshold_days} days and may be candidates for archiving",
                candidates.len(),
            ),
            risk_level: RiskLevel::Low,
            category: "file_organization".into(),
            target,
            rollback_plan: None,
            status: RecommendationStatus::Pending,
            rejection_reason: None,
            created_at: now,
            resolved_at: None,
        }]
    }

    fn find_scattered_groups(&self, files: &[FileInfo]) -> Vec<Recommendation> {
        let now = Utc::now();
        let mut by_category_ext: HashMap<(FileCategory, Option<&str>), Vec<&FileInfo>> =
            HashMap::new();

        for f in files {
            let key = (f.category, f.extension.as_deref());
            by_category_ext.entry(key).or_default().push(f);
        }

        by_category_ext
            .into_iter()
            .filter_map(|((cat, ext), group)| {
                if group.len() < self.thresholds.scattered_group_min {
                    return None;
                }

                let unique_dirs: HashSet<&std::path::Path> = group
                    .iter()
                    .filter_map(|f| f.path.parent())
                    .collect();

                if unique_dirs.len() < self.thresholds.scattered_dir_min {
                    return None;
                }

                let paths: Vec<PathBuf> = group.iter().map(|f| f.path.clone()).collect();
                let ext_label = ext.unwrap_or("no extension");

                Some(Recommendation {
                    id: Uuid::new_v4(),
                    rule_id: "file.scattered_group".into(),
                    title: format!(
                        "{} {} (.{}) files across {} directories",
                        group.len(),
                        cat,
                        ext_label,
                        unique_dirs.len(),
                    ),
                    description: format!(
                        "Found {} {} files (.{}) scattered across {} directories — consider grouping them",
                        group.len(),
                        cat,
                        ext_label,
                        unique_dirs.len(),
                    ),
                    risk_level: RiskLevel::Low,
                    category: "file_organization".into(),
                    target: paths.first().map(|p| p.display().to_string()).unwrap_or_default(),
                    rollback_plan: None,
                    status: RecommendationStatus::Pending,
                    rejection_reason: None,
                    created_at: now,
                    resolved_at: None,
                })
            })
            .collect()
    }

    fn find_duplicate_recommendations(
        &self,
        result: &ClassificationResult,
    ) -> Vec<Recommendation> {
        let now = Utc::now();

        result
            .duplicate_clusters
            .iter()
            .map(|cluster| {
                let mb = cluster.wasted_bytes as f64 / (1024.0 * 1024.0);
                let hash_prefix = &cluster.hash[..12.min(cluster.hash.len())];

                Recommendation {
                    id: Uuid::new_v4(),
                    rule_id: "file.duplicate_cluster".into(),
                    title: format!(
                        "{} duplicate files ({mb:.1} MB wasted)",
                        cluster.files.len(),
                    ),
                    description: format!(
                        "{} files share identical content (SHA-256: {hash_prefix}…), wasting {mb:.1} MB",
                        cluster.files.len(),
                    ),
                    risk_level: RiskLevel::Medium,
                    category: "file_organization".into(),
                    target: cluster
                        .files
                        .first()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    rollback_plan: Some("Restore from snapshot before deduplication".into()),
                    status: RecommendationStatus::Pending,
                    rejection_reason: None,
                    created_at: now,
                    resolved_at: None,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use lso_core::DuplicateCluster;

    fn make_file(
        path: &str,
        category: FileCategory,
        ext: Option<&str>,
        size: u64,
        modified_days_ago: i64,
        accessed_days_ago: i64,
    ) -> FileInfo {
        let now = Utc::now();
        FileInfo {
            path: PathBuf::from(path),
            category,
            extension: ext.map(String::from),
            size_bytes: size,
            created: None,
            modified: Some(now - Duration::days(modified_days_ago)),
            accessed: Some(now - Duration::days(accessed_days_ago)),
            content_hash: None,
        }
    }

    fn make_result(files: Vec<FileInfo>, duplicates: Vec<DuplicateCluster>) -> ClassificationResult {
        ClassificationResult {
            scan_id: Uuid::new_v4(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            category_summary: HashMap::new(),
            files,
            duplicate_clusters: duplicates,
        }
    }

    #[test]
    fn detects_large_unused_files() {
        let gen = FileSuggestionGenerator::with_defaults();
        let files = vec![
            make_file("/big.iso", FileCategory::Archives, Some("iso"), 200_000_000, 500, 500),
            make_file("/small.txt", FileCategory::Documents, Some("txt"), 100, 500, 500),
        ];

        let result = make_result(files, vec![]);
        let recs = gen.analyze(&result);

        assert!(recs.iter().any(|r| r.rule_id == "file.large_unused"));
        assert!(recs.iter().all(|r| r.status == RecommendationStatus::Pending));
    }

    #[test]
    fn detects_archive_candidates() {
        let gen = FileSuggestionGenerator::with_defaults();
        let files = vec![
            make_file("/old1.txt", FileCategory::Documents, Some("txt"), 1000, 400, 400),
            make_file("/old2.txt", FileCategory::Documents, Some("txt"), 2000, 500, 500),
            make_file("/recent.txt", FileCategory::Documents, Some("txt"), 1000, 10, 10),
        ];

        let result = make_result(files, vec![]);
        let recs = gen.analyze(&result);

        let archive_recs: Vec<_> = recs
            .iter()
            .filter(|r| r.rule_id == "file.archive_candidates")
            .collect();

        assert_eq!(archive_recs.len(), 1);
    }

    #[test]
    fn detects_scattered_files() {
        let gen = FileSuggestionGenerator::with_defaults();
        let files = vec![
            make_file("/a/dir1/f.pdf", FileCategory::Documents, Some("pdf"), 100, 10, 10),
            make_file("/b/dir2/g.pdf", FileCategory::Documents, Some("pdf"), 200, 10, 10),
            make_file("/c/dir3/h.pdf", FileCategory::Documents, Some("pdf"), 300, 10, 10),
        ];

        let result = make_result(files, vec![]);
        let recs = gen.analyze(&result);

        assert!(recs.iter().any(|r| r.rule_id == "file.scattered_group"));
    }

    #[test]
    fn converts_duplicates_to_recommendations() {
        let gen = FileSuggestionGenerator::with_defaults();
        let cluster = DuplicateCluster {
            hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".into(),
            files: vec![PathBuf::from("/a.txt"), PathBuf::from("/b.txt")],
            file_size: 5000,
            wasted_bytes: 5000,
        };

        let result = make_result(vec![], vec![cluster]);
        let recs = gen.analyze(&result);

        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].rule_id, "file.duplicate_cluster");
        assert_eq!(recs[0].risk_level, RiskLevel::Medium);
    }

    #[test]
    fn all_recommendations_are_pending() {
        let gen = FileSuggestionGenerator::with_defaults();
        let files = vec![
            make_file("/big.iso", FileCategory::Archives, Some("iso"), 200_000_000, 500, 500),
        ];

        let result = make_result(files, vec![]);
        let recs = gen.analyze(&result);

        for rec in &recs {
            assert_eq!(rec.status, RecommendationStatus::Pending);
        }
    }

    #[test]
    fn recommendations_have_rule_ids() {
        let gen = FileSuggestionGenerator::with_defaults();
        let files = vec![
            make_file("/big.iso", FileCategory::Archives, Some("iso"), 200_000_000, 500, 500),
        ];

        let result = make_result(files, vec![]);
        let recs = gen.analyze(&result);

        for rec in &recs {
            assert!(rec.rule_id.starts_with("file."));
            assert_eq!(rec.category, "file_organization");
        }
    }
}
