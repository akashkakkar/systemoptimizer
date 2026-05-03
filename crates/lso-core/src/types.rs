/// Core domain types shared across all LSO crates.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// Elevate risk by one level (system drive modifier).
    pub fn elevate(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Critical,
            Self::Critical => Self::Critical,
        }
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => f.write_str("Low"),
            Self::Medium => f.write_str("Medium"),
            Self::High => f.write_str("High"),
            Self::Critical => f.write_str("Critical"),
        }
    }
}

impl std::str::FromStr for RiskLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            other => Err(format!("unknown risk level: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    #[serde(rename = "macos")]
    MacOS,
    Linux,
    Windows,
}

impl Platform {
    /// Detect the current platform from `std::env::consts::OS`.
    pub fn detect() -> Option<Self> {
        match std::env::consts::OS {
            "macos" => Some(Self::MacOS),
            "linux" => Some(Self::Linux),
            "windows" => Some(Self::Windows),
            _ => None,
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MacOS => f.write_str("macOS"),
            Self::Linux => f.write_str("Linux"),
            Self::Windows => f.write_str("Windows"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrivilegeLevel {
    Unprivileged,
    Elevated,
    Root,
}

impl std::fmt::Display for PrivilegeLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unprivileged => f.write_str("Unprivileged"),
            Self::Elevated => f.write_str("Elevated"),
            Self::Root => f.write_str("Root"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.write_str("Pending"),
            Self::Approved => f.write_str("Approved"),
            Self::Rejected => f.write_str("Rejected"),
            Self::Expired => f.write_str("Expired"),
        }
    }
}

/// Full lifecycle status of a recommendation.
///
/// ```text
/// Pending → [Approved | Rejected | Expired]
///                ↓
///            Executing → [Succeeded | Failed]
///                              ↓ (if failed)
///                          RolledBack
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Executing,
    Succeeded,
    Failed,
    RolledBack,
}

impl RecommendationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Executing => "executing",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
        }
    }
}

impl std::fmt::Display for RecommendationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RecommendationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "expired" => Ok(Self::Expired),
            "executing" => Ok(Self::Executing),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "rolled_back" => Ok(Self::RolledBack),
            other => Err(format!("unknown recommendation status: {other}")),
        }
    }
}

/// Result of an executed actuator action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionResult {
    Success,
    Failed,
    RolledBack,
}

impl ActionResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
        }
    }
}

impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ActionResult {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "rolled_back" => Ok(Self::RolledBack),
            other => Err(format!("unknown action result: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: Uuid,
    pub rule_id: String,
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub category: String,
    pub target: String,
    pub rollback_plan: Option<String>,
    pub status: RecommendationStatus,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Recommendation {
    /// Deduplication key: same rule + same target = same recommendation.
    pub fn dedup_key(&self) -> String {
        format!("{}::{}", self.rule_id, self.target)
    }

    /// Transition to Approved state. Returns false if transition is invalid.
    pub fn approve(&mut self) -> bool {
        if self.status != RecommendationStatus::Pending {
            return false;
        }
        if self.risk_level == RiskLevel::Critical {
            return false;
        }
        self.status = RecommendationStatus::Approved;
        self.resolved_at = Some(Utc::now());
        true
    }

    /// Transition to Rejected state with optional reason.
    pub fn reject(&mut self, reason: Option<String>) -> bool {
        if self.status != RecommendationStatus::Pending {
            return false;
        }
        self.status = RecommendationStatus::Rejected;
        self.rejection_reason = reason;
        self.resolved_at = Some(Utc::now());
        true
    }

    /// Transition to Expired state (condition resolved).
    pub fn expire(&mut self) -> bool {
        if self.status != RecommendationStatus::Pending {
            return false;
        }
        self.status = RecommendationStatus::Expired;
        self.resolved_at = Some(Utc::now());
        true
    }
}

/// Result of an approval action returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub id: Uuid,
    pub status: RecommendationStatus,
    pub requires_double_confirm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetric {
    pub id: Uuid,
    pub probe_id: String,
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
    pub collected_at: DateTime<Utc>,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub probe_id: String,
    pub metrics: Vec<SystemMetric>,
    pub collected_at: DateTime<Utc>,
    pub platform: Platform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub target: String,
    pub category: String,
    pub risk_level: RiskLevel,
    pub user_approved: bool,
    pub snapshot_id: Option<String>,
    pub result: ActionResult,
    pub details: Option<String>,
    pub rollback_available: bool,
}

/// IPC wire type — disk usage for a single mount point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsageReport {
    pub mount_point: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f64,
}

/// Describes a category of temporary files to clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupTarget {
    SystemTemp,
    UserCache,
    AppLogs,
}

/// A file that was skipped during cleanup (safety rules).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: String,
}

