# Glossary.md — LSO Terminology & Conventions

## Core Concepts

| Term | Definition |
|------|-----------|
| **Sensor** | Read-only probe that collects system data. Never writes or modifies anything. |
| **Probe** | A single sensor function (e.g., disk usage probe, process list probe). |
| **Rule** | Deterministic condition → recommendation mapping. Defined in TOML. |
| **Recommendation** | A proposed action with risk score, explanation, and rollback plan. |
| **Approval Gate** | UI checkpoint where user explicitly confirms or rejects a recommendation. |
| **Actuator** | Executor that performs approved system changes. Cannot run without approval gate clearance. |
| **Snapshot** | OS-level checkpoint (APFS/Btrfs/VSS) created before any mutation. |
| **Rollback** | Reverting to a prior snapshot after a failed or unwanted change. |
| **Vault** | Encrypted SQLCipher database storing all LSO data locally. |
| **Plugin** | Python-based extension that adds custom analysis capabilities. |
| **Risk Score** | Rating (Low/Medium/High/Critical) assigned to each recommendation. |

## Risk Levels

| Level | Meaning | Approval Type |
|-------|---------|--------------|
| **Low** | Reversible, no data loss risk (e.g., clearing temp files) | Single confirm |
| **Medium** | Affects system config, easily reversible (e.g., disabling startup item) | Single confirm + snapshot |
| **High** | Affects system behavior, requires snapshot (e.g., modifying service config) | Double confirm + snapshot |
| **Critical** | Could affect boot/stability (e.g., kernel params) | Blocked by default, expert mode only |

## Naming Conventions

### Crates
- `lso-<module>` — e.g., `lso-core`, `lso-sensor`, `lso-engine`

### Modules/Files
- Snake case: `disk_usage.rs`, `process_monitor.rs`
- Traits: PascalCase, prefixed by purpose: `SystemProbe`, `ActionExecutor`
- Structs: PascalCase, descriptive: `DiskUsageReport`, `RecommendationEntry`

### Frontend
- Components: PascalCase files: `DashboardView.tsx`, `ApprovalDialog.tsx`
- Hooks: `use` prefix: `useSystemMetrics.ts`, `useApprovalFlow.ts`
- Stores: `<name>Store.ts`: `metricsStore.ts`, `settingsStore.ts`

### Rules (TOML)
- One file per category: `disk.toml`, `security.toml`, `startup.toml`
- Rule IDs: `category.specific_check` — e.g., `disk.tmp_overflow`, `security.open_ports`

### Tauri Commands
- Verb-noun: `get_disk_usage`, `list_startup_items`, `execute_recommendation`
- Always `async`, always return `Result<T, String>`
