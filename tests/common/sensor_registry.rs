//! Cross-platform sensor registry tests.

use lso_sensor::build_registry;

#[tokio::test]
async fn registry_returns_results_on_host() {
    let registry = build_registry();
    let results = registry.collect_all().await;
    assert!(
        !results.is_empty(),
        "sensor registry should return at least one probe result on any platform"
    );
}

#[tokio::test]
async fn disk_probe_returns_data() {
    let registry = build_registry();
    let results = registry.collect_all().await;
    let disk = results.iter().find(|r| r.probe_id == "disk.usage");
    assert!(
        disk.is_some(),
        "disk probe should succeed on all platforms"
    );
    let disk = disk.unwrap();
    assert!(!disk.metrics.is_empty(), "disk probe should return metrics");
}
