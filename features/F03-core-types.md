# F03 — Core Types, Errors & Platform Traits

**Phase:** 0 — Foundation
**Size:** S (one session)
**Branch:** `feat/P0-core-types`
**Depends on:** F02

## Objective

Define the shared types, error hierarchy, platform abstraction traits, and logging infrastructure that every other crate depends on.

## Deliverables

### 1. Error Types (`lso-core/src/error.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LsoError {
    #[error("Sensor error: {0}")]
    Sensor(#[from] SensorError),
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("Engine error: {0}")]
    Engine(#[from] EngineError),
    #[error("Actuator error: {0}")]
    Actuator(#[from] ActuatorError),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}

#[derive(Error, Debug)]
pub enum SensorError {
    #[error("Probe failed: {probe} — {reason}")]
    ProbeFailed { probe: String, reason: String },
    #[error("Permission required: {0}")]
    PermissionRequired(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
// ... similar for DatabaseError, EngineError, ActuatorError
```

### 2. Core Domain Types (`lso-core/src/types.rs`)

- `RiskLevel` — enum: Low, Medium, High, Critical
- `Recommendation` — struct: id, title, description, risk_level, category, target, rollback_plan
- `ApprovalStatus` — enum: Pending, Approved, Rejected, Expired
- `SystemMetric` — struct: name, value (f64), unit, timestamp, source_probe
- `ProbeResult` — struct: probe_id, metrics (Vec<SystemMetric>), collected_at, platform
- `AuditEntry` — struct: timestamp, action, target, risk_level, user_approved, snapshot_id, result
- `Platform` — enum: MacOS, Linux, Windows (detected at runtime)

### 3. Platform Trait (`lso-core/src/platform.rs`)

```rust
pub trait PlatformProvider: Send + Sync {
    fn platform(&self) -> Platform;
    fn home_dir(&self) -> PathBuf;
    fn temp_dir(&self) -> PathBuf;
    fn data_dir(&self) -> PathBuf;  // App data location
    fn config_dir(&self) -> PathBuf;
}
```

### 4. Sensor Trait (`lso-core/src/traits.rs`)

```rust
#[async_trait]
pub trait SystemProbe: Send + Sync {
    fn probe_id(&self) -> &str;
    fn description(&self) -> &str;
    fn required_privilege(&self) -> PrivilegeLevel;
    async fn collect(&self) -> Result<ProbeResult, SensorError>;
}
```

### 5. Logging (`tracing` setup)

- Use `tracing` crate with `tracing-subscriber`
- Log to file in app data dir (rotated, not to stdout in release)
- Structured logging: `tracing::info!(probe = "disk_usage", "Probe completed in {}ms", elapsed)`
- Log levels: ERROR for failures, WARN for degraded, INFO for operations, DEBUG for internals

## Steps

1. Create `error.rs`, `types.rs`, `platform.rs`, `traits.rs` in `lso-core/src/`
2. Update `lso-core/src/lib.rs` to re-export all public types
3. Add `serde` derives on all types for serialization
4. Add `Display` formatting for user-facing types
5. Write unit tests for serialization roundtrips
6. Verify downstream crates can import core types

## Acceptance Criteria

- [ ] All types compile and serialize/deserialize correctly
- [ ] `Platform` auto-detected at runtime via `std::env::consts::OS`
- [ ] Error types chain properly with `?` operator
- [ ] `tracing` subscriber initializes without panic
- [ ] Clippy clean, all tests pass
- [ ] No `unwrap()` anywhere
