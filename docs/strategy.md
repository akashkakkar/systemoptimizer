# LSO Development Strategy

> Master document governing how this project is built, session by session.

## Claude Code vs Claude Desktop — Decision

**Use Claude Code (CLI) as the primary development tool.**

| Capability | Claude Code | Claude Desktop |
|---|---|---|
| Run `cargo build/test/clippy` | Yes | No |
| Git operations (commit, branch, push) | Yes | No |
| File creation and editing | Yes | Yes |
| Multi-file refactors | Yes (direct) | Yes (artifacts) |
| Architecture discussions | Adequate | Better |
| Read project context | From filesystem | From project knowledge |
| Run Tauri dev server | Yes | No |
| Install dependencies | Yes | No |

**Workflow:** Use Claude Code for all implementation. Use Claude Desktop (this project) for architecture discussions, planning, and reviewing strategy docs.

**Claude Code setup:** Point it at the LSO repo root. It reads `claude.md` from the repo as its project instructions automatically (CLAUDE.md convention).

---

## Git Strategy

### Branching Model

```
main (stable, always builds)
 └── develop (integration branch)
      ├── feat/P0-workspace-scaffold
      ├── feat/P0-core-types
      ├── feat/P0-disk-sensor
      ├── feat/P1-rule-engine
      └── ...
```

### Workflow Per Feature

1. Create branch from `develop`: `git checkout -b feat/<phase>-<feature-name>`
2. Develop iteratively in Claude Code session
3. Run quality gates (build, clippy, test)
4. Commit with conventional commit message
5. Merge to `develop`: `git merge --no-ff feat/<phase>-<feature-name>`
6. After phase completion, merge `develop` → `main` with version tag
7. Delete feature branch

### Versioning

SemVer: `0.x.y` throughout pre-release.
- `0.1.0` — Phase 0 complete (workspace, core types, first sensor)
- `0.2.0` — Phase 1 complete (rule engine, recommendations)
- `0.3.0` — Phase 2 complete (actuator, snapshots)
- `0.4.0` — Phase 3 complete (local AI)
- `0.5.0` — Phase 4 (cross-platform parity)
- `1.0.0` — First public release

### Commit Convention

```
feat(sensor): add disk usage probe for Linux
fix(engine): handle empty rule set without panic
refactor(core): extract platform trait to separate module
test(actuator): add snapshot creation test with tempdir
docs(memory): record ADR-007 for plugin sandboxing
chore: update Cargo.lock
```

---

## Development Phases

### Phase 0 — Foundation (Features F01–F06)
Scaffold the project, establish patterns, get first end-to-end data flow working.

| Feature | Scope | Doc |
|---|---|---|
| F01 | Dev environment + repo init | `features/F01-dev-environment.md` |
| F02 | Cargo workspace + crate scaffold | `features/F02-workspace-scaffold.md` |
| F03 | Core types, errors, platform traits | `features/F03-core-types.md` |
| F04 | SQLCipher database layer | `features/F04-database-layer.md` |
| F05 | Disk usage sensor (first probe) | `features/F05-disk-sensor.md` |
| F06 | Tauri shell + dashboard UI | `features/F06-tauri-shell.md` |

### Phase 1 — Intelligence (Features F07–F10)
Rule engine, recommendations, approval flow.

| Feature | Scope | Doc |
|---|---|---|
| F07 | Rule engine + TOML schema | `features/F07-rule-engine.md` |
| F08 | Recommendation generator | `features/F08-recommendations.md` |
| F09 | Approval gate UI | `features/F09-approval-gate.md` |
| F10 | Process + memory sensors | `features/F10-system-sensors.md` |

### Phase 2 — Action (Features F11–F14)
Snapshot, execute, rollback.

| Feature | Scope | Doc |
|---|---|---|
| F11 | Snapshot manager (per-OS) | `features/F11-snapshot-manager.md` |
| F12 | Actuator framework | `features/F12-actuator.md` |
| F13 | First actuator: temp file cleanup | `features/F13-cleanup-actuator.md` |
| F14 | Audit log + history UI | `features/F14-audit-log.md` |

### Phase 3 — AI (Features F15–F18)
Local LLM integration for explanations and classification.

| Feature | Scope | Doc |
|---|---|---|
| F15 | llama.cpp integration layer | `features/F15-llm-integration.md` |
| F16 | Explanation generator | `features/F16-explanations.md` |
| F17 | RAG pipeline + vector store | `features/F17-rag-pipeline.md` |
| F18 | File classification sensor | `features/F18-file-classifier.md` |

### Phase 4 — Polish (Features F19–F22)
Cross-platform, packaging, UX.

| Feature | Scope | Doc |
|---|---|---|
| F19 | Startup item sensor + actuator | `features/F19-startup-items.md` |
| F20 | Security posture scanner | `features/F20-security-scanner.md` |
| F21 | Cross-platform test matrix | `features/F21-platform-testing.md` |
| F22 | Packaging + distribution | `features/F22-packaging.md` |

---

## Session Protocol

### Starting a Feature

```
1. Open new Claude Code session
2. Read the feature doc: features/F<XX>-<name>.md
3. Create feature branch
4. Implement per acceptance criteria in the doc
5. Run quality gates
6. Commit + push
7. Merge to develop
```

### Claude Code Session Template

Paste this to start each feature session:

```
I'm working on LSO (Local System Optimizer).
Read CLAUDE.md for project context.
Current task: Feature F<XX> — <name>
Read features/F<XX>-<name>.md for requirements.
Branch: feat/P<X>-<feature-name>
Let's start.
```

### Quality Gates (run before every merge)

```bash
cargo build --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
# Frontend (when applicable)
cd src && npm run typecheck && npm run lint
```

---

## Risk Registry

| Risk | Impact | Mitigation | Owner |
|---|---|---|---|
| Tauri v2 breaking changes | Medium | Pin version, test on all 3 OS early | F06 |
| SQLCipher FFI complexity | High | Spike in F04, fallback to rusqlite if needed | F04 |
| llama.cpp build complexity | Medium | Use ollama as fallback, defer to Phase 3 | F15 |
| Cross-platform sensor divergence | High | Abstract early (F03), test matrix (F21) | F03/F21 |
| Scope creep | High | Feature docs are contracts, defer extras | All |

---

## Gap Remediation

Items identified as missing from project knowledge, now addressed:

| Gap | Resolution | Where |
|---|---|---|
| Dev environment setup | Feature F01 doc | `F01-dev-environment.md` |
| Git branching strategy | This document | Strategy §Git |
| Versioning scheme | This document | Strategy §Versioning |
| Database migrations | Feature F04 doc | `F04-database-layer.md` |
| Rule engine TOML schema | Feature F07 doc | `F07-rule-engine.md` |
| Plugin API contract | Deferred to Phase 5 | Tracked in memory.md |
| Performance targets | Each feature doc | Per-feature acceptance criteria |
| Accessibility | Feature F06 doc | `F06-tauri-shell.md` |
| Packaging/distribution | Feature F22 doc | `F22-packaging.md` |
| Internal logging | Feature F03 doc | `F03-core-types.md` (tracing crate) |
| Feature flags | Cargo feature flags | Per-crate in workspace |
| Testing infrastructure | Feature F01 doc | `F01-dev-environment.md` |
