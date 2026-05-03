# Changelog

All notable changes to LSO (Local System Optimizer) will be documented in this file.

## [0.5.0] - 2026-05-04

### Added
- **Cross-platform support (F21):** macOS and Windows sensor implementations
  - macOS: process list via `proc_listallpids`, memory via `host_statistics64`, CPU via `host_processor_info`
  - macOS: startup items from LaunchAgents/LaunchDaemons (plist scanning)
  - macOS: open ports via `lsof`, firewall via ALF plist + pf status
  - macOS: LaunchAgent disable/enable via `launchctl`
  - Windows: disk via `GetDiskFreeSpaceExW`, process via `CreateToolhelp32Snapshot`
  - Windows: memory via `GlobalMemoryStatusEx`, CPU via `GetSystemTimes`
  - Windows: startup items from registry Run keys
  - Windows: open ports via `netstat`, firewall via `netsh advfirewall`
- **MockPlatform test harness:** `MockPlatform` in `lso-core` for cross-platform unit tests without target OS
- **Per-platform integration tests:** `tests/macos/`, `tests/linux/`, `tests/windows/`, `tests/common/`
- **Packaging (F22):** Tauri build config for dmg, AppImage, deb, msi targets
- **Build script:** `scripts/build-release.sh` with preflight checks and binary size verification
- **Release checklist:** `docs/release-checklist.md`

### Changed
- Sensor dispatch now routes to platform-specific implementations instead of returning `PlatformNotSupported`
- Startup disable actuator supports LaunchAgent (macOS) and registry Run (Windows) in addition to systemd/XDG (Linux)

## [0.4.0] - 2026-05-03

### Added
- Startup item discovery (F19): systemd services + XDG autostart on Linux
- Security scanner (F20): open ports, firewall status, file permissions

## [0.3.0] - 2026-05-02

### Added
- AI explanation service (F16) with Ollama backend
- RAG pipeline (F17) for context-aware explanations
- File classification sensor (F18) with duplicate detection

## [0.2.0] - 2026-05-01

### Added
- Temp file cleanup actuator (F13) with snapshot-before-mutate
- Audit log UI (F14) with filtering and CSV/JSON export
- Process, memory, and CPU probes (F10) with sensor registry

## [0.1.0] - 2026-04-30

### Added
- Initial scaffold: Cargo workspace with 6 crates
- Core types, error hierarchy, platform traits (F03)
- SQLCipher database layer (F04)
- Tauri shell + dashboard UI (F06)
- Deterministic rule engine with TOML schema (F07)
- Recommendation generator and approval gate (F08/F09)
- LLM integration layer with Ollama backend (F15)
