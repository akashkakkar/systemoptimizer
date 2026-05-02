# F04 — SQLCipher Database Layer

**Phase:** 0 — Foundation
**Size:** M (1-2 sessions)
**Branch:** `feat/P0-database-layer`
**Depends on:** F03

## Objective

Implement encrypted local storage using SQLite + SQLCipher. Handle schema creation, migrations, and CRUD for core types.

## Key Decisions

- **Crate:** `rusqlite` with `bundled-sqlcipher` feature (bundles SQLCipher, no system dependency)
- **Migrations:** Embedded SQL files, versioned, applied on startup
- **Key derivation:** OS keychain preferred, user passphrase fallback
- **Connection pooling:** Single connection with mutex (SQLite is single-writer anyway)

## Deliverables

### 1. Dependencies (`lso-db/Cargo.toml`)
```toml
[dependencies]
lso-core.path = "../lso-core"
rusqlite = { version = "0.32", features = ["bundled-sqlcipher"] }
```

### 2. Schema v1 (`lso-db/migrations/001_initial.sql`)
```sql
CREATE TABLE IF NOT EXISTS metrics (
    id TEXT PRIMARY KEY,
    probe_id TEXT NOT NULL,
    name TEXT NOT NULL,
    value REAL NOT NULL,
    unit TEXT,
    collected_at TEXT NOT NULL,
    platform TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS recommendations (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    category TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    user_approved INTEGER NOT NULL,
    snapshot_id TEXT,
    result TEXT NOT NULL,
    rollback_available INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
```

### 3. Database Manager (`lso-db/src/lib.rs`)
- `Database::open(path, key)` — open or create encrypted DB
- `Database::migrate()` — apply pending migrations
- `Database::store_metrics(Vec<SystemMetric>)` — batch insert
- `Database::store_recommendation(Recommendation)` — insert/update
- `Database::store_audit_entry(AuditEntry)` — append-only insert
- `Database::get_recent_metrics(probe_id, limit)` — query
- `Database::get_pending_recommendations()` — query

### 4. Migration Runner
- Embed SQL files at compile time (`include_str!`)
- Track applied version in `schema_version` table
- Run unapplied migrations in order on startup
- Wrap each migration in a transaction

## Steps

1. Add `rusqlite` with `bundled-sqlcipher` to workspace deps
2. Create migration SQL files
3. Implement `Database` struct with connection management
4. Implement migration runner
5. Implement CRUD operations with proper error mapping
6. Write integration tests using temp files
7. Test encryption: open with wrong key must fail

## Acceptance Criteria

- [ ] Database creates and opens with encryption key
- [ ] Opening with wrong key returns error (not garbage data)
- [ ] Migrations apply idempotently
- [ ] All CRUD operations work and roundtrip through serde
- [ ] `rusqlite` builds with bundled SQLCipher (no system dep)
- [ ] Integration tests use `tempfile` crate, clean up after
- [ ] No plaintext data on disk (verify with hex editor)
