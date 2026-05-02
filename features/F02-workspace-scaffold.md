# F02 — Cargo Workspace + Crate Scaffold

**Phase:** 0 — Foundation
**Size:** S (one session)
**Branch:** `feat/P0-workspace-scaffold`
**Depends on:** F01

## Objective

Create the Cargo workspace with all crate shells. Each crate compiles, has a placeholder lib.rs, and the dependency graph enforces architectural boundaries.

## Steps

1. **Create workspace `Cargo.toml`**
   ```toml
   [workspace]
   resolver = "2"
   members = [
     "crates/lso-core",
     "crates/lso-sensor",
     "crates/lso-engine",
     "crates/lso-actuator",
     "crates/lso-ai",
     "crates/lso-db",
   ]

   [workspace.package]
   version = "0.1.0"
   edition = "2021"
   license = "MIT"
   rust-version = "1.75"

   [workspace.dependencies]
   thiserror = "2"
   anyhow = "1"
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   tokio = { version = "1", features = ["full"] }
   tracing = "0.1"
   tracing-subscriber = "0.3"
   chrono = { version = "0.4", features = ["serde"] }
   uuid = { version = "1", features = ["v4", "serde"] }
   ```

2. **Create each crate** with `cargo init --lib crates/lso-<name>`

3. **Set crate dependencies** enforcing the architectural boundary:
   ```
   lso-core     → (no internal deps — leaf crate)
   lso-db       → lso-core
   lso-sensor   → lso-core
   lso-engine   → lso-core, lso-sensor (types only)
   lso-actuator → lso-core, lso-db
   lso-ai       → lso-core
   ```

   **Key constraint:** `lso-sensor` CANNOT depend on `lso-actuator` and vice versa. Enforced by Cargo.

4. **Each crate's `Cargo.toml`** uses workspace dependencies:
   ```toml
   [package]
   name = "lso-core"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   thiserror.workspace = true
   serde.workspace = true
   ```

5. **Each crate's `lib.rs`** starts with:
   ```rust
   //! LSO <module> — <one-line description>

   #[cfg(test)]
   mod tests {
       #[test]
       fn it_compiles() {
           assert!(true);
       }
   }
   ```

6. **Verify**
   ```bash
   cargo build --workspace
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```

7. **Commit**
   ```bash
   git add -A
   git commit -m "feat: scaffold Cargo workspace with 6 crates"
   ```

## Acceptance Criteria

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` — all placeholder tests pass
- [ ] Dependency graph matches architecture (sensor cannot import actuator)
- [ ] All crates use workspace dependency versions
- [ ] No circular dependencies

## Dependency Graph (enforced by Cargo)

```
        lso-core (leaf)
       /    |     \     \
  lso-db  lso-sensor  lso-ai  lso-engine
                                  |
                             (reads lso-sensor types,
                              NOT lso-sensor itself)
  lso-actuator
      |
   lso-core, lso-db
```
