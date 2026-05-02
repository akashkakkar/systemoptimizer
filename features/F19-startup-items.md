# F19 — Startup Item Sensor + Actuator

**Phase:** 4 — Polish
**Size:** M (1-2 sessions)
**Branch:** `feat/P4-startup-items`
**Depends on:** F10, F12

## Objective

Sensor to discover startup/login items. Actuator to disable unnecessary ones (with approval).

## Platform Implementations

### macOS
- **Sensor:** `launchctl list`, Login Items (SMAppService), LaunchAgents/Daemons plist files
- **Actuator:** `launchctl unload`, remove from Login Items

### Linux
- **Sensor:** `systemctl list-unit-files --type=service --state=enabled`, `~/.config/autostart/`
- **Actuator:** `systemctl disable`, remove desktop file

### Windows
- **Sensor:** Registry `HKCU\...\Run`, Task Scheduler, Services
- **Actuator:** Registry key removal, disable scheduled task

## Deliverables

### 1. StartupProbe
- List all startup items with: name, type, command, enabled status, publisher
- Flag unknown/unsigned items
- Estimate boot time impact (if measurable)

### 2. StartupDisableExecutor
- Risk level: Medium (snapshot + single confirm)
- Preflight: verify item exists and is currently enabled
- Execute: disable via platform-appropriate method
- Verify: confirm item no longer in startup list
- Rollback: re-enable from snapshot data

### 3. Rules
- `startup.unknown_publisher` — startup item from unknown publisher
- `startup.high_count` — more than 15 startup items
- `startup.duplicate` — same app registered multiple times

### 4. UI
- Startup items list with enable/disable toggles
- Publisher info and trust indicators
- "Impact" column (low/medium/high based on resource usage)

## Acceptance Criteria

- [ ] Lists all startup items on primary dev platform
- [ ] Disable/enable through approval gate
- [ ] Snapshot before disable
- [ ] Rollback re-enables correctly
- [ ] Unknown publishers flagged
- [ ] Other platforms stubbed