/// Pre-execution report showing what a cleanup would do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    pub target: CleanupTarget,
    pub file_count: u64,
    pub total_bytes: u64,
    pub oldest_modified: Option<DateTime<Utc>>,
    pub newest_modified: Option<DateTime<Utc>>,
    pub skipped: Vec<SkippedFile>,
}

/// Progress update emitted during cleanup execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupProgress {
    pub processed: u64,
    pub total: u64,
    pub current_file: String,
    pub bytes_so_far: u64,
}

/// Summary returned after cleanup completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub files_deleted: u64,
    pub bytes_reclaimed: u64,
    pub errors: Vec<String>,
    pub skipped: Vec<SkippedFile>,
    pub snapshot_id: String,
    pub duration_ms: u64,
}

/// Filter for querying the audit log.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    pub status: Option<ActionResult>,
    pub risk_level: Option<RiskLevel>,
    pub category: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub search: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

/// Paginated audit log response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPage {
    pub entries: Vec<AuditEntry>,
    pub total_count: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

/// Export format for audit log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
}

// ---------------------------------------------------------------------------
// File classification (F18)
// ---------------------------------------------------------------------------

/// High-level file category for classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    Documents,
    Media,
    Code,
    Archives,
    Data,
    Temporary,
    Unknown,
}

impl std::fmt::Display for FileCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Documents => write!(f, "Documents"),
            Self::Media => write!(f, "Media"),
            Self::Code => write!(f, "Code"),
            Self::Archives => write!(f, "Archives"),
            Self::Data => write!(f, "Data"),
            Self::Temporary => write!(f, "Temporary"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Metadata about a single file (never reads content for classification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: std::path::PathBuf,
    pub category: FileCategory,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub accessed: Option<DateTime<Utc>>,
    /// SHA-256 hex digest — only populated when duplicate detection is opted-in.
    pub content_hash: Option<String>,
}

/// Configuration for a file classification scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Directories the user has explicitly approved for scanning.
    pub directories: Vec<std::path::PathBuf>,
    /// Whether to compute SHA-256 hashes for duplicate detection.
    pub detect_duplicates: bool,
    /// Maximum directory depth (None = unlimited).
    pub max_depth: Option<usize>,
}

/// Progress update emitted during a file classification scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub files_scanned: u64,
    pub files_total_estimate: Option<u64>,
    pub current_directory: std::path::PathBuf,
    pub bytes_scanned: u64,
}

/// Complete result of a file classification scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub scan_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub files: Vec<FileInfo>,
    pub category_summary: std::collections::HashMap<FileCategory, CategoryStats>,
    pub duplicate_clusters: Vec<DuplicateCluster>,
}

/// Aggregate stats for one file category.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryStats {
    pub count: u64,
    pub total_bytes: u64,
}

/// A group of files sharing the same content hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCluster {
    pub hash: String,
    pub files: Vec<std::path::PathBuf>,
    pub file_size: u64,
    pub wasted_bytes: u64,
}

/// Sanitized file summary safe for inclusion in AI prompts (no full paths).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedFileSummary {
    pub category: FileCategory,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub age_days: u64,
}

impl SanitizedFileSummary {
    /// Build from a `FileInfo`, stripping the full path.
    pub fn from_file_info(info: &FileInfo, now: DateTime<Utc>) -> Self {
        let age_days = info
            .modified
            .map(|m| (now - m).num_days().unsigned_abs())
            .unwrap_or(0);
        Self {
            category: info.category,
            extension: info.extension.clone(),
            size_bytes: info.size_bytes,
            age_days,
        }
    }
}

// ---------------------------------------------------------------------------
// Startup items (F19)
// ---------------------------------------------------------------------------

/// A single startup/login item discovered by a probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupItem {
    pub name: String,
    pub item_type: StartupType,
    pub command: String,
    pub enabled: bool,
    pub publisher: Option<String>,
    pub impact: StartupImpact,
    /// Platform-specific identifier used to disable/enable the item.
    pub platform_id: String,
}

/// How the item is registered to start at boot/login.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupType {
    SystemdService,
    DesktopAutostart,
    LaunchAgent,
    LaunchDaemon,
    RegistryRun,
    ScheduledTask,
}

impl std::fmt::Display for StartupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SystemdService => f.write_str("systemd service"),
            Self::DesktopAutostart => f.write_str("desktop autostart"),
            Self::LaunchAgent => f.write_str("launch agent"),
            Self::LaunchDaemon => f.write_str("launch daemon"),
            Self::RegistryRun => f.write_str("registry run key"),
            Self::ScheduledTask => f.write_str("scheduled task"),
        }
    }
}

