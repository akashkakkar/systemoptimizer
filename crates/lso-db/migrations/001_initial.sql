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
