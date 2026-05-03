//! LSO Sensor — read-only system probes for gathering system state.

mod disk_usage;
mod platform;

pub use disk_usage::{get_disk_reports, DiskUsageProbe, PROBE_ID as DISK_USAGE_PROBE_ID};

use lso_core::ProbeResult;

/// Collect probe results from all available sensors synchronously.
pub fn collect_all_probes() -> Vec<ProbeResult> {
    let mut results = Vec::new();
    if let Ok(probe) = DiskUsageProbe::for_host() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        if let Ok(rt) = rt {
            if let Ok(result) = rt.block_on(lso_core::SystemProbe::collect(&probe)) {
                results.push(result);
            }
        }
    }
    results
}