/// Estimated boot-time impact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StartupImpact {
    Low,
    Medium,
    High,
    Unknown,
}

// ---------------------------------------------------------------------------
// Security posture (F20)
// ---------------------------------------------------------------------------

/// A listening network port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPort {
    pub port: u16,
    pub protocol: NetProtocol,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub bind_address: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetProtocol {
    Tcp,
    Tcp6,
    Udp,
    Udp6,
}

impl std::fmt::Display for NetProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp => f.write_str("tcp"),
            Self::Tcp6 => f.write_str("tcp6"),
            Self::Udp => f.write_str("udp"),
            Self::Udp6 => f.write_str("udp6"),
        }
    }
}

/// A file/directory permission issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionIssue {
    pub path: std::path::PathBuf,
    pub issue_type: PermissionIssueType,
    pub current_mode: u32,
    pub recommended_mode: Option<u32>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionIssueType {
    WorldWritable,
    SuidBinary,
    SgidBinary,
    OverlyPermissiveHome,
    InsecureSshKey,
}

impl std::fmt::Display for PermissionIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorldWritable => f.write_str("world-writable"),
            Self::SuidBinary => f.write_str("SUID binary"),
            Self::SgidBinary => f.write_str("SGID binary"),
            Self::OverlyPermissiveHome => f.write_str("overly permissive home"),
            Self::InsecureSshKey => f.write_str("insecure SSH key"),
        }
    }
}

/// Firewall status report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallStatus {
    pub enabled: bool,
    pub backend: FirewallBackend,
    pub default_policy: String,
    pub rule_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FirewallBackend {
    Iptables,
    Nftables,
    Ufw,
    Pf,
    WindowsFirewall,
    Unknown,
}

impl std::fmt::Display for FirewallBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Iptables => f.write_str("iptables"),
            Self::Nftables => f.write_str("nftables"),
            Self::Ufw => f.write_str("ufw"),
            Self::Pf => f.write_str("pf"),
            Self::WindowsFirewall => f.write_str("Windows Firewall"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

/// Aggregated result from all security probes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResult {
    pub open_ports: Vec<OpenPort>,
    pub permission_issues: Vec<PermissionIssue>,
    pub firewall: FirewallStatus,
    /// 0–100, higher is better.
    pub score: u8,
}

