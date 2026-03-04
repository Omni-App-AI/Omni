CREATE TABLE IF NOT EXISTS channel_instances (
    id TEXT PRIMARY KEY,
    channel_type TEXT NOT NULL,
    instance_id TEXT NOT NULL,
    display_name TEXT,
    config TEXT,
    credentials TEXT,
    auto_connect INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(channel_type, instance_id)
);
CREATE INDEX IF NOT EXISTS idx_channel_instances_type
    ON channel_instances(channel_type);
