//! Integration test: runs the disk usage probe on the current platform
//! and verifies it returns real data.

use lso_core::{Platform, PrivilegeLevel};
use lso_sensor::create_disk_probe;

#[tokio::test]
async fn probe_returns_real_disk_data() {
    let platform = Platform::current();
    let probe = create_disk_probe(platform);

    assert_eq!(probe.probe_id(), "disk.usage");
    assert_eq!(probe.required_privilege(), PrivilegeLevel::None);

    let result = probe.collect().await;

    // On Linux/macOS this should succeed; on Windows it returns PlatformNotSupported
    if cfg!(any(target_os = "linux", target_os = "macos")) {
        let result = result.expect("probe should succeed on this platform");
        assert_eq!(result.probe_id, "disk.usage");
        assert!(
            !result.metrics.is_empty(),
            "should have at least one mounted volume"
        );

        for metric in &result.metrics {
            let total = match metric.get("total_bytes") {
                Some(lso_core::MetricValue::Uint(v)) => *v,
                _ => panic!("missing total_bytes"),
            };
            let used = match metric.get("used_bytes") {
                Some(lso_core::MetricValue::Uint(v)) => *v,
                _ => panic!("missing used_bytes"),
            };
            let available = match metric.get("available_bytes") {
                Some(lso_core::MetricValue::Uint(v)) => *v,
                _ => panic!("missing available_bytes"),
            };
            let percent = match metric.get("usage_percent") {
                Some(lso_core::MetricValue::Float(v)) => *v,
                _ => panic!("missing usage_percent"),
            };

            assert!(total > 0, "total_bytes should be > 0");
            assert!(used <= total, "used should not exceed total");
            assert!(available <= total, "available should not exceed total");
            assert!((0.0..=100.0).contains(&percent), "percent out of range");

            assert!(
                metric.contains_key("mount_point"),
                "missing mount_point"
            );
            assert!(metric.contains_key("fs_type"), "missing fs_type");
        }
    } else {
        assert!(result.is_err(), "should return PlatformNotSupported");
    }
}

#[tokio::test]
async fn probe_result_serializes_to_json() {
    let platform = Platform::current();
    let probe = create_disk_probe(platform);

    if cfg!(any(target_os = "linux", target_os = "macos")) {
        let result = probe.collect().await.expect("probe should succeed");
        let json = serde_json::to_string(&result).expect("should serialize");
        assert!(json.contains("disk.usage"));
        assert!(json.contains("total_bytes"));
    }
}
