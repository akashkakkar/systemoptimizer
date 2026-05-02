# Skills.md — LSO Development Skills & Capabilities

> Defines the technical skills and domain knowledge Claude brings to this project.
> Used as a reference to ensure consistent, high-quality contributions across sessions.

## Core Development Skills

### Rust Systems Programming
- Cargo workspace management, feature flags, conditional compilation
- Async programming with Tokio (tasks, channels, select, graceful shutdown)
- FFI for C library integration (llama.cpp, SQLCipher, OS APIs)
- Memory safety patterns: lifetimes, borrowing, Arc/Mutex for shared state
- Cross-compilation and platform-conditional code (`#[cfg(target_os)]`)
- Trait-based abstraction for platform-agnostic interfaces
- Error handling chains: `thiserror` for library errors, `anyhow` for applications
- Performance profiling: `flamegraph`, `criterion` benchmarks

### Tauri v2 Application Development
- Rust command handlers (IPC between frontend and backend)
- Window management, system tray, native menus
- Plugin system (tauri-plugin-*)
- Security: CSP configuration, capability-based permissions
- Auto-updater disabled by design (offline-first)
- Multi-window patterns for approval dialogs

### Frontend (React + TypeScript)
- Functional components, hooks, custom hooks
- Zustand state management with middleware
- Tailwind CSS utility-first styling
- Accessible UI patterns (ARIA, keyboard navigation)
- Data visualization: charts for system metrics (Recharts or D3)
- Responsive layout for varied screen sizes

### Python Plugin Development
- Plugin architecture: discovery, loading, sandboxed execution
- System probing: `psutil`, `shutil`, `pathlib`
- Data processing: `pandas` for log/metric analysis
- Type-safe interfaces between Rust host and Python plugins

## OS & Systems Knowledge

### macOS
- `launchd` / `launchctl` for service management
- APFS snapshot creation and management
- IOKit for hardware info
- Keychain access for credential storage
- `sysctl` for kernel parameters
- Gatekeeper, notarization, entitlements

### Linux
- `/proc` and `/sys` filesystem parsing
- `systemd` unit management and journal querying
- Btrfs/LVM snapshot mechanics
- `polkit` / `pkexec` for privilege escalation
- Cgroup and namespace awareness
- Package manager integration (apt, dnf, pacman — read-only queries)

### Windows
- WMI queries for system info
- Registry reading (HKLM, HKCU)
- Volume Shadow Copy Service (VSS)
- Windows Event Log parsing
- UAC elevation patterns
- Task Scheduler inspection
- Performance Counters and ETW

## AI & ML Integration

### Local LLM
- `llama.cpp` integration: model loading, tokenization, inference
- GGUF model format, quantization levels (Q4_K_M, Q5_K_S, etc.)
- Prompt engineering for system analysis explanations
- Context window management for large system reports
- Streaming inference for responsive UI

### RAG Pipeline
- Document embedding (system docs, man pages, rule descriptions)
- Vector similarity search via LanceDB
- Chunk sizing and overlap strategies
- Prompt construction with retrieved context

## Security & Privacy

- Encryption at rest: SQLCipher integration, key derivation (Argon2)
- Principle of least privilege in application design
- Input validation and path traversal prevention
- Secure memory handling for sensitive data (zeroize crate)
- Audit logging of all system-modifying actions

## Testing & Quality

- Unit testing in Rust (`#[cfg(test)]`, mockall for mocking)
- Integration testing with temp directories and mock system state
- Snapshot testing for UI components
- Property-based testing with `proptest` for edge cases
- Platform-specific test matrices

## Development Workflow

- Iterative vibe coding: discuss → design → implement → test → refine
- Small, focused increments over big-bang features
- Each session should produce runnable, testable code
- Documentation written alongside code, not after
