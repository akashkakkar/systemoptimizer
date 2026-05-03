//! Windows-specific sensor integration tests.
//! Only compiled and run on Windows.

#![cfg(target_os = "windows")]

use lso_core::{Platform, SystemProbe};
use lso_sensor::{CpuProbe, MemoryProbe, ProcessListProbe};

#[tokio::test]
async fn process_probe_returns_data() {
    let probe = ProcessListProbe::new(Platform::Windows);
    let result = probe.collect().await.unwrap();
    assert!(!result.metrics.is_empty());
}

#[tokio::test]
async fn memory_probe_returns_data() {
    let probe = MemoryProbe::new(Platform::Windows);
    let result = probe.collect().await.unwrap();
    assert_eq!(result.metrics.len(), 7);
}

#[tokio::test]
async fn cpu_probe_returns_data() {
    let probe = CpuProbe::new(Platform::Windows);
    let result = probe.collect().await.unwrap();
    assert!(!result.metrics.is_empty());
}

#[test]
fn platform_detection_returns_windows() {
    assert_eq!(Platform::detect(), Some(Platform::Windows));
}
