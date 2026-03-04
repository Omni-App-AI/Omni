-- Channel-to-Extension Bindings
CREATE TABLE IF NOT EXISTS channel_bindings (
    id TEXT PRIMARY KEY,
    channel_instance TEXT NOT NULL,
    extension_id TEXT NOT NULL,
    peer_filter TEXT,
    group_filter TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_channel_bindings_channel ON channel_bindings(channel_instance);
CREATE INDEX IF NOT EXISTS idx_channel_bindings_extension ON channel_bindings(extension_id);
