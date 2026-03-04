CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    metadata TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_call_id TEXT,
    tool_calls TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    token_count INTEGER
);
CREATE INDEX IF NOT EXISTS idx_messages_session
    ON messages(session_id, created_at);

CREATE TABLE IF NOT EXISTS permission_policies (
    id TEXT PRIMARY KEY,
    extension_id TEXT NOT NULL,
    capability TEXT NOT NULL,
    scope TEXT,
    decision TEXT NOT NULL,
    duration TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(extension_id, capability)
);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    event_type TEXT NOT NULL,
    extension_id TEXT,
    capability TEXT,
    decision TEXT,
    details TEXT,
    session_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp
    ON audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_extension
    ON audit_log(extension_id);

CREATE TABLE IF NOT EXISTS extension_state (
    extension_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (extension_id, key)
);

CREATE TABLE IF NOT EXISTS guardian_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    scan_type TEXT NOT NULL,
    layer TEXT NOT NULL,
    result TEXT NOT NULL,
    confidence REAL,
    details TEXT,
    session_id TEXT,
    extension_id TEXT
);
