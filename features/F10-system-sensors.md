# F10 — Process + Memory + CPU Sensors

**Phase:** 1 — Intelligence
**Size:** M (1-2 sessions)
**Branch:** `feat/P1-system-sensors`
**Depends on:** F05 (pattern established)

## Objective

Add process list, memory usage, and CPU load probes. Expand the sensor registry pattern.

## Probes

### ProcessListProbe (`sensor.process.list`)
- PID, name, CPU %, memory %, status, user, start time
- Linux: parse `/proc/[pid]/stat` and `/proc/[pid]/status`
- macOS: `libproc` / `sysctl` KERN_PROC
- Windows: `CreateToolhelp32Snapshot` or WMI

### MemoryProbe (`sensor.memory.usage`)
- Total, used, available, swap total, swap used
- Linux: `/proc/meminfo`
- macOS: `host_statistics64` (Mach API)
- Windows: `GlobalMemoryStatusEx`

### CpuProbe (`sensor.cpu.load`)
- Per-core usage %, load average (1/5/15 min on Unix)
- Core count, model name
- Linux: `/proc/stat`, `/proc/cpuinfo`
- macOS: `host_processor_info`
- Windows: Performance Counters

## Sensor Registry Pattern

```rust
pub struct SensorRegistry {
    probes: Vec<Box<dyn SystemProbe>>,
}

impl SensorRegistry {
    pub fn new(platform: Platform) -> Self;
    pub fn register(&mut self, probe: Box<dyn SystemProbe>);
    pub async fn collect_all(&self) -> Vec<ProbeResult>;
}
```

## New Rules (add to `rules/`)
- `process.high_cpu` — process using > 80% CPU for > 5 minutes
- `memory.usage_high` — RAM > 90% used
- `memory.swap_active` — swap usage > 50% (indicates memory pressure)

## Acceptance Criteria

- [ ] All three probes return accurate data on primary dev platform
- [ ] Sensor registry collects all probes in parallel (tokio::join)
- [ ] Results store in DB with correct probe IDs
- [ ] Dashboard shows process list, memory bar, CPU load
- [ ] New rules trigger recommendations correctly
- [ ] Other platforms stubbed with `PlatformNotSupported`
