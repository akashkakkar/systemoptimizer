//! End-to-end test: probe disk -> store result -> retrieve from DB.

use lso_core::Platform;
use lso_db::{ProbeStore, StorageBackend};
use lso_sensor::create_disk_probe;

#[tokio::test]
async fn probe_store_retrieve_pipeline() {
    if cfg!(not(any(target_os = "linux", target_os = "macos"))) {
        return;
    }

    let platform = Platform::current();
    let probe = create_disk_probe(platform);
    let result = probe.collect().await.expect("probe should succeed");

    let dir = tempfile::tempdir().unwrap();
    let store = StorageBackend::new(dir.path()).unwrap();

    store.store(&result).unwrap();

    let retrieved = store
        .latest("disk.usage")
        .unwrap()
        .expect("should retrieve stored result");

    assert_eq!(retrieved.probe_id, "disk.usage");
    assert_eq!(retrieved.metrics.len(), result.metrics.len());
}
