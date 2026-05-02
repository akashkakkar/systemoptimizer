# Memory.md — LSO Project Memory

> This file tracks architectural decisions, technical choices, and project evolution.
> Update this file as the project progresses.

## Project Overview

- **Name:** LSO (Local System Optimizer)
- **Type:** Desktop application — offline, privacy-first system analyzer and optimizer
- **Platforms:** macOS, Windows, Linux
- **Development Mode:** Vibe coding with Claude as co-developer
- **Status:** Pre-development / Architecture phase

## Architectural Decisions

### ADR-001: Rust + Tauri over Electron
- **Date:** 2026-05-02
- **Decision:** Use Rust core with Tauri for UI instead of Electron/Node.js
- **Rationale:** Smaller binary, lower RAM footprint, native system access via Rust, no bundled Chromium. Tauri uses OS webview.
- **Trade-off:** Steeper learning curve, smaller ecosystem than Electron

### ADR-002: Local AI via llama.cpp
- **Date:** 2026-05-02
- **Decision:** Use llama.cpp (via ollama or direct bindings) for local LLM inference
- **Rationale:** Mature, optimized for CPU/GPU inference, supports quantized models, MIT licensed. No cloud dependency.
- **Model candidates:** Phi-3-mini (3.8B Q4), Llama 3.2 3B (Q4)
- **Trade-off:** AI features require ~3GB RAM overhead. Made optional for low-spec machines.

### ADR-003: AI explains, rules decide
- **Date:** 2026-05-02
- **Decision:** LLM never generates executable commands or makes optimization decisions. Rule engine handles all decisions. LLM provides human-readable explanations only.
- **Rationale:** Eliminates hallucination risk in critical system operations. Deterministic behavior is auditable.

### ADR-004: Permission model — explicit consent per action
- **Date:** 2026-05-02
- **Decision:** Every system-modifying action requires user approval via UI. High-risk actions require double confirmation. No batch "approve all."
- **Rationale:** User trust is paramount. Accidental system damage is unacceptable.

### ADR-005: SQLCipher for local storage
- **Date:** 2026-05-02
- **Decision:** Use SQLite + SQLCipher for all persistent data
- **Rationale:** Encrypted at rest, zero-config, no server process, battle-tested. Key derived from user passphrase or OS keychain.

### ADR-006: Workspace crate structure
- **Date:** 2026-05-02
- **Decision:** Cargo workspace with separate crates: core, sensor, engine, actuator, ai, db
- **Rationale:** Enforces separation of concerns at compile time. Sensor crate literally cannot import actuator code.

## Tech Stack (Confirmed)

| Layer | Technology | Version/Notes |
|-------|-----------|---------------|
| Language (core) | Rust | Edition 2021, stable |
| Language (plugins) | Python | 3.11+ |
| UI framework | Tauri v2 | Rust backend + WebView frontend |
| Frontend | React + TypeScript | Strict TS, Tailwind CSS |
| State management | Zustand | Lightweight |
| Database | SQLite + SQLCipher | Encrypted local storage |
| Local AI | llama.cpp / ollama | Quantized models, optional |
| Vector store | LanceDB | On-disk, for RAG embeddings |
| Async runtime | Tokio | For Rust async |
| Serialization | serde + serde_json | Standard |
| Error handling | thiserror + anyhow | Typed + application errors |

## Platform-Specific Notes

### macOS
- Sensors: `sysctl`, `IOKit`, `launchctl`, `diskutil`, APFS snapshots
- Privilege: `osascript` for elevation prompts, or `launchd` plist for privileged helper
- Notarization required for distribution

### Linux
- Sensors: `/proc`, `/sys`, `systemctl`, `lsblk`, `ss`
- Privilege: `pkexec` / `polkit` for elevation
- Snapshots: Btrfs snapshots where available, tar-based fallback
- Packaging: AppImage primary, .deb/.rpm secondary

### Windows
- Sensors: WMI, `Get-Process`, Registry, `netstat`, Performance Counters
- Privilege: UAC elevation via manifest
- Snapshots: Volume Shadow Copy (VSS)
- Packaging: MSI / MSIX

## Current Phase

**Phase 0 — Project Setup**
- [ ] Initialize Cargo workspace
- [ ] Scaffold Tauri v2 project
- [ ] Set up CI (local, no cloud CI)
- [ ] Create core types and error handling crate
- [ ] First sensor: disk usage probe

## Open Questions

1. Should plugins use WASM sandboxing instead of Python for better security?
2. LanceDB vs ChromaDB for local vector store — benchmark needed
3. Tauri v2 stable readiness — verify cross-platform IPC maturity
4. License choice: MIT vs Apache-2.0 vs AGPL?

## User Preferences (Developer)

- Expert-level technologist, multi-decade experience
- Comfortable with systems programming, OS internals
- Prefers direct, technical communication
- Values security and privacy deeply
- Development style: vibe coding with Claude — iterative, conversational
