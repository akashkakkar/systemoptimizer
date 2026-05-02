//! Disk usage probe implementation.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use chrono::Utc;
use lso_core::{MetricValue, Platform, PrivilegeLevel, ProbeResult, SensorError, SystemProbe};

use crate::platform;

/// Probe that collects disk usage metrics for all mounted volumes.
pub struct DiskUsageProbe {
    platform: Platform,
}

impl DiskUsageProbe {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }
}

impl SystemProbe for DiskUsageProbe {
    fn probe_id(&self) -> &str {
        "disk.usage"
    }

    fn required_privilege(&self) -> PrivilegeLevel {
        PrivilegeLevel::None
    }

    fn collect(&self) -> Pin<Box<dyn Future<Output = Result<ProbeResult, SensorError>> + Send + '_>> {
        Box::pin(async move {
            let mount_stats = platform::collect_mount_stats(self.platform)?;

            let metrics: Vec<HashMap<String, MetricValue>> = mount_stats
                .into_iter()
                .map(|ms| {
                    let mut map = HashMap::new();
                    map.insert("mount_point".to_string(), MetricValue::Text(ms.mount_point));
                    map.insert("fs_type".to_string(), MetricValue::Text(ms.fs_type));
                    map.insert("total_bytes".to_string(), MetricValue::Uint(ms.total_bytes));
                    map.insert("used_bytes".to_string(), MetricValue::Uint(ms.used_bytes));
                    map.insert(
                        "available_bytes".to_string(),
                        MetricValue::Uint(ms.available_bytes),
                    );
                    map.insert(
                        "usage_percent".to_string(),
                        MetricValue::Float(ms.usage_percent),
                    );
                    map
                })
                .collect();

            Ok(ProbeResult {
                probe_id: self.probe_id().to_string(),
                timestamp: Utc::now(),
                metrics,
            })
        })
    }
}
