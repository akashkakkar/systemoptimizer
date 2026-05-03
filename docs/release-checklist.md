# Release Checklist

## Pre-release

- [ ] Version bumped in `Cargo.toml` workspace and `src-tauri/tauri.conf.json`
- [ ] Version bumped in `package.json`
- [ ] `CHANGELOG.md` updated with release notes
- [ ] All tests passing: `cargo test --workspace`
- [ ] Clippy clean: `cargo clippy --workspace -- -D warnings`
- [ ] Frontend checks: `npm run typecheck && npm run lint`
- [ ] No `PlatformNotSupported` errors for supported features

## Build

- [ ] Run `./scripts/build-release.sh` on each target platform
- [ ] Binary size < 50MB (excluding AI models)
- [ ] No network connections on startup (verify with Little Snitch / Wireshark / `lsof -i`)

## Platform Verification

### macOS
- [ ] `.dmg` installs via drag-to-Applications
- [ ] App launches from Applications folder
- [ ] App icon appears in Dock
- [ ] All sensors return data (disk, process, memory, CPU, startup, security)
- [ ] Recommendations generate and display
- [ ] Approval flow works (approve/reject/execute)
- [ ] Code signed with Apple Developer certificate (when available)
- [ ] Notarized via `xcrun notarytool` (when available)

### Linux
- [ ] AppImage runs without installation (`chmod +x && ./LSO.AppImage`)
- [ ] `.deb` installs on Debian/Ubuntu
- [ ] Desktop file + icon appear in app launcher
- [ ] All sensors return data
- [ ] Systemd startup item disable/rollback works

### Windows
- [ ] MSI installer completes
- [ ] App launches from Start Menu
- [ ] WebView2 bootstrapper installs if needed
- [ ] All sensors return data
- [ ] Registry startup item management works

## Post-release

- [ ] Git tag created: `git tag -a v<VERSION> -m "Release v<VERSION>"`
- [ ] Tag pushed: `git push origin v<VERSION>`
- [ ] Release artifacts uploaded to distribution channel
- [ ] Smoke test on clean machine (no dev tools installed)
