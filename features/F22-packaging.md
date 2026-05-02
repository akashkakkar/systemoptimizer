# F22 — Packaging + Distribution

**Phase:** 4 — Polish
**Size:** M (1-2 sessions)
**Branch:** `feat/P4-packaging`
**Depends on:** F21

## Objective

Build distributable packages for macOS, Linux, and Windows. No auto-updater, no telemetry, no cloud.

## Platform Packages

### macOS
- `.dmg` with drag-to-Applications
- Code signing with Apple Developer certificate
- Notarization via `xcrun notarytool`
- Entitlements: filesystem read, network (localhost only)

### Linux
- **Primary:** AppImage (single file, runs anywhere)
- **Secondary:** `.deb` (Debian/Ubuntu), `.rpm` (Fedora/RHEL)
- Desktop file + icon for app launcher
- Optional: Flatpak for sandboxed distribution

### Windows
- **Primary:** MSI installer
- **Secondary:** MSIX for Microsoft Store (future)
- Code signing with EV certificate (optional for initial release)
- Include WebView2 bootstrapper

## Deliverables

### 1. Tauri Build Config (`src-tauri/tauri.conf.json`)
```json
{
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev"
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "appimage", "deb", "msi"],
    "icon": ["icons/icon.png"],
    "identifier": "com.lso.app"
  }
}
```

### 2. Build Script (`scripts/build-release.sh`)
```bash
#!/bin/bash
# Build for current platform
cargo tauri build --release
# Output: src-tauri/target/release/bundle/
```

### 3. Release Checklist
- [ ] Version bumped in Cargo.toml + package.json
- [ ] CHANGELOG.md updated
- [ ] All tests passing on target platform
- [ ] Binary size acceptable (target: < 50MB without AI models)
- [ ] Signed (macOS/Windows)
- [ ] Notarized (macOS)
- [ ] Smoke test on clean machine

### 4. Offline Update Mechanism
- Check hash of downloaded update package
- User manually downloads new version from website
- In-app notice: "You're running v0.3.0. Latest is v0.4.0. Download from lso.dev"
- No auto-download, no phone-home

## Acceptance Criteria

- [ ] macOS .dmg installs and launches on clean machine
- [ ] Linux AppImage runs without installation
- [ ] Windows MSI installs and launches
- [ ] No network connections on startup (verified with Wireshark/Little Snitch)
- [ ] Binary size < 50MB (excluding AI models)
- [ ] App icon appears in system launcher/dock/taskbar
