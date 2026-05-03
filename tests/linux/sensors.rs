//! Linux-specific sensor integration tests.
//! Only compiled and run on Linux.

#![cfg(target_os = "linux")]

use lso_core::{Platform, SystemProbe};
use lso_sensor::{CpuProbe, MemoryProbe, ProcessListProbe};

#[tokio::test]
async fn process_probe_returns_data() {
    let probe = ProcessListProbe::new(Platform::Linux);
    let result = probe.collect().await.unwrap();
    assert!(!result.metrics.is_empty());
}

#[tokio::test]
async fn memory_probe_returns_data() {
    let probe = MemoryProbe::new(Platform::Linux);
    let result = probe.collect().await.unwrap();
    assert_eq!(result.metrics.len(), 7);
}

#[tokio::test]
async fn cpu_probe_returns_data() {
    let probe = CpuProbe::new(Platform::Linux);
    let result = probe.collect().await.unwrap();
    let core_count = result
        .metrics
        .iter()
        .find(|m| m.name == "system::core_count")
        .unwrap();
    assert!(core_count.value >= 1.0);
}

#[test]
fn platform_detection_returns_linux() {
    assert_eq!(Platform::detect(), Some(Platform::Linux));
}
