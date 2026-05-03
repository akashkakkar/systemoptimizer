//! Sensor registry — registers probes and collects results in parallel.

use lso_core::{ProbeResult, SensorError, SystemProbe};
use tracing::{debug, warn};

/// Central registry that holds all system probes and can collect from them in parallel.
pub struct SensorRegistry {
    probes: Vec<Box<dyn SystemProbe>>,
}

impl SensorRegistry {
    pub fn new() -> Self {
        Self { probes: Vec::new() }
    }

    /// Register a probe with the registry.
    pub fn register(&mut self, probe: Box<dyn SystemProbe>) {
        debug!(probe_id = probe.probe_id(), "registered probe");
        self.probes.push(probe);
    }

    /// Number of registered probes.
    pub fn len(&self) -> usize {
        self.probes.len()
    }

    /// Whether the registry has no probes.
    pub fn is_empty(&self) -> bool {
        self.probes.is_empty()
    }

    /// Collect results from all registered probes in parallel.
    pub async fn collect_all(&self) -> Vec<ProbeResult> {
        let futures: Vec<_> = self
            .probes
            .iter()
            .map(|probe| async move {
                let id = probe.probe_id().to_string();
                match probe.collect().await {
                    Ok(result) => Some(result),
                    Err(SensorError::PermissionRequired(msg)) => {
                        warn!(probe_id = %id, %msg, "probe skipped: permission required");
                        None
                    }
                    Err(e) => {
                        warn!(probe_id = %id, error = %e, "probe collection failed");
                        None
                    }
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;
        results.into_iter().flatten().collect()
    }
}

impl Default for SensorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use lso_core::{Platform, PrivilegeLevel, SystemMetric};
    use uuid::Uuid;

    struct FakeProbe {
        id: &'static str,
        value: f64,
    }

    #[async_trait]
    impl SystemProbe for FakeProbe {
        fn probe_id(&self) -> &str {
            self.id
        }
        fn description(&self) -> &str {
            "fake"
        }
        fn required_privilege(&self) -> PrivilegeLevel {
            PrivilegeLevel::Unprivileged
        }
        async fn collect(&self) -> Result<ProbeResult, SensorError> {
            Ok(ProbeResult {
                probe_id: self.id.to_string(),
                metrics: vec![SystemMetric {
                    id: Uuid::new_v4(),
                    probe_id: self.id.to_string(),
                    name: "test::value".to_string(),
                    value: self.value,
                    unit: None,
                    collected_at: Utc::now(),
                    platform: "test".to_string(),
                }],
                collected_at: Utc::now(),
                platform: Platform::Linux,
            })
        }
    }

    struct FailingProbe;

    #[async_trait]
    impl SystemProbe for FailingProbe {
        fn probe_id(&self) -> &str {
            "failing"
        }
        fn description(&self) -> &str {
            "always fails"
        }
        fn required_privilege(&self) -> PrivilegeLevel {
            PrivilegeLevel::Unprivileged
        }
        async fn collect(&self) -> Result<ProbeResult, SensorError> {
            Err(SensorError::ProbeFailed {
                probe: "failing".into(),
                reason: "intentional".into(),
            })
        }
    }

    #[tokio::test]
    async fn collect_all_parallel() {
        let mut registry = SensorRegistry::new();
        registry.register(Box::new(FakeProbe { id: "a", value: 1.0 }));
        registry.register(Box::new(FakeProbe { id: "b", value: 2.0 }));

        let results = registry.collect_all().await;
        assert_eq!(results.len(), 2);

        let ids: Vec<&str> = results.iter().map(|r| r.probe_id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
    }

    #[tokio::test]
    async fn collect_all_skips_failures() {
        let mut registry = SensorRegistry::new();
        registry.register(Box::new(FakeProbe { id: "good", value: 1.0 }));
        registry.register(Box::new(FailingProbe));

        let results = registry.collect_all().await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].probe_id, "good");
    }

    #[tokio::test]
    async fn empty_registry_returns_empty() {
        let registry = SensorRegistry::new();
        let results = registry.collect_all().await;
        assert!(results.is_empty());
    }

    #[test]
    fn register_increments_len() {
        let mut registry = SensorRegistry::new();
        assert!(registry.is_empty());
        registry.register(Box::new(FakeProbe { id: "x", value: 0.0 }));
        assert_eq!(registry.len(), 1);
    }
}
