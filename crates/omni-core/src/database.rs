use std::path::Path;

use rusqlite::{params, Connection};

use crate::error::{OmniError, Result};

const KEYRING_SERVICE: &str = "omni";
const KEYRING_USER: &str = "database-key";

/// Attempt to retrieve or create the database encryption key.
///
/// Tries in order:
/// 1. OS keychain via the `keyring` crate
/// 2. `OMNI_DB_KEY` environment variable
/// 3. File-based key at `<data_dir>/Omni/db.key`
/// 4. Generate new key and persist to both keyring and file
pub fn get_or_create_encryption_key() -> Result<String> {
    // 1. Try OS keychain (best-effort)
    if let Some(key) = try_keyring_get() {
        return Ok(key);
    }

    // 2. Try env var
    if let Ok(key) = std::env::var("OMNI_DB_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // 3. Try file-based key
    let key_path = key_file_path()?;
    if key_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(&key_path) {
            let key = contents.trim().to_string();
            if !key.is_empty() {
                // Also try to store in keyring for next time
                try_keyring_set(&key);
                return Ok(key);
            }
        }
    }

    // 4. Generate new key, persist to BOTH keyring and file
    let key = uuid::Uuid::new_v4().to_string();
    try_keyring_set(&key);
    persist_key_file(&key_path, &key)?;
    Ok(key)
}

fn key_file_path() -> Result<std::path::PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| OmniError::Config("Cannot determine data directory".into()))?;
    let app_name = if cfg!(target_os = "linux") {
        "omni"
    } else {
        "Omni"
    };
    Ok(data_dir.join(app_name).join("db.key"))
}

fn try_keyring_get() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .ok()?
        .get_password()
        .ok()
}

fn try_keyring_set(key: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.set_password(key);
    }
}

fn persist_key_file(path: &std::path::Path, key: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, key)?;
    Ok(())
}

pub struct Database {
    conn: Connection,
}

pub struct Session {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<String>,
}

pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<String>,
    pub created_at: String,
    pub token_count: Option<i64>,
}

pub struct NewMessage {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<String>,
    pub token_count: Option<i64>,
}

pub struct AuditEntry {
    pub event_type: String,
    pub extension_id: Option<String>,
    pub capability: Option<String>,
    pub decision: Option<String>,
    pub details: Option<String>,
    pub session_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct AuditRecord {
    pub id: i64,
    pub timestamp: String,
    pub event_type: String,
    pub extension_id: Option<String>,
    pub capability: Option<String>,
    pub decision: Option<String>,
    pub details: Option<String>,
    pub session_id: Option<String>,
}

pub struct GuardianEventEntry {
    pub scan_type: String,
    pub layer: String,
    pub result: String,
    pub confidence: Option<f64>,
    pub details: Option<String>,
    pub session_id: Option<String>,
    pub extension_id: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct GuardianEventRecord {
    pub id: i64,
    pub timestamp: String,
    pub scan_type: String,
    pub layer: String,
    pub result: String,
    pub confidence: Option<f64>,
    pub details: Option<String>,
    pub session_id: Option<String>,
    pub extension_id: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct GuardianStats {
    pub total_scans: i64,
    pub total_blocked: i64,
    pub total_passed: i64,
    pub blocks_by_layer: Vec<(String, i64)>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChannelInstanceRow {
    pub id: String,
    pub channel_type: String,
    pub instance_id: String,
    pub display_name: Option<String>,
    pub config: Option<String>,
    pub credentials: Option<String>,
    pub auto_connect: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChannelBindingRow {
    pub id: String,
    pub channel_instance: String,
    pub extension_id: String,
    pub peer_filter: Option<String>,
    pub group_filter: Option<String>,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtensionInstanceRow {
    pub instance_id: String,
    pub extension_id: String,
    pub instance_name: String,
    pub display_name: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub struct PermissionPolicyRow {
    pub id: String,
    pub extension_id: String,
    pub capability: String,
    pub scope: Option<String>,
    pub decision: String,
    pub duration: String,
    pub created_at: String,
    pub updated_at: String,
    pub use_count: i64,
    pub last_used: Option<String>,
}

impl Database {
    /// Open or create the database with encryption.
    pub fn open(path: &Path, encryption_key: &str) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.pragma_update(None, "key", encryption_key)?;
        conn.pragma_update(None, "kdf_iter", 256000)?;

        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;

        let current_version: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _migrations",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let migrations: &[&str] = &[
            include_str!("../migrations/001_initial.sql"),
            include_str!("../migrations/002_permission_policy_usage.sql"),
            include_str!("../migrations/003_channel_instances.sql"),
            include_str!("../migrations/004_channel_bindings.sql"),
            include_str!("../migrations/005_extension_instances.sql"),
        ];

        for (i, sql) in migrations.iter().enumerate() {
            let version = (i + 1) as i64;
            if version > current_version {
                self.conn.execute_batch(sql)?;
                self.conn.execute(
                    "INSERT INTO _migrations (version) VALUES (?1)",
                    params![version],
                )?;
            }
        }

        // Run programmatic migration for extension instances (safe to call multiple times)
        if current_version < 5 {
            self.migrate_to_instance_ids()?;
        }

        Ok(())
    }

    // --- Sessions ---

    pub fn create_session(&self, metadata: Option<&str>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO sessions (id, metadata) VALUES (?1, ?2)",
            params![id, metadata],
        )?;
        Ok(id)
    }

    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, updated_at, metadata FROM sessions WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Session {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                metadata: row.get(3)?,
            })
        })?;
        match rows.next() {
            Some(Ok(session)) => Ok(Some(session)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, updated_at, metadata FROM sessions ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                created_at: row.get(1)?,
                updated_at: row.get(2)?,
                metadata: row.get(3)?,
            })
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    // --- Messages ---

