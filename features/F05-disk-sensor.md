# F05 — Disk Usage Sensor (First Probe)

**Phase:** 0 — Foundation
**Size:** S (one session)
**Branch:** `feat/P0-disk-sensor`
**Depends on:** F03, F04

## Objective

Implement the first working sensor: disk usage probe. Establishes the pattern all future sensors follow.

## Deliverables

### 1. Platform-Specific Implementations

**Linux/macOS:** Use `statvfs` via `nix` crate or `std::fs` metadata
**Windows:** Use `GetDiskFreeSpaceExW` via `windows` crate

### 2. DiskUsageProbe (`lso-sensor/src/disk_usage.rs`)

```rust
pub struct DiskUsageProbe {
    platform: Platform,
}

impl SystemProbe for DiskUsageProbe {
    fn probe_id(&self) -> &str { "disk.usage" }
    fn required_privilege(&self) -> PrivilegeLevel { PrivilegeLevel::None }

    async fn collect(&self) -> Result<ProbeResult, SensorError> {
        // Returns metrics for each mounted volume:
        // - disk.total_bytes
        // - disk.used_bytes
        // - disk.available_bytes
        // - disk.usage_percent
        // - disk.mount_point (as label)
    }
}
```

### 3. Data Collected Per Mount Point
- Total capacity (bytes)
- Used space (bytes)
- Available space (bytes)
- Usage percentage
- Filesystem type (ext4, APFS, NTFS, etc.)
- Mount point path

### 4. Platform Abstraction
```rust
// In lso-sensor/src/platform/
mod linux;   // statvfs + /proc/mounts
mod macos;   // statvfs + diskutil
mod windows; // GetDiskFreeSpaceExW

// Factory
pub fn create_disk_probe(platform: Platform) -> Box<dyn SystemProbe>
```

## Steps

1. Add platform detection to `lso-sensor`
2. Implement Linux disk probe (primary dev platform)
3. Create `SystemProbe` implementation
4. Store results via `lso-db`
5. Write unit tests with mock data
6. Write integration test that reads real disk (runs on dev machine)
7. Stub macOS/Windows implementations (compile but return `PlatformNotSupported`)

## Acceptance Criteria

- [ ] Probe returns accurate disk usage for all mounted volumes
- [ ] Results serialize to JSON correctly
- [ ] Results store in database via F04
- [ ] Runs without elevated privileges
- [ ] Handles permission errors gracefully (e.g., restricted mounts)
- [ ] Excludes pseudo-filesystems (proc, sys, devfs)
- [ ] At least one platform fully working, others stubbed
