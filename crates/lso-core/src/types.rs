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

/// Lifecycle status of a recommendation, tracked by the DB layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationStatus {
    Pending,
    Accepted,
    Dismissed,
    Applied,
}

impl RecommendationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Dismissed => "dismissed",
            Self::Applied => "applied",
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
            "accepted" => Ok(Self::Accepted),
            "dismissed" => Ok(Self::Dismissed),
            "applied" => Ok(Self::Applied),
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
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub category: String,
    pub target: String,
    pub rollback_plan: Option<String>,
    pub status: RecommendationStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
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
    pub risk_level: RiskLevel,
    pub user_approved: bool,
    pub snapshot_id: Option<String>,
    pub result: ActionResult,
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
        for s in ["pending", "accepted", "dismissed", "applied"] {
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
            title: "Remove cache".into(),
            description: "Clear stale build cache".into(),
            risk_level: RiskLevel::Low,
            category: "cleanup".into(),
            target: "/tmp/build-cache".into(),
            rollback_plan: Some("Restore from snapshot".into()),
            status: RecommendationStatus::Pending,
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
            risk_level: RiskLevel::Medium,
            user_approved: true,
            snapshot_id: Some("snap-001".into()),
            result: ActionResult::Success,
            rollback_available: true,
        };
        let rt = roundtrip(&entry);
        assert_eq!(entry.action, rt.action);
        assert_eq!(entry.risk_level, rt.risk_level);
        assert_eq!(entry.user_approved, rt.user_approved);
        assert_eq!(entry.snapshot_id, rt.snapshot_id);
        assert_eq!(entry.result, rt.result);
    }

    #[test]
    fn display_impls() {
        assert_eq!(RiskLevel::High.to_string(), "High");
        assert_eq!(Platform::MacOS.to_string(), "macOS");
        assert_eq!(PrivilegeLevel::Root.to_string(), "Root");
        assert_eq!(ApprovalStatus::Expired.to_string(), "Expired");
    }
}
