//! LSO Sensor — read-only system probes for gathering system state.

pub mod cpu;
mod disk_usage;
pub mod file_classifier;
pub mod memory;
mod platform;
pub mod process;
pub mod registry;
pub mod security;
pub mod startup;

pub use cpu::{CpuInfo, CpuProbe, PROBE_ID as CPU_PROBE_ID};
pub use disk_usage::{get_disk_reports, DiskUsageProbe, PROBE_ID as DISK_USAGE_PROBE_ID};
pub use memory::{MemoryInfo, MemoryProbe, PROBE_ID as MEMORY_PROBE_ID};
pub use process::{ProcessInfo, ProcessListProbe, PROBE_ID as PROCESS_PROBE_ID};
pub use file_classifier::{classify_by_extension, FileClassificationProbe};
pub use registry::SensorRegistry;
pub use startup::{StartupProbe, PROBE_ID as STARTUP_PROBE_ID};
pub use security::{
    FirewallProbe, OpenPortsProbe, PermissionsProbe,
    FIREWALL_PROBE_ID, OPEN_PORTS_PROBE_ID, PERMISSIONS_PROBE_ID,
};

use lso_core::{Platform, ProbeResult};

/// Build a fully-configured sensor registry for the host platform.
pub fn build_registry() -> SensorRegistry {
    let mut registry = SensorRegistry::new();
    let platform = Platform::detect().unwrap_or(Platform::Linux);

    registry.register(Box::new(DiskUsageProbe::new(platform)));

    if let Ok(probe) = ProcessListProbe::for_host() {
        registry.register(Box::new(probe));
    }
    if let Ok(probe) = MemoryProbe::for_host() {
        registry.register(Box::new(probe));
    }
    if let Ok(probe) = CpuProbe::for_host() {
        registry.register(Box::new(probe));
    }
    if let Ok(probe) = StartupProbe::for_host() {
        registry.register(Box::new(probe));
    }
    if let Ok(probe) = OpenPortsProbe::for_host() {
        registry.register(Box::new(probe));
    }
    if let Ok(probe) = PermissionsProbe::for_host() {
        registry.register(Box::new(probe));
    }
    if let Ok(probe) = FirewallProbe::for_host() {
        registry.register(Box::new(probe));
    }

    registry
}

/// Collect probe results from all available sensors (async).
pub async fn collect_all_async() -> Vec<ProbeResult> {
    let registry = build_registry();
    registry.collect_all().await
}

/// Collect probe results from all available sensors synchronously.
pub fn collect_all_probes() -> Vec<ProbeResult> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    match rt {
        Ok(rt) => rt.block_on(collect_all_async()),
        Err(_) => Vec::new(),
    }
}
