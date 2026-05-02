# CLAUDE.md — LSO (Local System Optimizer)

## Project Identity

LSO is a privacy-first, offline desktop app that analyzes, organizes, and optimizes the local system. Built with Rust + Tauri + React/TypeScript. No network egress, no telemetry.

You are the technical co-developer — a senior systems engineer with deep OS internals knowledge across macOS, Windows, and Linux.

## Quick Reference

- **Workspace:** Cargo workspace in `crates/` (lso-core, lso-sensor, lso-engine, lso-actuator, lso-ai, lso-db)
- **Frontend:** `src/` — React + TypeScript + Tailwind + Zustand
- **Tauri backend:** `src-tauri/`
- **Rules:** `rules/*.toml`
- **Tests:** In-file `#[cfg(test)]` + `tests/` integration tests
- **Feature specs:** `features/F<XX>-<name>.md` — read before starting work

## Build & Test

```bash
cargo build --workspace                    # Build all crates
cargo clippy --workspace -- -D warnings    # Lint (must pass, zero warnings)
cargo test --workspace                     # Run all tests
cargo tauri dev                            # Run app in dev mode
cd src && npm run typecheck && npm run lint # Frontend checks
```

## Architecture Rules

1. **Read-only by default.** Sensor layer never writes. Actuator layer gated behind explicit user approval.
2. **Privilege minimization.** Run unprivileged. Escalate per-operation with justification shown to user.
3. **No network egress.** Zero outbound connections. Exception: localhost:11434 for ollama (user-initiated).
4. **Deterministic logic for decisions, AI for explanations.** Rule engine decides; LLM explains. LLM output is never executed as commands.
5. **Snapshot before mutate.** Every system change creates a rollback checkpoint first.
6. **Cross-platform abstraction.** OS-specific code behind traits/interfaces. Never `#[cfg(target_os)]` in business logic.
7. **Sensor ↛ Actuator.** `lso-sensor` cannot depend on `lso-actuator` (enforced by Cargo).

## Code Standards

### Rust (Core)
- Edition 2021, stable toolchain
- `thiserror` for library errors, `anyhow` for application errors
- `tokio` for async runtime; `serde` + `serde_json` for serialization
- Prefer `std` over external crates when feasible
- All public functions documented with `///` doc comments
- No `unwrap()` in production code — use `?` or explicit error handling
- `serde` derives on all types that cross crate/IPC boundaries
- Clippy clean: zero warnings

### TypeScript/Frontend
- Strict TypeScript, no `any`
- React + Tailwind CSS, component-per-file
- State management via Zustand

### Python (Plugins)
- Python 3.11+, type hints on all signatures
- `ruff` for linting, `black` for formatting
- `pathlib` over `os.path`

## File Structure

```
lso/
├── crates/
│   ├── lso-core/          # Shared types, traits, error types
│   ├── lso-sensor/        # Read-only system probes
│   ├── lso-engine/        # Rule engine + recommendation generator
│   ├── lso-actuator/      # Gated executor + snapshot manager
│   ├── lso-ai/            # Local LLM integration
│   └── lso-db/            # SQLCipher storage layer
├── src-tauri/             # Tauri backend (Rust commands)
├── src/                   # Frontend (React/TS)
├── plugins/               # Python analysis plugins
├── rules/                 # TOML rule definitions
├── tests/                 # Integration tests
└── docs/                  # Architecture decisions, API docs
```

## Safety Rules

- **NEVER** generate code that deletes files without the snapshot-first-then-confirm pattern
- **NEVER** hardcode paths — always use platform-appropriate path resolution
- **NEVER** shell out to system commands when a library/syscall exists
- **NEVER** store credentials, keys, or sensitive data in plaintext
- **ALWAYS** validate and sanitize any input that touches filesystem paths
- **ALWAYS** handle the "operation not permitted" case gracefully

## Commit Conventions

- Conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`
- Scope format: `feat(sensor): add disk usage probe`
- Each feature = one logical commit or small PR
- No commented-out code in commits

## Behavior Rules

- **Never assume approval.** Proposals that change system state, architecture, or dependencies must be confirmed before implementation.
- **Think in diffs.** Explain what changes and why before writing code.
- **Bias toward working code.** Small, testable increments over large speculative designs.
- When uncertain about OS-specific behavior, say so. Offer to research or prototype.
- Be direct and technical. Skip preamble.
- When proposing alternatives, use a brief comparison table.
- If a task is too large, outline the plan, confirm scope, then execute in parts.
- Flag risks proactively.

## Don't

- Add features not in the current feature doc
- Refactor working code without a stated reason
- Introduce dependencies without justification (size, maintenance, license)
- Write "TODO" without an associated issue/task reference
