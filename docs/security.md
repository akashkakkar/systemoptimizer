# Security.md — LSO Threat Model & Security Design

## Threat Model

### What We're Protecting
- User's system integrity (no accidental damage)
- User's data privacy (no exfiltration, no telemetry)
- User's trust (transparent actions, no surprises)

### Trust Boundaries

```
┌─────────────────────────────────────────┐
│             USER TRUST ZONE             │
│  ┌──────────┐  ┌──────────┐            │
│  │   UI     │  │  Vault   │            │
│  │ (WebView)│  │(SQLCipher)│            │
│  └────┬─────┘  └──────────┘            │
│       │ IPC (Tauri commands)            │
│  ┌────▼──────────────────────────┐      │
│  │     APPLICATION SANDBOX       │      │
│  │  ┌────────┐  ┌────────────┐   │      │
│  │  │ Sensor │  │   Engine   │   │      │
│  │  │(unpriv)│  │(rule eval) │   │      │
│  │  └────────┘  └────────────┘   │      │
│  │  ┌────────┐  ┌────────────┐   │      │
│  │  │Actuator│  │  AI (LLM)  │   │      │
│  │  │(gated) │  │ (isolated) │   │      │
│  │  └────────┘  └────────────┘   │      │
│  └───────────────────────────────┘      │
│                                         │
│  ┌───────────────────────────────┐      │
│  │   OS PRIVILEGE BOUNDARY       │      │
│  │  (elevation per-operation)    │      │
│  └───────────────────────────────┘      │
└─────────────────────────────────────────┘

OUTSIDE: Network (blocked), Cloud (nonexistent)
```

### Threat Categories

| Threat | Mitigation |
|--------|-----------|
| Accidental system damage | Snapshot before every mutation; approval gates |
| Data exfiltration | No network egress; firewall rules; no DNS |
| Malicious plugin | Python plugins sandboxed; no filesystem write access; no network |
| LLM prompt injection | LLM has no access to actuator; output is text-only, never executed |
| Privilege escalation | Per-operation elevation; no persistent root/admin |
| Vault tampering | SQLCipher encryption; integrity checks on open |
| Supply chain | Minimal dependencies; audit lockfiles; vendor critical crates |
| Path traversal | All paths canonicalized and validated against allowed roots |

## Data Handling

### What LSO Stores
- System metrics snapshots (CPU, RAM, disk, processes)
- Analysis results and recommendations
- User approval/rejection history (audit log)
- Rule definitions and custom configurations
- AI conversation context (ephemeral, purged on session end)

### What LSO Never Stores
- File contents (only metadata: name, size, type, modified date)
- Passwords, credentials, tokens
- Browser history, cookies, personal documents
- Network traffic content

### Encryption
- Database: SQLCipher (AES-256-CBC, PBKDF2 key derivation)
- Key source: OS keychain (macOS Keychain, Windows Credential Store, Linux Secret Service) or user passphrase
- Memory: Sensitive data zeroized on drop (`zeroize` crate)

## Plugin Security

Python plugins run in a restricted environment:
- No network access (socket module blocked)
- No filesystem writes (read-only access to specific paths)
- No subprocess/os.system calls
- Resource limits: CPU time, memory ceiling
- Plugin manifest declares required permissions; user approves on install

## Audit Log

Every action is logged to the vault:
```
{
  "timestamp": "ISO-8601",
  "action": "disable_startup_item",
  "target": "com.example.autoupdater",
  "risk_level": "medium",
  "user_approved": true,
  "snapshot_id": "snap-20260502-001",
  "result": "success",
  "rollback_available": true
}
```

Audit log is append-only within the application. User can export/clear via UI.
