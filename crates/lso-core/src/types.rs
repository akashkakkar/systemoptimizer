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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub category: String,
    pub target: String,
    pub rollback_plan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
    pub source_probe: String,
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
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub target: String,
    pub risk_level: RiskLevel,
    pub user_approved: bool,
    pub snapshot_id: Option<Uuid>,
    pub result: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn roundtrip<T: Serialize + for<'de> Deserialize<'de>>(value: &T) -> T {
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
    fn recommendation_serde_roundtrip() {
        let rec = Recommendation {
            id: Uuid::new_v4(),
            title: "Remove cache".into(),
            description: "Clear stale build cache".into(),
            risk_level: RiskLevel::Low,
            category: "cleanup".into(),
            target: "/tmp/build-cache".into(),
            rollback_plan: Some("Restore from snapshot".into()),
        };
        let rt = roundtrip(&rec);
        assert_eq!(rec.id, rt.id);
        assert_eq!(rec.title, rt.title);
        assert_eq!(rec.risk_level, rt.risk_level);
    }

    #[test]
    fn system_metric_serde_roundtrip() {
        let metric = SystemMetric {
            name: "disk_free".into(),
            value: 42.5,
            unit: "GB".into(),
            timestamp: Utc::now(),
            source_probe: "disk_usage".into(),
        };
        let rt = roundtrip(&metric);
        assert_eq!(metric.name, rt.name);
        assert!((metric.value - rt.value).abs() < f64::EPSILON);
    }

    #[test]
    fn probe_result_serde_roundtrip() {
        let result = ProbeResult {
            probe_id: "disk_usage".into(),
            metrics: vec![SystemMetric {
                name: "disk_free".into(),
                value: 100.0,
                unit: "GB".into(),
                timestamp: Utc::now(),
                source_probe: "disk_usage".into(),
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
            timestamp: Utc::now(),
            action: "delete_cache".into(),
            target: "/tmp/cache".into(),
            risk_level: RiskLevel::Medium,
            user_approved: true,
            snapshot_id: Some(Uuid::new_v4()),
            result: "success".into(),
        };
        let rt = roundtrip(&entry);
        assert_eq!(entry.action, rt.action);
        assert_eq!(entry.risk_level, rt.risk_level);
        assert_eq!(entry.user_approved, rt.user_approved);
        assert_eq!(entry.snapshot_id, rt.snapshot_id);
    }

    #[test]
    fn display_impls() {
        assert_eq!(RiskLevel::High.to_string(), "High");
        assert_eq!(Platform::MacOS.to_string(), "macOS");
        assert_eq!(PrivilegeLevel::Root.to_string(), "Root");
        assert_eq!(ApprovalStatus::Expired.to_string(), "Expired");
    }
}
