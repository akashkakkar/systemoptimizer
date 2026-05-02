//! LSO Sensor — read-only system probes for gathering system state.

mod disk_usage;
mod platform;

pub use disk_usage::{get_disk_reports, DiskUsageProbe, PROBE_ID as DISK_USAGE_PROBE_ID};
