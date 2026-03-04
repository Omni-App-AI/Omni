-- Extension instances: multiple runtime instances per installed extension
CREATE TABLE IF NOT EXISTS extension_instances (
    instance_id TEXT PRIMARY KEY,
    extension_id TEXT NOT NULL,
    instance_name TEXT NOT NULL,
    display_name TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_ext_instances_ext ON extension_instances(extension_id);
