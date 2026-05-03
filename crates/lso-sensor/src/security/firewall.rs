//! Firewall status probe.
//!
//! Linux: tries ufw, then iptables, then nftables.
//! macOS / Windows: stubbed.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use lso_core::{
    FirewallBackend, FirewallStatus, Platform, PrivilegeLevel, ProbeResult, SensorError,
    SystemMetric, SystemProbe,
};

pub const PROBE_ID: &str = "sensor.security.firewall";

pub struct FirewallProbe {
    platform: Platform,
}

impl FirewallProbe {
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
impl SystemProbe for FirewallProbe {
    fn probe_id(&self) -> &str {
        PROBE_ID
    }

    fn description(&self) -> &str {
        "System firewall status"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::Unprivileged
    }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        let status = collect_firewall_status(self.platform);
        let now = Utc::now();
        let platform_label = self.platform.to_string();

        let metrics = vec![
            SystemMetric {
                id: Uuid::new_v4(),
                probe_id: PROBE_ID.into(),
                name: "system::enabled".into(),
                value: if status.enabled { 1.0 } else { 0.0 },
                unit: None,
                collected_at: now,
                platform: platform_label.clone(),
            },
            SystemMetric {
                id: Uuid::new_v4(),
                probe_id: PROBE_ID.into(),
                name: "system::rule_count".into(),
                value: status.rule_count as f64,
                unit: None,
                collected_at: now,
                platform: platform_label,
            },
        ];

        Ok(ProbeResult {
            probe_id: PROBE_ID.into(),
            metrics,
            collected_at: now,
            platform: self.platform,
        })
    }
}

fn collect_firewall_status(platform: Platform) -> FirewallStatus {
    match platform {
        Platform::Linux => {
            #[cfg(target_os = "linux")]
            {
                if let Some(s) = try_ufw() {
                    return s;
                }
                if let Some(s) = try_iptables() {
                    return s;
                }
                if let Some(s) = try_nftables() {
                    return s;
                }
            }
            FirewallStatus {
                enabled: false,
                backend: FirewallBackend::Unknown,
                default_policy: "unknown".into(),
                rule_count: 0,
            }
        }
        _ => {
            tracing::info!("firewall probe: stubbed on {platform}");
            FirewallStatus {
                enabled: false,
                backend: FirewallBackend::Unknown,
                default_policy: "unknown".into(),
                rule_count: 0,
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn try_ufw() -> Option<FirewallStatus> {
    use std::process::Command;
    let output = Command::new("ufw").arg("status").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.contains("inactive") {
        return Some(FirewallStatus {
            enabled: false,
            backend: FirewallBackend::Ufw,
            default_policy: "allow".into(),
            rule_count: 0,
        });
    }

    if stdout.contains("active") {
        let rule_count = stdout
            .lines()
            .filter(|l| l.contains("ALLOW") || l.contains("DENY") || l.contains("REJECT"))
            .count();
        let default_policy = if stdout.contains("deny (incoming)") {
            "deny"
        } else {
            "allow"
        };
        return Some(FirewallStatus {
            enabled: true,
            backend: FirewallBackend::Ufw,
            default_policy: default_policy.into(),
            rule_count,
        });
    }

    None
}

#[cfg(target_os = "linux")]
fn try_iptables() -> Option<FirewallStatus> {
    use std::process::Command;
    let output = Command::new("iptables")
        .args(["-L", "-n", "--line-numbers"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let rule_count = stdout
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with("Chain")
                && !t.starts_with("num")
                && !t.starts_with("target")
        })
        .count();

    let default_policy = stdout
        .lines()
        .find(|l| l.starts_with("Chain INPUT"))
        .map(|l| {
            if l.contains("DROP") {
                "drop"
            } else if l.contains("REJECT") {
                "reject"
            } else {
                "accept"
            }
        })
        .unwrap_or("accept");

    Some(FirewallStatus {
        enabled: rule_count > 0 || default_policy != "accept",
        backend: FirewallBackend::Iptables,
        default_policy: default_policy.into(),
        rule_count,
    })
}

#[cfg(target_os = "linux")]
fn try_nftables() -> Option<FirewallStatus> {
    use std::process::Command;
    let output = Command::new("nft")
        .args(["list", "ruleset"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let rule_count = stdout
        .lines()
        .filter(|l| l.trim().starts_with("rule"))
        .count();

    Some(FirewallStatus {
        enabled: rule_count > 0,
        backend: FirewallBackend::Nftables,
        default_policy: "unknown".into(),
        rule_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_id_correct() {
        let probe = FirewallProbe::new(Platform::Linux);
        assert_eq!(probe.probe_id(), PROBE_ID);
    }

    #[tokio::test]
    async fn collect_returns_metrics() {
        let probe = FirewallProbe::new(Platform::detect().unwrap_or(Platform::Linux));
        let result = probe.collect().await.unwrap();
        let names: Vec<&str> = result.metrics.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"system::enabled"));
        assert!(names.contains(&"system::rule_count"));
    }
}
