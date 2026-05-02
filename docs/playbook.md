# Playbook.md — LSO Development Playbook

> How we work together. Patterns for productive vibe coding sessions.

## Session Workflow

### Starting a Session
1. State what you want to work on (feature, bug, exploration)
2. Claude reviews current project state (memory.md, existing code)
3. Agree on scope and deliverable for this session
4. Build incrementally, test as we go

### During a Session
- **"Build it"** — Claude writes the code, explains key decisions inline
- **"Explain first"** — Claude describes the approach, waits for approval, then codes
- **"Options"** — Claude presents 2-3 approaches with trade-offs, you pick
- **"Review this"** — Paste code, Claude reviews for bugs, style, security issues
- **"Spike it"** — Quick prototype to validate an idea, throwaway quality OK

### Ending a Session
- Summarize what was built/changed
- List any open items or decisions deferred
- Update memory.md with new ADRs or status changes

## Communication Shortcuts

Use these to steer Claude quickly:

| Shortcut | Meaning |
|----------|---------|
| **"LGTM"** | Approved, proceed with implementation |
| **"Nope"** | Rejected, explain why or ask for alternatives |
| **"Deeper"** | Need more technical detail on the current topic |
| **"Simpler"** | Over-engineered, simplify the approach |
| **"Ship it"** | Good enough, move to the next task |
| **"Pause"** | Stop coding, let's discuss the design first |
| **"Risk?"** | What could go wrong with this approach? |
| **"Test it"** | Write tests for the current code before moving on |
| **"Refactor"** | Code works but needs cleanup, proceed with refactor |

## Task Sizing

| Size | Description | Session Expectation |
|------|------------|-------------------|
| **XS** | Single function, config change, small fix | Done in one exchange |
| **S** | One module, one probe, one UI component | Done in one session |
| **M** | Feature spanning 2-3 modules | 1-2 sessions |
| **L** | New subsystem or major cross-cutting change | 3-5 sessions, break into smaller tasks |

## Quality Gates

Before marking any feature "done":

- [ ] Code compiles (`cargo build` clean)
- [ ] Clippy passes (`cargo clippy -- -D warnings`)
- [ ] Unit tests pass (`cargo test`)
- [ ] No `unwrap()` in production paths
- [ ] Error messages are human-readable
- [ ] UI states: loading, success, error, empty all handled
- [ ] Platform-specific code behind trait abstraction
- [ ] No hardcoded paths

## Escalation Patterns

When Claude is uncertain:
1. **Unknown OS behavior** → "I'm not certain how macOS handles X. Let me draft two approaches and we can test."
2. **Security-sensitive** → "This touches privilege escalation. Here's my approach — please validate before I proceed."
3. **Architecture-changing** → "This would affect the crate boundary. Proposing as ADR before coding."
4. **Performance-critical** → "This could be a bottleneck. Suggest we benchmark before committing."

## Anti-Patterns to Avoid

- **Gold plating** — Don't add features beyond the session scope
- **Premature abstraction** — Don't abstract until the second concrete use case
- **Config-driven everything** — Not everything needs to be configurable; hardcode sensible defaults
- **Ignoring errors** — Never `let _ = ...` for Results that matter
- **Mega-sessions** — If a session exceeds ~10 exchanges on one feature, checkpoint and reassess
