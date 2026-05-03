//! macOS-specific sensor integration tests.
//! Only compiled and run on macOS.

#![cfg(target_os = "macos")]

use lso_core::{Platform, PlatformProvider, SystemProbe};
use lso_sensor::{CpuProbe, MemoryProbe, ProcessListProbe};

#[tokio::test]
async fn process_probe_returns_data() {
    let probe = ProcessListProbe::new(Platform::MacOS);
    let result = probe.collect().await.unwrap();
    assert_eq!(result.probe_id, "sensor.process.list");
    assert!(
        !result.metrics.is_empty(),
        "macOS process probe should return metrics"
    );
}

#[tokio::test]
async fn memory_probe_returns_data() {
    let probe = MemoryProbe::new(Platform::MacOS);
    let result = probe.collect().await.unwrap();
    assert_eq!(result.probe_id, "sensor.memory.usage");
    assert_eq!(result.metrics.len(), 7, "memory probe returns 7 metrics");

    let total = result
        .metrics
        .iter()
        .find(|m| m.name == "system::total_bytes")
        .expect("should have total_bytes");
    assert!(
        total.value > 1_000_000_000.0,
        "total memory should be > 1GB"
    );

    let usage = result
        .metrics
        .iter()
        .find(|m| m.name == "system::usage_percent")
        .expect("should have usage_percent");
    assert!(usage.value > 0.0 && usage.value <= 100.0);
}

#[tokio::test]
async fn cpu_probe_returns_data() {
    let probe = CpuProbe::new(Platform::MacOS);
    let result = probe.collect().await.unwrap();
    assert_eq!(result.probe_id, "sensor.cpu.load");

    let core_count = result
        .metrics
        .iter()
        .find(|m| m.name == "system::core_count")
        .expect("should have core_count");
    assert!(core_count.value >= 1.0, "should have at least 1 core");

    let load1 = result
        .metrics
        .iter()
        .find(|m| m.name == "system::load_avg_1")
        .expect("should have load_avg_1");
    assert!(load1.value >= 0.0, "load average should be non-negative");
}

#[tokio::test]
async fn startup_probe_returns_data() {
    let probe = lso_sensor::StartupProbe::new(Platform::MacOS);
    let result = probe.collect().await.unwrap();
    assert_eq!(result.probe_id, "sensor.startup");
    let names: Vec<&str> = result.metrics.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"total::item_count"));
}

#[test]
fn platform_detection_returns_macos() {
    let platform = Platform::detect().expect("should detect platform");
    assert_eq!(platform, Platform::MacOS);
}

#[test]
fn path_resolution_uses_macos_conventions() {
    let provider =
        lso_core::NativePlatformProvider::new().expect("should create provider");
    let home = provider.home_dir();
    assert!(
        home.starts_with("/Users") || home.starts_with("/var"),
        "macOS home should start with /Users, got: {home:?}"
    );
    let data = provider.data_dir();
    assert!(
        data.to_string_lossy().contains("Library")
            || data.to_string_lossy().contains("lso"),
        "macOS data dir should be in Library, got: {data:?}"
    );
}