    pub fn insert_message(&self, msg: &NewMessage) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO messages (id, session_id, role, content, tool_call_id, tool_calls, token_count) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                msg.session_id,
                msg.role,
                msg.content,
                msg.tool_call_id,
                msg.tool_calls,
                msg.token_count,
            ],
        )?;
        Ok(id)
    }

    pub fn get_messages_for_session(&self, session_id: &str) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, tool_call_id, tool_calls, created_at, token_count \
             FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_call_id: row.get(4)?,
                tool_calls: row.get(5)?,
                created_at: row.get(6)?,
                token_count: row.get(7)?,
            })
        })?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    // --- Audit Log ---

    pub fn log_audit_event(&self, entry: &AuditEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO audit_log (event_type, extension_id, capability, decision, details, session_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.event_type,
                entry.extension_id,
                entry.capability,
                entry.decision,
                entry.details,
                entry.session_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_audit_log(&self, limit: usize) -> Result<Vec<AuditRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, event_type, extension_id, capability, decision, details, session_id \
             FROM audit_log ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                event_type: row.get(2)?,
                extension_id: row.get(3)?,
                capability: row.get(4)?,
                decision: row.get(5)?,
                details: row.get(6)?,
                session_id: row.get(7)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    /// Get the count of sessions in the database.
    pub fn session_count(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
        Ok(count)
    }

    // --- Permission Policies ---

    pub fn insert_permission_policy(
        &self,
        id: &str,
        extension_id: &str,
        capability: &str,
        scope: Option<&str>,
        decision: &str,
        duration: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO permission_policies (id, extension_id, capability, scope, decision, duration) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(extension_id, capability) DO UPDATE SET \
                 decision = excluded.decision, \
                 duration = excluded.duration, \
                 scope = excluded.scope, \
                 updated_at = datetime('now')",
            params![id, extension_id, capability, scope, decision, duration],
        )?;
        Ok(())
    }

    pub fn get_permission_policy(
        &self,
        extension_id: &str,
        capability: &str,
    ) -> Result<Option<PermissionPolicyRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, extension_id, capability, scope, decision, duration, \
                    created_at, updated_at, use_count, last_used \
             FROM permission_policies WHERE extension_id = ?1 AND capability = ?2",
        )?;
        let mut rows = stmt.query_map(params![extension_id, capability], |row| {
            Ok(PermissionPolicyRow {
                id: row.get(0)?,
                extension_id: row.get(1)?,
                capability: row.get(2)?,
                scope: row.get(3)?,
                decision: row.get(4)?,
                duration: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                use_count: row.get(8)?,
                last_used: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(Ok(row)) => Ok(Some(row)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub fn update_policy_usage(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE permission_policies SET use_count = use_count + 1, \
             last_used = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn delete_permission_policy(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM permission_policies WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn delete_policies_for_extension(&self, extension_id: &str) -> Result<u64> {
        let count = self.conn.execute(
            "DELETE FROM permission_policies WHERE extension_id = ?1",
            params![extension_id],
        )?;
        Ok(count as u64)
    }

    pub fn delete_all_policies(&self) -> Result<u64> {
        let count = self
            .conn
            .execute("DELETE FROM permission_policies", [])?;
        Ok(count as u64)
    }

    // --- Filtered Audit Log ---

    pub fn query_audit_log_filtered(
        &self,
        extension_id: Option<&str>,
        capability: Option<&str>,
        decision: Option<&str>,
        start_time: Option<&str>,
        end_time: Option<&str>,
        limit: usize,
    ) -> Result<Vec<AuditRecord>> {
        let mut sql = String::from(
            "SELECT id, timestamp, event_type, extension_id, capability, decision, details, session_id \
             FROM audit_log WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ext) = extension_id {
            sql.push_str(" AND extension_id = ?");
            param_values.push(Box::new(ext.to_string()));
        }
        if let Some(cap) = capability {
            sql.push_str(" AND capability = ?");
            param_values.push(Box::new(cap.to_string()));
        }
        if let Some(dec) = decision {
            sql.push_str(" AND decision = ?");
            param_values.push(Box::new(dec.to_string()));
        }
        if let Some(start) = start_time {
            sql.push_str(" AND timestamp >= ?");
            param_values.push(Box::new(start.to_string()));
        }
        if let Some(end) = end_time {
            sql.push_str(" AND timestamp <= ?");
            param_values.push(Box::new(end.to_string()));
        }

        sql.push_str(" ORDER BY timestamp DESC LIMIT ?");
        param_values.push(Box::new(limit as i64));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                event_type: row.get(2)?,
                extension_id: row.get(3)?,
                capability: row.get(4)?,
                decision: row.get(5)?,
                details: row.get(6)?,
                session_id: row.get(7)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }
    // --- Extension State ---

    pub fn get_extension_state(&self, extension_id: &str, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM extension_state WHERE extension_id = ?1 AND key = ?2",
        )?;
        let mut rows = stmt.query_map(params![extension_id, key], |row| row.get(0))?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn set_extension_state(
        &self,
        extension_id: &str,
        key: &str,
        value: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO extension_state (extension_id, key, value, updated_at) \
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![extension_id, key, value],
        )?;
        Ok(())
    }

    pub fn delete_extension_state_key(&self, extension_id: &str, key: &str) -> Result<bool> {
        let count = self.conn.execute(
            "DELETE FROM extension_state WHERE extension_id = ?1 AND key = ?2",
            params![extension_id, key],
        )?;
        Ok(count > 0)
    }

    pub fn delete_extension_state(&self, extension_id: &str) -> Result<u64> {
        let count = self.conn.execute(
            "DELETE FROM extension_state WHERE extension_id = ?1",
            params![extension_id],
        )?;
        Ok(count as u64)
    }

    // --- Channel Instances ---

    pub fn upsert_channel_instance(
        &self,
        channel_type: &str,
        instance_id: &str,
        display_name: Option<&str>,
        config: Option<&str>,
        credentials: Option<&str>,
        auto_connect: bool,
    ) -> Result<String> {
        let id = format!("{}:{}", channel_type, instance_id);
        self.conn.execute(
            "INSERT INTO channel_instances (id, channel_type, instance_id, display_name, config, credentials, auto_connect) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(channel_type, instance_id) DO UPDATE SET \
                 display_name = excluded.display_name, \
                 config = excluded.config, \
                 credentials = excluded.credentials, \
                 auto_connect = excluded.auto_connect, \
                 updated_at = datetime('now')",
            params![id, channel_type, instance_id, display_name, config, credentials, auto_connect as i32],
        )?;
        Ok(id)
    }

    pub fn get_channel_instance(
        &self,
        channel_type: &str,
        instance_id: &str,
    ) -> Result<Option<ChannelInstanceRow>> {
        let id = format!("{}:{}", channel_type, instance_id);
        let mut stmt = self.conn.prepare(
            "SELECT id, channel_type, instance_id, display_name, config, credentials, auto_connect, created_at, updated_at \
             FROM channel_instances WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(ChannelInstanceRow {
                id: row.get(0)?,
                channel_type: row.get(1)?,
                instance_id: row.get(2)?,
                display_name: row.get(3)?,
                config: row.get(4)?,
                credentials: row.get(5)?,
                auto_connect: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(Ok(row)) => Ok(Some(row)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub fn list_channel_instances(&self) -> Result<Vec<ChannelInstanceRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, channel_type, instance_id, display_name, config, credentials, auto_connect, created_at, updated_at \
             FROM channel_instances ORDER BY channel_type, instance_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ChannelInstanceRow {
                id: row.get(0)?,
                channel_type: row.get(1)?,
                instance_id: row.get(2)?,
                display_name: row.get(3)?,
                config: row.get(4)?,
                credentials: row.get(5)?,
                auto_connect: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn list_channel_instances_by_type(
        &self,
        channel_type: &str,
    ) -> Result<Vec<ChannelInstanceRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, channel_type, instance_id, display_name, config, credentials, auto_connect, created_at, updated_at \
             FROM channel_instances WHERE channel_type = ?1 ORDER BY instance_id",
        )?;
        let rows = stmt.query_map(params![channel_type], |row| {
            Ok(ChannelInstanceRow {
                id: row.get(0)?,
                channel_type: row.get(1)?,
                instance_id: row.get(2)?,
                display_name: row.get(3)?,
                config: row.get(4)?,
                credentials: row.get(5)?,
                auto_connect: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn delete_channel_instance(
        &self,
        channel_type: &str,
        instance_id: &str,
    ) -> Result<bool> {
        let id = format!("{}:{}", channel_type, instance_id);
        let count = self.conn.execute(
            "DELETE FROM channel_instances WHERE id = ?1",
            params![id],
        )?;
        Ok(count > 0)
    }

    // --- Channel Bindings ---

    pub fn upsert_binding(
        &self,
        id: &str,
        channel_instance: &str,
        extension_id: &str,
        peer_filter: Option<&str>,
        group_filter: Option<&str>,
        priority: i32,
        enabled: bool,
    ) -> Result<String> {
        self.conn.execute(
            "INSERT INTO channel_bindings (id, channel_instance, extension_id, peer_filter, group_filter, priority, enabled) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(id) DO UPDATE SET \
                 channel_instance = excluded.channel_instance, \
                 extension_id = excluded.extension_id, \
                 peer_filter = excluded.peer_filter, \
                 group_filter = excluded.group_filter, \
                 priority = excluded.priority, \
                 enabled = excluded.enabled, \
                 updated_at = datetime('now')",
            params![id, channel_instance, extension_id, peer_filter, group_filter, priority, enabled as i32],
        )?;
        Ok(id.to_string())
    }

    pub fn list_bindings(&self) -> Result<Vec<ChannelBindingRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, channel_instance, extension_id, peer_filter, group_filter, priority, enabled, created_at, updated_at \
             FROM channel_bindings ORDER BY priority DESC, channel_instance",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ChannelBindingRow {
                id: row.get(0)?,
                channel_instance: row.get(1)?,
                extension_id: row.get(2)?,
                peer_filter: row.get(3)?,
                group_filter: row.get(4)?,
                priority: row.get(5)?,
                enabled: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn list_bindings_for_extension(
        &self,
        extension_id: &str,
    ) -> Result<Vec<ChannelBindingRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, channel_instance, extension_id, peer_filter, group_filter, priority, enabled, created_at, updated_at \
             FROM channel_bindings WHERE extension_id = ?1 ORDER BY priority DESC",
        )?;
        let rows = stmt.query_map(params![extension_id], |row| {
            Ok(ChannelBindingRow {
                id: row.get(0)?,
                channel_instance: row.get(1)?,
                extension_id: row.get(2)?,
                peer_filter: row.get(3)?,
                group_filter: row.get(4)?,
                priority: row.get(5)?,
                enabled: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn delete_binding(&self, id: &str) -> Result<bool> {
        let count = self.conn.execute(
            "DELETE FROM channel_bindings WHERE id = ?1",
            params![id],
        )?;
        Ok(count > 0)
    }

    // --- Guardian Events ---

    pub fn log_guardian_event(&self, entry: &GuardianEventEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO guardian_events (scan_type, layer, result, confidence, details, session_id, extension_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.scan_type,
                entry.layer,
                entry.result,
                entry.confidence,
                entry.details,
                entry.session_id,
                entry.extension_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_guardian_events(&self, limit: usize) -> Result<Vec<GuardianEventRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, scan_type, layer, result, confidence, details, session_id, extension_id \
             FROM guardian_events ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(GuardianEventRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                scan_type: row.get(2)?,
                layer: row.get(3)?,
                result: row.get(4)?,
                confidence: row.get(5)?,
                details: row.get(6)?,
                session_id: row.get(7)?,
                extension_id: row.get(8)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn get_guardian_stats(&self) -> Result<GuardianStats> {
        let total: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM guardian_events",
            [],
            |r| r.get(0),
        )?;

        let blocked: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM guardian_events WHERE result = 'blocked'",
            [],
            |r| r.get(0),
        )?;

        let passed: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM guardian_events WHERE result = 'passed'",
            [],
            |r| r.get(0),
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT layer, COUNT(*) FROM guardian_events WHERE result = 'blocked' GROUP BY layer",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut blocks_by_layer = Vec::new();
        for row in rows {
            blocks_by_layer.push(row?);
        }

        Ok(GuardianStats {
            total_scans: total,
            total_blocked: blocked,
            total_passed: passed,
            blocks_by_layer,
        })
    }

    // --- Extension Instances ---

    pub fn create_extension_instance(
        &self,
        instance_id: &str,
        extension_id: &str,
        instance_name: &str,
        display_name: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO extension_instances (instance_id, extension_id, instance_name, display_name) \
             VALUES (?1, ?2, ?3, ?4)",
            params![instance_id, extension_id, instance_name, display_name],
        )?;
        Ok(())
    }

    pub fn update_extension_instance(
        &self,
        instance_id: &str,
        display_name: Option<&str>,
        enabled: bool,
    ) -> Result<bool> {
        let count = self.conn.execute(
            "UPDATE extension_instances SET display_name = ?2, enabled = ?3, updated_at = datetime('now') \
             WHERE instance_id = ?1",
            params![instance_id, display_name, enabled as i32],
        )?;
        Ok(count > 0)
    }

    pub fn list_extension_instances(&self) -> Result<Vec<ExtensionInstanceRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT instance_id, extension_id, instance_name, display_name, enabled, created_at, updated_at \
             FROM extension_instances ORDER BY extension_id, instance_name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ExtensionInstanceRow {
                instance_id: row.get(0)?,
                extension_id: row.get(1)?,
                instance_name: row.get(2)?,
                display_name: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn list_extension_instances_for(
        &self,
        extension_id: &str,
    ) -> Result<Vec<ExtensionInstanceRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT instance_id, extension_id, instance_name, display_name, enabled, created_at, updated_at \
             FROM extension_instances WHERE extension_id = ?1 ORDER BY instance_name",
        )?;
        let rows = stmt.query_map(params![extension_id], |row| {
            Ok(ExtensionInstanceRow {
                instance_id: row.get(0)?,
                extension_id: row.get(1)?,
                instance_name: row.get(2)?,
                display_name: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn delete_extension_instance(&self, instance_id: &str) -> Result<bool> {
        let count = self.conn.execute(
            "DELETE FROM extension_instances WHERE instance_id = ?1",
            params![instance_id],
        )?;
        Ok(count > 0)
    }

    pub fn delete_extension_instances_for(&self, extension_id: &str) -> Result<u64> {
        let count = self.conn.execute(
            "DELETE FROM extension_instances WHERE extension_id = ?1",
            params![extension_id],
        )?;
        Ok(count as u64)
    }

    /// Run the programmatic part of migration 005: migrate existing extension_state
    /// and channel_bindings rows to use `::default` instance_id format.
    /// Safe to call multiple times -- skips rows already containing `::`.
    pub fn migrate_to_instance_ids(&self) -> Result<()> {
        // 1. Find distinct extension_ids in extension_state that don't have `::`
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT extension_id FROM extension_state WHERE extension_id NOT LIKE '%::%'",
        )?;
        let ext_ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        for ext_id in &ext_ids {
            let instance_id = format!("{}::default", ext_id);
            // Create default instance row (ignore if already exists)
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO extension_instances (instance_id, extension_id, instance_name) \
                 VALUES (?1, ?2, 'default')",
                params![instance_id, ext_id],
            );
            // Update extension_state rows
            self.conn.execute(
                "UPDATE extension_state SET extension_id = ?1 WHERE extension_id = ?2",
                params![instance_id, ext_id],
            )?;
        }

        // 2. Migrate channel_bindings.extension_id to ::default
        let mut stmt2 = self.conn.prepare(
            "SELECT DISTINCT extension_id FROM channel_bindings WHERE extension_id NOT LIKE '%::%'",
        )?;
        let binding_ext_ids: Vec<String> = stmt2
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        for ext_id in &binding_ext_ids {
            let instance_id = format!("{}::default", ext_id);
            // Create default instance row (ignore if already exists)
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO extension_instances (instance_id, extension_id, instance_name) \
                 VALUES (?1, ?2, 'default')",
                params![instance_id, ext_id],
            );
            self.conn.execute(
                "UPDATE channel_bindings SET extension_id = ?1 WHERE extension_id = ?2",
                params![instance_id, ext_id],
            )?;
        }

        Ok(())
    }

    pub fn list_extension_state_keys(&self, extension_id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT key FROM extension_state WHERE extension_id = ?1 ORDER BY key",
        )?;
        let rows = stmt.query_map(params![extension_id], |row| row.get(0))?;
        let mut keys = Vec::new();
        for row in rows {
            keys.push(row?);
        }
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_db() -> (Database, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path, "test-key-123").unwrap();
        (db, dir)
    }

    #[test]
    fn test_create_and_open_encrypted_db() {
        let (db, _dir) = test_db();
        // Verify tables exist by querying them
        let count = db.session_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_wrong_key_fails() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create with one key
        let _db = Database::open(&db_path, "correct-key").unwrap();
        drop(_db);

        // Open with wrong key - should fail when trying to use the database
        let db2 = Database::open(&db_path, "wrong-key");
        assert!(db2.is_err());
    }

    #[test]
    fn test_session_crud() {
        let (db, _dir) = test_db();

        let id = db.create_session(Some(r#"{"name": "test"}"#)).unwrap();
        assert!(!id.is_empty());

        let session = db.get_session(&id).unwrap().unwrap();
        assert_eq!(session.id, id);
        assert_eq!(session.metadata.as_deref(), Some(r#"{"name": "test"}"#));

        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn test_message_crud() {
        let (db, _dir) = test_db();
        let session_id = db.create_session(None).unwrap();

        let msg_id = db
            .insert_message(&NewMessage {
                session_id: session_id.clone(),
                role: "user".to_string(),
                content: "Hello, world!".to_string(),
                tool_call_id: None,
                tool_calls: None,
                token_count: Some(3),
            })
            .unwrap();
        assert!(!msg_id.is_empty());

        let messages = db.get_messages_for_session(&session_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello, world!");
        assert_eq!(messages[0].token_count, Some(3));
    }

    #[test]
    fn test_audit_log() {
        let (db, _dir) = test_db();

        db.log_audit_event(&AuditEntry {
            event_type: "permission_check".to_string(),
            extension_id: Some("ext-1".to_string()),
            capability: Some("fs:read".to_string()),
            decision: Some("allow".to_string()),
            details: None,
            session_id: None,
        })
        .unwrap();

        let records = db.get_audit_log(10).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].event_type, "permission_check");
        assert_eq!(records[0].extension_id.as_deref(), Some("ext-1"));
    }

    #[test]
    fn test_migrations_idempotent() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Open and run migrations
        let db = Database::open(&db_path, "test-key").unwrap();
        db.create_session(None).unwrap();
        drop(db);

        // Open again - migrations should run without error
        let db = Database::open(&db_path, "test-key").unwrap();
        let count = db.session_count().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_extension_state_set_and_get() {
        let (db, _dir) = test_db();
        db.set_extension_state("ext-1", "api_key", "secret123")
            .unwrap();
        let val = db.get_extension_state("ext-1", "api_key").unwrap();
        assert_eq!(val, Some("secret123".to_string()));
    }

    #[test]
    fn test_extension_state_get_nonexistent() {
        let (db, _dir) = test_db();
        let val = db.get_extension_state("ext-1", "missing").unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn test_extension_state_overwrite() {
        let (db, _dir) = test_db();
        db.set_extension_state("ext-1", "key", "value1").unwrap();
        db.set_extension_state("ext-1", "key", "value2").unwrap();
        let val = db.get_extension_state("ext-1", "key").unwrap();
        assert_eq!(val, Some("value2".to_string()));
    }

    #[test]
    fn test_extension_state_delete_key() {
        let (db, _dir) = test_db();
        db.set_extension_state("ext-1", "key", "value").unwrap();
        let deleted = db.delete_extension_state_key("ext-1", "key").unwrap();
        assert!(deleted);
        let val = db.get_extension_state("ext-1", "key").unwrap();
        assert_eq!(val, None);

        let deleted_again = db.delete_extension_state_key("ext-1", "key").unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_extension_state_delete_all() {
        let (db, _dir) = test_db();
        db.set_extension_state("ext-1", "k1", "v1").unwrap();
        db.set_extension_state("ext-1", "k2", "v2").unwrap();
        db.set_extension_state("ext-2", "k1", "v1").unwrap();

        let count = db.delete_extension_state("ext-1").unwrap();
        assert_eq!(count, 2);

        // ext-2 should still have its data
        let val = db.get_extension_state("ext-2", "k1").unwrap();
        assert_eq!(val, Some("v1".to_string()));
    }

    #[test]
    fn test_extension_state_list_keys() {
        let (db, _dir) = test_db();
        db.set_extension_state("ext-1", "beta", "v").unwrap();
        db.set_extension_state("ext-1", "alpha", "v").unwrap();
        db.set_extension_state("ext-1", "gamma", "v").unwrap();

        let keys = db.list_extension_state_keys("ext-1").unwrap();
        assert_eq!(keys, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_guardian_event_log_and_retrieve() {
        let (db, _dir) = test_db();

        db.log_guardian_event(&GuardianEventEntry {
            scan_type: "input".to_string(),
            layer: "signature".to_string(),
            result: "blocked".to_string(),
            confidence: Some(0.95),
            details: Some("SIG-001 matched".to_string()),
            session_id: None,
            extension_id: None,
        })
        .unwrap();

        db.log_guardian_event(&GuardianEventEntry {
            scan_type: "input".to_string(),
            layer: "signature".to_string(),
            result: "passed".to_string(),
            confidence: None,
            details: None,
            session_id: None,
            extension_id: None,
        })
        .unwrap();

        let events = db.get_guardian_events(10).unwrap();
        assert_eq!(events.len(), 2);
        // Verify both events exist
        let results: Vec<&str> = events.iter().map(|e| e.result.as_str()).collect();
        assert!(results.contains(&"blocked"));
        assert!(results.contains(&"passed"));
        let blocked_event = events.iter().find(|e| e.result == "blocked").unwrap();
        assert_eq!(blocked_event.confidence, Some(0.95));
    }

    #[test]
    fn test_channel_instance_upsert_and_get() {
        let (db, _dir) = test_db();
        let id = db
            .upsert_channel_instance("discord", "production", Some("Discord Prod"), None, None, true)
            .unwrap();
        assert_eq!(id, "discord:production");

        let row = db.get_channel_instance("discord", "production").unwrap().unwrap();
        assert_eq!(row.channel_type, "discord");
        assert_eq!(row.instance_id, "production");
        assert_eq!(row.display_name.as_deref(), Some("Discord Prod"));
        assert!(row.auto_connect);
    }

    #[test]
    fn test_channel_instance_upsert_updates() {
        let (db, _dir) = test_db();
        db.upsert_channel_instance("discord", "prod", Some("V1"), None, None, false)
            .unwrap();
        db.upsert_channel_instance("discord", "prod", Some("V2"), None, None, true)
            .unwrap();

        let row = db.get_channel_instance("discord", "prod").unwrap().unwrap();
        assert_eq!(row.display_name.as_deref(), Some("V2"));
        assert!(row.auto_connect);
    }

    #[test]
    fn test_channel_instance_list_all() {
        let (db, _dir) = test_db();
        db.upsert_channel_instance("discord", "prod", None, None, None, false).unwrap();
        db.upsert_channel_instance("discord", "staging", None, None, None, false).unwrap();
        db.upsert_channel_instance("twitter", "brand-a", None, None, None, false).unwrap();

        let all = db.list_channel_instances().unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].channel_type, "discord");
    }

    #[test]
    fn test_channel_instance_list_by_type() {
        let (db, _dir) = test_db();
        db.upsert_channel_instance("discord", "prod", None, None, None, false).unwrap();
        db.upsert_channel_instance("discord", "staging", None, None, None, false).unwrap();
        db.upsert_channel_instance("twitter", "brand-a", None, None, None, false).unwrap();

        let discord = db.list_channel_instances_by_type("discord").unwrap();
        assert_eq!(discord.len(), 2);

        let twitter = db.list_channel_instances_by_type("twitter").unwrap();
        assert_eq!(twitter.len(), 1);
    }

    #[test]
    fn test_channel_instance_delete() {
        let (db, _dir) = test_db();
        db.upsert_channel_instance("discord", "prod", None, None, None, false).unwrap();

        let deleted = db.delete_channel_instance("discord", "prod").unwrap();
        assert!(deleted);

        let row = db.get_channel_instance("discord", "prod").unwrap();
        assert!(row.is_none());

        let deleted_again = db.delete_channel_instance("discord", "prod").unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_channel_instance_get_nonexistent() {
        let (db, _dir) = test_db();
        let row = db.get_channel_instance("discord", "nope").unwrap();
        assert!(row.is_none());
    }

    #[test]
    fn test_guardian_stats() {
        let (db, _dir) = test_db();

        // Log some events
        for _ in 0..3 {
            db.log_guardian_event(&GuardianEventEntry {
                scan_type: "input".to_string(),
                layer: "signature".to_string(),
                result: "passed".to_string(),
                confidence: None,
                details: None,
                session_id: None,
                extension_id: None,
            })
            .unwrap();
        }

        db.log_guardian_event(&GuardianEventEntry {
            scan_type: "input".to_string(),
            layer: "signature".to_string(),
            result: "blocked".to_string(),
            confidence: Some(0.95),
            details: None,
            session_id: None,
            extension_id: None,
        })
        .unwrap();

        db.log_guardian_event(&GuardianEventEntry {
            scan_type: "output_chunk".to_string(),
            layer: "heuristic".to_string(),
            result: "blocked".to_string(),
            confidence: Some(0.80),
            details: None,
            session_id: None,
            extension_id: None,
        })
        .unwrap();

        let stats = db.get_guardian_stats().unwrap();
        assert_eq!(stats.total_scans, 5);
        assert_eq!(stats.total_blocked, 2);
        assert_eq!(stats.total_passed, 3);
        assert_eq!(stats.blocks_by_layer.len(), 2);
    }

    // --- Channel Binding tests ---

    #[test]
    fn test_binding_upsert_and_list() {
        let (db, _dir) = test_db();
        let id = db
            .upsert_binding("b1", "discord:prod", "ext-a", None, None, 10, true)
            .unwrap();
        assert_eq!(id, "b1");

        let bindings = db.list_bindings().unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].channel_instance, "discord:prod");
        assert_eq!(bindings[0].extension_id, "ext-a");
        assert_eq!(bindings[0].priority, 10);
        assert!(bindings[0].enabled);
    }

    #[test]
    fn test_binding_upsert_updates() {
        let (db, _dir) = test_db();
        db.upsert_binding("b1", "discord:prod", "ext-a", None, None, 5, true)
            .unwrap();
        db.upsert_binding("b1", "discord:staging", "ext-b", Some("admin-*"), None, 20, false)
            .unwrap();

        let bindings = db.list_bindings().unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].channel_instance, "discord:staging");
        assert_eq!(bindings[0].extension_id, "ext-b");
        assert_eq!(bindings[0].peer_filter.as_deref(), Some("admin-*"));
        assert_eq!(bindings[0].priority, 20);
        assert!(!bindings[0].enabled);
    }

    #[test]
    fn test_binding_list_for_extension() {
        let (db, _dir) = test_db();
        db.upsert_binding("b1", "discord:prod", "ext-a", None, None, 10, true).unwrap();
        db.upsert_binding("b2", "twitter:brand-a", "ext-a", None, None, 5, true).unwrap();
        db.upsert_binding("b3", "discord:prod", "ext-b", None, None, 10, true).unwrap();

        let ext_a = db.list_bindings_for_extension("ext-a").unwrap();
        assert_eq!(ext_a.len(), 2);

        let ext_b = db.list_bindings_for_extension("ext-b").unwrap();
        assert_eq!(ext_b.len(), 1);

        let ext_c = db.list_bindings_for_extension("ext-c").unwrap();
        assert_eq!(ext_c.len(), 0);
    }

    #[test]
    fn test_binding_delete() {
        let (db, _dir) = test_db();
        db.upsert_binding("b1", "discord:prod", "ext-a", None, None, 10, true).unwrap();

        let deleted = db.delete_binding("b1").unwrap();
        assert!(deleted);

        let bindings = db.list_bindings().unwrap();
        assert!(bindings.is_empty());

        let deleted_again = db.delete_binding("b1").unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_binding_with_filters() {
        let (db, _dir) = test_db();
        db.upsert_binding(
            "b1", "discord:prod", "ext-a",
            Some("admin-*"), Some("support-*"), 10, true,
        ).unwrap();

        let bindings = db.list_bindings().unwrap();
        assert_eq!(bindings[0].peer_filter.as_deref(), Some("admin-*"));
        assert_eq!(bindings[0].group_filter.as_deref(), Some("support-*"));
    }

    // --- Extension Instance tests ---

    #[test]
    fn test_extension_instance_create_and_list() {
        let (db, _dir) = test_db();
        db.create_extension_instance(
            "com.example.ext::bot-a",
            "com.example.ext",
            "bot-a",
            Some("Bot A"),
        )
        .unwrap();
        db.create_extension_instance(
            "com.example.ext::bot-b",
            "com.example.ext",
            "bot-b",
            None,
        )
        .unwrap();

        let all = db.list_extension_instances().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].instance_id, "com.example.ext::bot-a");
        assert_eq!(all[0].extension_id, "com.example.ext");
        assert_eq!(all[0].instance_name, "bot-a");
        assert_eq!(all[0].display_name.as_deref(), Some("Bot A"));
        assert!(all[0].enabled);

        assert_eq!(all[1].instance_id, "com.example.ext::bot-b");
        assert!(all[1].display_name.is_none());
    }

    #[test]
    fn test_extension_instance_list_for_extension() {
        let (db, _dir) = test_db();
        db.create_extension_instance("ext-a::default", "ext-a", "default", None).unwrap();
        db.create_extension_instance("ext-a::custom", "ext-a", "custom", None).unwrap();
        db.create_extension_instance("ext-b::default", "ext-b", "default", None).unwrap();

        let ext_a = db.list_extension_instances_for("ext-a").unwrap();
        assert_eq!(ext_a.len(), 2);

        let ext_b = db.list_extension_instances_for("ext-b").unwrap();
        assert_eq!(ext_b.len(), 1);

        let ext_c = db.list_extension_instances_for("ext-c").unwrap();
        assert_eq!(ext_c.len(), 0);
    }

    #[test]
    fn test_extension_instance_update() {
        let (db, _dir) = test_db();
        db.create_extension_instance("ext::inst", "ext", "inst", None).unwrap();

        let updated = db
            .update_extension_instance("ext::inst", Some("New Name"), false)
            .unwrap();
        assert!(updated);

        let instances = db.list_extension_instances().unwrap();
        assert_eq!(instances[0].display_name.as_deref(), Some("New Name"));
        assert!(!instances[0].enabled);
    }

    #[test]
    fn test_extension_instance_update_nonexistent() {
        let (db, _dir) = test_db();
        let updated = db
            .update_extension_instance("ext::nope", Some("X"), true)
            .unwrap();
        assert!(!updated);
    }

    #[test]
    fn test_extension_instance_delete() {
        let (db, _dir) = test_db();
        db.create_extension_instance("ext::inst", "ext", "inst", None).unwrap();

        let deleted = db.delete_extension_instance("ext::inst").unwrap();
        assert!(deleted);

        let all = db.list_extension_instances().unwrap();
        assert!(all.is_empty());

        let deleted_again = db.delete_extension_instance("ext::inst").unwrap();
        assert!(!deleted_again);
    }

    #[test]
    fn test_extension_instance_delete_for_extension() {
        let (db, _dir) = test_db();
        db.create_extension_instance("ext::a", "ext", "a", None).unwrap();
        db.create_extension_instance("ext::b", "ext", "b", None).unwrap();
        db.create_extension_instance("other::x", "other", "x", None).unwrap();

        let count = db.delete_extension_instances_for("ext").unwrap();
        assert_eq!(count, 2);

        let all = db.list_extension_instances().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].extension_id, "other");
    }

    #[test]
    fn test_extension_instance_duplicate_rejected() {
        let (db, _dir) = test_db();
        db.create_extension_instance("ext::inst", "ext", "inst", None).unwrap();

        let result = db.create_extension_instance("ext::inst", "ext", "inst", None);
        assert!(result.is_err()); // UNIQUE constraint on instance_id
    }

    #[test]
    fn test_migrate_to_instance_ids() {
        let (db, _dir) = test_db();

        // Seed some extension_state rows with bare extension_ids
        db.set_extension_state("ext-a", "key1", "val1").unwrap();
        db.set_extension_state("ext-a", "key2", "val2").unwrap();
        db.set_extension_state("ext-b", "key3", "val3").unwrap();

        // Seed some channel_bindings with bare extension_ids
        db.upsert_binding("b1", "discord:prod", "ext-a", None, None, 0, true).unwrap();
        db.upsert_binding("b2", "twitter:main", "ext-b", None, None, 0, true).unwrap();

        // Run migration
        db.migrate_to_instance_ids().unwrap();

        // Verify extension_state rows were migrated
        let val = db.get_extension_state("ext-a::default", "key1").unwrap();
        assert_eq!(val.as_deref(), Some("val1"));
        let val = db.get_extension_state("ext-a::default", "key2").unwrap();
        assert_eq!(val.as_deref(), Some("val2"));
        let val = db.get_extension_state("ext-b::default", "key3").unwrap();
        assert_eq!(val.as_deref(), Some("val3"));

        // Old bare keys should no longer exist
        let val = db.get_extension_state("ext-a", "key1").unwrap();
        assert!(val.is_none());

        // Verify channel_bindings were migrated
        let bindings = db.list_bindings().unwrap();
        assert_eq!(bindings.len(), 2);
        assert!(bindings.iter().all(|b| b.extension_id.contains("::")));
        assert!(bindings.iter().any(|b| b.extension_id == "ext-a::default"));
        assert!(bindings.iter().any(|b| b.extension_id == "ext-b::default"));

        // Verify default instance rows were created
        let instances = db.list_extension_instances().unwrap();
        assert!(instances.iter().any(|i| i.instance_id == "ext-a::default"));
        assert!(instances.iter().any(|i| i.instance_id == "ext-b::default"));
    }

    #[test]
    fn test_migrate_to_instance_ids_idempotent() {
        let (db, _dir) = test_db();

        // Set up state and run migration
        db.set_extension_state("ext-a", "k", "v").unwrap();
        db.migrate_to_instance_ids().unwrap();

        // Run again -- should be a no-op
        db.migrate_to_instance_ids().unwrap();

        // Still only one instance row
        let instances = db.list_extension_instances_for("ext-a").unwrap();
        assert_eq!(instances.len(), 1);

        // State is still accessible
        let val = db.get_extension_state("ext-a::default", "k").unwrap();
        assert_eq!(val.as_deref(), Some("v"));
    }

    #[test]
    fn test_migrate_skips_already_migrated() {
        let (db, _dir) = test_db();

        // Pre-create an instance with :: already
        db.create_extension_instance("ext::custom", "ext", "custom", None).unwrap();
        db.set_extension_state("ext::custom", "k", "v").unwrap();

        // Run migration -- should not touch rows already containing ::
        db.migrate_to_instance_ids().unwrap();

        // The custom instance state should be untouched
        let val = db.get_extension_state("ext::custom", "k").unwrap();
        assert_eq!(val.as_deref(), Some("v"));
    }
}
