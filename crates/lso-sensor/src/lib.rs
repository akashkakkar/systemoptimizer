//! LSO Sensor — read-only system probes for gathering system state.

mod disk_usage;
mod platform;

pub use disk_usage::DiskUsageProbe;

use lso_core::{Platform, SystemProbe};

/// Factory function to create a disk usage probe for the given platform.
pub fn create_disk_probe(platform: Platform) -> Box<dyn SystemProbe> {
    Box::new(DiskUsageProbe::new(platform))
}
