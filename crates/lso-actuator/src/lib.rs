//! LSO Actuator — gated executor with snapshot-before-mutate safety.

pub mod cleanup;
pub mod snapshot;

pub use cleanup::TempCleanupExecutor;
pub use snapshot::SnapshotManager;
