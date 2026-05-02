# F20 — Security Posture Scanner

**Phase:** 4 — Polish
**Size:** M (1-2 sessions)
**Branch:** `feat/P4-security-scanner`
**Depends on:** F10, F07

## Objective

Scan for common security misconfigurations: open ports, weak permissions, outdated software, firewall gaps.

## Probes

### OpenPortsProbe (`sensor.security.open_ports`)
- Linux: parse `/proc/net/tcp`, `/proc/net/tcp6` or `ss -tlnp`
- macOS: `lsof -iTCP -sTCP:LISTEN`
- Windows: `netstat -an` or `Get-NetTCPConnection`
- Report: port, protocol, PID, process name, bind address (0.0.0.0 vs 127.0.0.1)

### PermissionsProbe (`sensor.security.permissions`)
- World-writable files in sensitive locations
- SUID/SGID binaries (Linux/macOS)
- Overly permissive home directory
- SSH key permissions

### FirewallProbe (`sensor.security.firewall`)
- macOS: `pfctl -sr` (Application Firewall status)
- Linux: `iptables -L` / `nft list ruleset` / `ufw status`
- Windows: `Get-NetFirewallProfile`
- Report: enabled/disabled, default policy, rule count

## Rules
- `security.open_port_unexpected` — port open that isn't in whitelist
- `security.world_writable` — sensitive directory is world-writable
- `security.firewall_disabled` — system firewall not active
- `security.suid_unexpected` — SUID binary outside known set
- `security.ssh_key_permissions` — SSH keys readable by others

## Deliverables

### 1. Security Dashboard Tab
- Security score (0-100 based on findings)
- Open ports table
- Permission issues list
- Firewall status indicator

### 2. Offline CVE Check (stretch goal)
- Ship curated CVE database for common packages
- Match installed versions against known vulnerabilities
- Update via manual download (air-gapped)

## Acceptance Criteria

- [ ] Open ports probe accurate on primary platform
- [ ] Permission issues detected and reported
- [ ] Firewall status correctly read
- [ ] Security score calculated deterministically
- [ ] All findings pass through recommendation → approval pipeline
- [ ] No elevated privileges needed for basic scan
