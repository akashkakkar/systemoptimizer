//! Permissions probe — world-writable dirs, SSH key perms, home dir perms.
//!
//! Works on Unix (Linux + macOS). Windows stubbed.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use lso_core::{
    PermissionIssue, PermissionIssueType, Platform, PrivilegeLevel, ProbeResult, SensorError,
    SystemMetric, SystemProbe,
};

pub const PROBE_ID: &str = "sensor.security.permissions";

pub struct PermissionsProbe {
    platform: Platform,
}

impl PermissionsProbe {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn for_host() -> Result<Self, SensorError> {
        let platform = Platform::detect().ok_or_else(|| SensorError::ProbeFailed {
            probe: PROBE_ID.into(),
            reason: format!("unknown platform: {}", std::env::consts::OS),
        })?;
        Ok(Self::new(platform))
    }
}

#[async_trait]
impl SystemProbe for PermissionsProbe {
    fn probe_id(&self) -> &str {
        PROBE_ID
    }

    fn description(&self) -> &str {
        "File/directory permission issues"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::Unprivileged
    }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        let issues = collect_permission_issues(self.platform);
        let now = Utc::now();
        let platform_label = self.platform.to_string();

        let mut metrics: Vec<SystemMetric> = issues
            .iter()
            .map(|issue| {
                let metric_name = match issue.issue_type {
                    PermissionIssueType::WorldWritable => "world_writable",
                    PermissionIssueType::InsecureSshKey => "insecure_ssh_key",
                    PermissionIssueType::OverlyPermissiveHome => "permissive_home",
                    PermissionIssueType::SuidBinary => "suid_binary",
                    PermissionIssueType::SgidBinary => "sgid_binary",
                };
                SystemMetric {
                    id: Uuid::new_v4(),
                    probe_id: PROBE_ID.into(),
                    name: format!("{}::{metric_name}", issue.path.display()),
                    value: 1.0,
                    unit: None,
                    collected_at: now,
                    platform: platform_label.clone(),
                }
            })
            .collect();

        let ssh_issue_count = issues
            .iter()
            .filter(|i| i.issue_type == PermissionIssueType::InsecureSshKey)
            .count();

        metrics.push(SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.into(),
            name: "total::issue_count".into(),
            value: issues.len() as f64,
            unit: None,
            collected_at: now,
            platform: platform_label.clone(),
        });
        metrics.push(SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.into(),
            name: "total::ssh_issue_count".into(),
            value: ssh_issue_count as f64,
            unit: None,
            collected_at: now,
            platform: platform_label,
        });

        Ok(ProbeResult {
            probe_id: PROBE_ID.into(),
            metrics,
            collected_at: now,
            platform: self.platform,
        })
    }
}

fn collect_permission_issues(_platform: Platform) -> Vec<PermissionIssue> {
    let mut issues = Vec::new();

    #[cfg(unix)]
    {
        issues.extend(check_world_writable_dirs());
        issues.extend(check_ssh_key_permissions());
        issues.extend(check_home_permissions());
    }

    #[cfg(not(unix))]
    {
        tracing::info!("permissions probe: stubbed on {}", std::env::consts::OS);
    }

    issues
}

#[cfg(unix)]
fn file_mode(path: &std::path::Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).ok().map(|m| m.permissions().mode())
}

#[cfg(unix)]
fn check_world_writable_dirs() -> Vec<PermissionIssue> {
    let sensitive_dirs = ["/tmp", "/var/tmp"];
    let mut issues = Vec::new();

    for dir in &sensitive_dirs {
        let path = std::path::PathBuf::from(dir);
        if let Some(mode) = file_mode(&path) {
            let world_writable = mode & 0o002 != 0;
            let sticky = mode & 0o1000 != 0;
            if world_writable && !sticky {
                issues.push(PermissionIssue {
                    path,
                    issue_type: PermissionIssueType::WorldWritable,
                    current_mode: mode & 0o7777,
                    recommended_mode: Some(0o1777),
                    description: format!(
                        "{dir} is world-writable without sticky bit (mode {:04o})",
                        mode & 0o7777
                    ),
                });
            }
        }
    }

    issues
}

#[cfg(unix)]
fn check_ssh_key_permissions() -> Vec<PermissionIssue> {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };

    let ssh_dir = std::path::PathBuf::from(&home).join(".ssh");
    if !ssh_dir.exists() {
        return Vec::new();
    }

    let mut issues = Vec::new();

    if let Some(mode) = file_mode(&ssh_dir) {
        let perms = mode & 0o7777;
        if perms & 0o077 != 0 {
            issues.push(PermissionIssue {
                path: ssh_dir.clone(),
                issue_type: PermissionIssueType::InsecureSshKey,
                current_mode: perms,
                recommended_mode: Some(0o700),
                description: format!(".ssh directory too permissive (mode {perms:04o}, should be 0700)"),
            });
        }
    }

    let entries = match std::fs::read_dir(&ssh_dir) {
        Ok(e) => e,
        Err(_) => return issues,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_owned();
        if !name.starts_with("id_") || name.ends_with(".pub") {
            continue;
        }
        if let Some(mode) = file_mode(&path) {
            let perms = mode & 0o7777;
            if perms & 0o077 != 0 {
                issues.push(PermissionIssue {
                    path,
                    issue_type: PermissionIssueType::InsecureSshKey,
                    current_mode: perms,
                    recommended_mode: Some(0o600),
                    description: format!(
                        "SSH private key {name} too permissive (mode {perms:04o}, should be 0600)"
                    ),
                });
            }
        }
    }

    issues
}

#[cfg(unix)]
fn check_home_permissions() -> Vec<PermissionIssue> {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };

    let home_path = std::path::PathBuf::from(&home);
    if let Some(mode) = file_mode(&home_path) {
        let perms = mode & 0o7777;
        if perms & 0o006 != 0 {
            return vec![PermissionIssue {
                path: home_path,
                issue_type: PermissionIssueType::OverlyPermissiveHome,
                current_mode: perms,
                recommended_mode: Some(0o750),
                description: format!(
                    "home directory world-accessible (mode {perms:04o}, recommended 0750)"
                ),
            }];
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_id_correct() {
        let probe = PermissionsProbe::new(Platform::Linux);
        assert_eq!(probe.probe_id(), PROBE_ID);
    }

    #[tokio::test]
    async fn collect_returns_summary_metrics() {
        let probe = PermissionsProbe::new(Platform::detect().unwrap_or(Platform::Linux));
        let result = probe.collect().await.unwrap();
        let names: Vec<&str> = result.metrics.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"total::issue_count"));
        assert!(names.contains(&"total::ssh_issue_count"));
    }

    #[cfg(unix)]
    #[test]
    fn file_mode_works() {
        let mode = file_mode(std::path::Path::new("/tmp"));
        assert!(mode.is_some());
    }
}
