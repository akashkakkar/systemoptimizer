//! Startup item discovery probe (F19).
//!
//! Linux: systemd services + XDG autostart (fully implemented).
//! macOS / Windows: stubbed, returns empty results.

#[cfg(target_os = "linux")]
mod linux;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use lso_core::{
    Platform, PrivilegeLevel, ProbeResult, SensorError, StartupItem, SystemMetric, SystemProbe,
};

pub const PROBE_ID: &str = "sensor.startup";

pub struct StartupProbe {
    platform: Platform,
}

impl StartupProbe {
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
impl SystemProbe for StartupProbe {
    fn probe_id(&self) -> &str {
        PROBE_ID
    }

    fn description(&self) -> &str {
        "Startup/login item discovery"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::Unprivileged
    }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        let items = collect_startup_items(self.platform)?;
        let now = Utc::now();
        let platform_label = self.platform.to_string();

        let mut metrics = Vec::new();
        let mut duplicate_count = 0u32;
        let mut seen_commands = std::collections::HashMap::<String, u32>::new();

        for item in &items {
            let prefix = &item.name;
            metrics.push(SystemMetric {
                id: Uuid::new_v4(),
                probe_id: PROBE_ID.into(),
                name: format!("{prefix}::enabled"),
                value: if item.enabled { 1.0 } else { 0.0 },
                unit: None,
                collected_at: now,
                platform: platform_label.clone(),
            });
            metrics.push(SystemMetric {
                id: Uuid::new_v4(),
                probe_id: PROBE_ID.into(),
                name: format!("{prefix}::has_publisher"),
                value: if item.publisher.is_some() { 1.0 } else { 0.0 },
                unit: None,
                collected_at: now,
                platform: platform_label.clone(),
            });

            *seen_commands.entry(item.command.clone()).or_default() += 1;
        }

        for count in seen_commands.values() {
            if *count > 1 {
                duplicate_count += 1;
            }
        }

        let enabled_count = items.iter().filter(|i| i.enabled).count();

        metrics.push(SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.into(),
            name: "total::item_count".into(),
            value: items.len() as f64,
            unit: None,
            collected_at: now,
            platform: platform_label.clone(),
        });
        metrics.push(SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.into(),
            name: "total::enabled_count".into(),
            value: enabled_count as f64,
            unit: None,
            collected_at: now,
            platform: platform_label.clone(),
        });
        metrics.push(SystemMetric {
            id: Uuid::new_v4(),
            probe_id: PROBE_ID.into(),
            name: "total::duplicate_count".into(),
            value: duplicate_count as f64,
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

/// Collect raw startup items for the given platform.
/// Exposed for the UI layer to render the full item list.
pub fn collect_startup_items(platform: Platform) -> Result<Vec<StartupItem>, SensorError> {
    match platform {
        Platform::Linux => {
            #[cfg(target_os = "linux")]
            {
                linux::scan_startup_items()
            }
            #[cfg(not(target_os = "linux"))]
            {
                tracing::info!("Linux startup probe: cross-compiled stub, returning empty");
                Ok(Vec::new())
            }
        }
        Platform::MacOS => {
            tracing::info!("macOS startup probe: not yet implemented");
            Ok(Vec::new())
        }
        Platform::Windows => {
            tracing::info!("Windows startup probe: not yet implemented");
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_id_correct() {
        let probe = StartupProbe::new(Platform::Linux);
        assert_eq!(probe.probe_id(), "sensor.startup");
        assert_eq!(probe.required_privilege(), PrivilegeLevel::Unprivileged);
    }

    #[tokio::test]
    async fn collect_returns_summary_metrics() {
        let probe = StartupProbe::new(Platform::detect().unwrap_or(Platform::Linux));
        let result = probe.collect().await.unwrap();
        assert_eq!(result.probe_id, PROBE_ID);
        let metric_names: Vec<&str> = result.metrics.iter().map(|m| m.name.as_str()).collect();
        assert!(metric_names.contains(&"total::item_count"));
        assert!(metric_names.contains(&"total::enabled_count"));
        assert!(metric_names.contains(&"total::duplicate_count"));
    }
}
