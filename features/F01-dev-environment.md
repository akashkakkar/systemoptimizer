# F01 — Development Environment Setup

**Phase:** 0 — Foundation
**Size:** S (one session)
**Branch:** `feat/P0-dev-environment`
**Depends on:** Nothing (first task)

## Objective

Set up the complete development environment, initialize the git repository, and verify all tooling works.

## Prerequisites to Install

### All Platforms
- Rust (stable, via `rustup`): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js 20+ LTS (for Tauri frontend)
- Python 3.11+ (for plugins, later)
- Git 2.40+

### macOS
```bash
xcode-select --install                    # Xcode CLI tools
brew install pkg-config openssl sqlite    # Build deps for SQLCipher
```

### Linux (Ubuntu/Debian)
```bash
sudo apt install build-essential pkg-config libssl-dev libsqlite3-dev \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev
```

### Windows
- Visual Studio Build Tools 2022 (C++ workload)
- WebView2 Runtime (usually pre-installed on Win 11)

### Rust Tooling
```bash
rustup component add clippy rustfmt
cargo install cargo-watch     # Auto-rebuild on save
cargo install cargo-nextest   # Better test runner (optional)
```

### Tauri CLI
```bash
cargo install tauri-cli --version "^2"
```

## Steps

1. **Create GitHub/local repo**
   ```bash
   mkdir lso && cd lso
   git init
   git checkout -b main
   ```

2. **Create `.gitignore`**
   ```
   /target
   /node_modules
   /dist
   *.db
   *.db-journal
   .env
   .DS_Store
   Thumbs.db
   ```

3. **Create `CLAUDE.md`** (copy from project knowledge `claude.md` — Claude Code reads this automatically)

4. **Create `rust-toolchain.toml`**
   ```toml
   [toolchain]
   channel = "stable"
   components = ["clippy", "rustfmt"]
   ```

5. **Create `.rustfmt.toml`**
   ```toml
   edition = "2021"
   max_width = 100
   use_field_init_shorthand = true
   ```

6. **Create `clippy.toml`** (empty initially, add rules as needed)

7. **Verify toolchain**
   ```bash
   rustc --version
   cargo --version
   cargo clippy --version
   node --version
   python3 --version
   cargo tauri --version
   ```

8. **Initial commit**
   ```bash
   git add -A
   git commit -m "chore: initialize repository with toolchain config"
   ```

## Acceptance Criteria

- [ ] `rustc`, `cargo`, `clippy`, `rustfmt` all working
- [ ] `cargo tauri --version` returns v2.x
- [ ] Node.js 20+ installed
- [ ] Git repo initialized with `.gitignore`
- [ ] `CLAUDE.md` in repo root
- [ ] Initial commit on `main`
- [ ] `develop` branch created from `main`

## Notes

- No cloud CI. Testing is local-only per architecture principles.
- `cargo-watch` optional but recommended: `cargo watch -x "clippy -- -D warnings" -x test`