/// Compute a security score from 0 (terrible) to 100 (clean).
///
/// Deductions:
/// - Firewall disabled: −30
/// - Each unexpected open port: −5 (max −25)
/// - Each non-SSH permission issue: −5 (max −25)
/// - Each insecure SSH key: −10 (max −20)
pub fn compute_security_score(
    firewall: &FirewallStatus,
    open_ports: &[OpenPort],
    permission_issues: &[PermissionIssue],
) -> u8 {
    let mut score: i32 = 100;

    if !firewall.enabled {
        score -= 30;
    }

    let port_penalty = (open_ports.len() as i32 * 5).min(25);
    score -= port_penalty;

    let ssh_issues = permission_issues
        .iter()
        .filter(|p| p.issue_type == PermissionIssueType::InsecureSshKey)
        .count();
    let other_issues = permission_issues.len() - ssh_issues;

    score -= (ssh_issues as i32 * 10).min(20);
    score -= (other_issues as i32 * 5).min(25);

    score.clamp(0, 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn roundtrip<T: serde::Serialize + for<'de> serde::Deserialize<'de>>(value: &T) -> T {
        let json = serde_json::to_string(value).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn risk_level_serde_roundtrip() {
        for level in [RiskLevel::Low, RiskLevel::Medium, RiskLevel::High, RiskLevel::Critical] {
            let rt = roundtrip(&level);
            assert_eq!(level, rt);
        }
    }

    #[test]
    fn risk_level_serializes_lowercase() {
        let json = serde_json::to_string(&RiskLevel::Critical).expect("serialize");
        assert_eq!(json, "\"critical\"");
    }

    #[test]
    fn risk_level_fromstr_roundtrip() {
        for s in ["low", "medium", "high", "critical"] {
            let level: RiskLevel = s.parse().unwrap();
            assert_eq!(level.as_str(), s);
        }
    }

    #[test]
    fn platform_serde_roundtrip() {
        for p in [Platform::MacOS, Platform::Linux, Platform::Windows] {
            let rt = roundtrip(&p);
            assert_eq!(p, rt);
        }
    }

    #[test]
    fn platform_macos_serializes_correctly() {
        let json = serde_json::to_string(&Platform::MacOS).expect("serialize");
        assert_eq!(json, "\"macos\"");
    }

    #[test]
    fn platform_detect_returns_some() {
        let platform = Platform::detect();
        assert!(platform.is_some(), "should detect a known platform in CI/dev");
    }

    #[test]
    fn privilege_level_serde_roundtrip() {
        for lvl in [PrivilegeLevel::Unprivileged, PrivilegeLevel::Elevated, PrivilegeLevel::Root] {
            let rt = roundtrip(&lvl);
            assert_eq!(lvl, rt);
        }
    }

    #[test]
    fn approval_status_serde_roundtrip() {
        for status in [
            ApprovalStatus::Pending,
            ApprovalStatus::Approved,
            ApprovalStatus::Rejected,
            ApprovalStatus::Expired,
        ] {
            let rt = roundtrip(&status);
            assert_eq!(status, rt);
        }
    }

    #[test]
    fn recommendation_status_roundtrip() {
        for s in [
            "pending", "approved", "rejected", "expired", "executing",
            "succeeded", "failed", "rolled_back",
        ] {
            let status: RecommendationStatus = s.parse().unwrap();
            assert_eq!(status.as_str(), s);
        }
    }

    #[test]
    fn action_result_roundtrip() {
        for s in ["success", "failed", "rolled_back"] {
            let result: ActionResult = s.parse().unwrap();
            assert_eq!(result.as_str(), s);
        }
    }

    #[test]
    fn recommendation_serde_roundtrip() {
        let rec = Recommendation {
            id: Uuid::new_v4(),
            rule_id: "disk.cleanup".into(),
            title: "Remove cache".into(),
            description: "Clear stale build cache".into(),
            risk_level: RiskLevel::Low,
            category: "cleanup".into(),
            target: "/tmp/build-cache".into(),
            rollback_plan: Some("Restore from snapshot".into()),
            status: RecommendationStatus::Pending,
            rejection_reason: None,
            created_at: Utc::now(),
            resolved_at: None,
        };
        let rt = roundtrip(&rec);
        assert_eq!(rec.id, rt.id);
        assert_eq!(rec.title, rt.title);
        assert_eq!(rec.risk_level, rt.risk_level);
        assert_eq!(rec.status, rt.status);
    }

    #[test]
    fn system_metric_serde_roundtrip() {
        let metric = SystemMetric {
            id: Uuid::new_v4(),
            probe_id: "disk_usage".into(),
            name: "disk_free".into(),
            value: 42.5,
            unit: Some("GB".into()),
            collected_at: Utc::now(),
            platform: "macos".into(),
        };
        let rt = roundtrip(&metric);
        assert_eq!(metric.id, rt.id);
        assert_eq!(metric.name, rt.name);
        assert!((metric.value - rt.value).abs() < f64::EPSILON);
    }

    #[test]
    fn probe_result_serde_roundtrip() {
        let result = ProbeResult {
            probe_id: "disk_usage".into(),
            metrics: vec![SystemMetric {
                id: Uuid::new_v4(),
                probe_id: "disk_usage".into(),
                name: "disk_free".into(),
                value: 100.0,
                unit: Some("GB".into()),
                collected_at: Utc::now(),
                platform: "macos".into(),
            }],
            collected_at: Utc::now(),
            platform: Platform::MacOS,
        };
        let rt = roundtrip(&result);
        assert_eq!(result.probe_id, rt.probe_id);
        assert_eq!(result.metrics.len(), rt.metrics.len());
        assert_eq!(result.platform, rt.platform);
    }

    #[test]
    fn audit_entry_serde_roundtrip() {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: "delete_cache".into(),
            target: "/tmp/cache".into(),
            category: "cleanup".into(),
            risk_level: RiskLevel::Medium,
            user_approved: true,
            snapshot_id: Some("snap-001".into()),
            result: ActionResult::Success,
            details: Some("Deleted 42 files, 1.2 GB reclaimed".into()),
            rollback_available: true,
        };
        let rt = roundtrip(&entry);
        assert_eq!(entry.action, rt.action);
        assert_eq!(entry.category, rt.category);
        assert_eq!(entry.risk_level, rt.risk_level);
        assert_eq!(entry.user_approved, rt.user_approved);
        assert_eq!(entry.snapshot_id, rt.snapshot_id);
        assert_eq!(entry.result, rt.result);
        assert_eq!(entry.details, rt.details);
    }

    #[test]
    fn preflight_report_serde_roundtrip() {
        let report = PreflightReport {
            target: CleanupTarget::SystemTemp,
            file_count: 100,
            total_bytes: 1024 * 1024,
            oldest_modified: Some(Utc::now()),
            newest_modified: Some(Utc::now()),
            skipped: vec![SkippedFile {
                path: "/tmp/in_use.log".into(),
                reason: "file in use".into(),
            }],
        };
        let rt = roundtrip(&report);
        assert_eq!(rt.file_count, 100);
        assert_eq!(rt.skipped.len(), 1);
    }

    #[test]
    fn audit_filter_defaults() {
        let filter = AuditFilter::default();
        assert!(filter.status.is_none());
        assert!(filter.page.is_none());
    }

    #[test]
    fn display_impls() {
        assert_eq!(RiskLevel::High.to_string(), "High");
        assert_eq!(Platform::MacOS.to_string(), "macOS");
        assert_eq!(PrivilegeLevel::Root.to_string(), "Root");
        assert_eq!(ApprovalStatus::Expired.to_string(), "Expired");
    }

    #[test]
    fn file_category_display() {
        assert_eq!(FileCategory::Documents.to_string(), "Documents");
        assert_eq!(FileCategory::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn file_category_serde_roundtrip() {
        let cat = FileCategory::Media;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"media\"");
        let back: FileCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cat);
    }

    #[test]
    fn sanitized_summary_strips_path() {
        let info = FileInfo {
            path: std::path::PathBuf::from("/Users/secret/Documents/report.pdf"),
            category: FileCategory::Documents,
            extension: Some("pdf".into()),
            size_bytes: 1024,
            created: None,
            modified: Some(Utc::now() - chrono::Duration::days(30)),
            accessed: None,
            content_hash: None,
        };
        let summary = SanitizedFileSummary::from_file_info(&info, Utc::now());
        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("secret"));
        assert!(!json.contains("/Users"));
        assert!(summary.age_days >= 29 && summary.age_days <= 31);
    }

    #[test]
    fn startup_item_serde_roundtrip() {
        let item = StartupItem {
            name: "docker".into(),
            item_type: StartupType::SystemdService,
            command: "/usr/bin/dockerd".into(),
            enabled: true,
            publisher: Some("Docker Inc".into()),
            impact: StartupImpact::Medium,
            platform_id: "docker.service".into(),
        };
        let rt = roundtrip(&item);
        assert_eq!(rt.name, "docker");
        assert_eq!(rt.item_type, StartupType::SystemdService);
    }

    #[test]
    fn open_port_serde_roundtrip() {
        let port = OpenPort {
            port: 8080,
            protocol: NetProtocol::Tcp,
            pid: Some(1234),
            process_name: Some("nginx".into()),
            bind_address: "0.0.0.0".into(),
        };
        let rt = roundtrip(&port);
        assert_eq!(rt.port, 8080);
        assert_eq!(rt.process_name.as_deref(), Some("nginx"));
    }

    #[test]
    fn security_score_perfect() {
        let fw = FirewallStatus {
            enabled: true,
            backend: FirewallBackend::Ufw,
            default_policy: "deny".into(),
            rule_count: 3,
        };
        assert_eq!(compute_security_score(&fw, &[], &[]), 100);
    }

    #[test]
    fn security_score_firewall_disabled() {
        let fw = FirewallStatus {
            enabled: false,
            backend: FirewallBackend::Unknown,
            default_policy: "none".into(),
            rule_count: 0,
        };
        assert_eq!(compute_security_score(&fw, &[], &[]), 70);
    }

    #[test]
    fn security_score_clamps_to_zero() {
        let fw = FirewallStatus {
            enabled: false,
            backend: FirewallBackend::Unknown,
            default_policy: "none".into(),
            rule_count: 0,
        };
        let ports: Vec<OpenPort> = (0..20)
            .map(|i| OpenPort {
                port: 8000 + i,
                protocol: NetProtocol::Tcp,
                pid: None,
                process_name: None,
                bind_address: "0.0.0.0".into(),
            })
            .collect();
        let mut perms: Vec<PermissionIssue> = (0..20)
            .map(|i| PermissionIssue {
                path: format!("/tmp/bad{i}").into(),
                issue_type: PermissionIssueType::WorldWritable,
                current_mode: 0o777,
                recommended_mode: Some(0o755),
                description: "world-writable".into(),
            })
            .collect();
        for i in 0..5 {
            perms.push(PermissionIssue {
                path: format!("/home/user/.ssh/id_rsa_{i}").into(),
                issue_type: PermissionIssueType::InsecureSshKey,
                current_mode: 0o644,
                recommended_mode: Some(0o600),
                description: "insecure SSH key".into(),
            });
        }
        assert_eq!(compute_security_score(&fw, &ports, &perms), 0);
    }
}
