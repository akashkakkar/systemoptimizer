# F11 — Snapshot Manager

**Phase:** 2 — Action
**Size:** M (1-2 sessions)
**Branch:** `feat/P2-snapshot-manager`
**Depends on:** F03

## Objective

Create OS-level snapshots before any system mutation. Provides rollback safety net.

## Platform Implementations

### macOS — APFS Snapshots
```bash
tmutil localsnapshot  # Creates APFS snapshot
tmutil listlocalsnapshots /
tmutil deletelocalsnapshots <date>
```

### Linux — Btrfs or Tar Fallback
- Btrfs: `btrfs subvolume snapshot`
- Fallback: tar archive of target directory
- Detect filesystem type, choose automatically

### Windows — Volume Shadow Copy (VSS)
- `vssadmin create shadow /for=C:`
- Requires elevation (UAC prompt)

## Deliverables

### 1. Snapshot Trait (`lso-core/src/traits.rs`)
```rust
#[async_trait]
pub trait SnapshotProvider: Send + Sync {
    async fn create(&self, label: &str) -> Result<SnapshotId, ActuatorError>;
    async fn list(&self) -> Result<Vec<SnapshotInfo>, ActuatorError>;
    async fn restore(&self, id: &SnapshotId) -> Result<(), ActuatorError>;
    async fn delete(&self, id: &SnapshotId) -> Result<(), ActuatorError>;
    fn supports_platform(&self) -> bool;
}
```

### 2. SnapshotInfo Type
```rust
pub struct SnapshotInfo {
    pub id: SnapshotId,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub size_estimate: Option<u64>,
    pub platform_detail: String, // e.g., "APFS snapshot com.apple.TimeMachine..."
}
```

### 3. Snapshot Manager (`lso-actuator/src/snapshot.rs`)
- Auto-label with timestamp + action name
- Track in database for UI display
- Cleanup: expire snapshots older than N days (configurable)

## Acceptance Criteria

- [ ] Snapshot creates successfully on at least one platform
- [ ] Snapshot ID stored in database, linked to action
- [ ] Restore reverts changes (verified in integration test with temp dir)
- [ ] Handles "insufficient space" error gracefully
- [ ] Requires elevation only on platforms that need it
- [ ] Unsupported platform returns clear error, doesn't crash
