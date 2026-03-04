use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use omni_core::config::PermissionDefaults;
use omni_core::database::Database;
use omni_core::error::Result;

use crate::capability::Capability;

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny { reason: String },
    Prompt { reason: String, capability: Capability },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionDuration {
    Once,
    Session,
    Always,
}

impl PermissionDuration {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Session => "session",
            Self::Always => "always",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StoredDecision {
    Allow,
    Deny,
}

impl StoredDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(Self::Allow),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    pub id: String,
    pub extension_id: String,
    pub capability: Capability,
    pub decision: StoredDecision,
    pub duration: PermissionDuration,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub use_count: u64,
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum DefaultPolicy {
    Deny,
    Prompt,
}

pub struct PolicyEngine {
    db: Arc<Mutex<Database>>,
    session_cache: RwLock<HashMap<(String, String), StoredDecision>>,
    default_policy: DefaultPolicy,
}

impl PolicyEngine {
    pub fn new(db: Arc<Mutex<Database>>, default_policy: DefaultPolicy) -> Self {
        Self {
            db,
            session_cache: RwLock::new(HashMap::new()),
            default_policy,
        }
    }

    pub fn from_config(db: Arc<Mutex<Database>>, config: &PermissionDefaults) -> Self {
        let default_policy = match config.default_policy.as_str() {
            "prompt" => DefaultPolicy::Prompt,
            _ => DefaultPolicy::Deny,
        };
        Self::new(db, default_policy)
    }

    /// Synchronous permission check using only the session cache.
    /// Used by WASM host functions where async is not available.
    /// Permissions should be pre-cached during extension activation.
    pub fn check_sync(
        &self,
        extension_id: &str,
        requested: &Capability,
    ) -> PermissionDecision {
        let cap_key = requested.capability_key().to_string();
        let key = (extension_id.to_string(), cap_key);
        // Use try_read to avoid blocking -- works both inside and outside tokio runtime
        match self.session_cache.try_read() {
            Ok(cache) => match cache.get(&key) {
                Some(StoredDecision::Allow) => PermissionDecision::Allow,
                Some(StoredDecision::Deny) => PermissionDecision::Deny {
                    reason: "Previously denied by user".to_string(),
                },
                None => self.default_decision(requested),
            },
            Err(_) => {
                // Cache is write-locked, fall back to default policy
                self.default_decision(requested)
            }
        }
    }

    fn default_decision(&self, requested: &Capability) -> PermissionDecision {
        match self.default_policy {
            DefaultPolicy::Deny => PermissionDecision::Deny {
                reason: "No permission policy found".to_string(),
            },
            DefaultPolicy::Prompt => PermissionDecision::Prompt {
                reason: "Extension requires permission".to_string(),
                capability: requested.clone(),
            },
        }
    }

    /// Grant a permission in the session cache only (not persisted to DB).
    ///
    /// Used by the chat agent to pre-approve native tool capabilities so they
    /// aren't blocked by the default deny policy.
    pub async fn grant_session_cache(&self, caller_id: &str, capability: &Capability) {
        let key = (
            caller_id.to_string(),
            capability.capability_key().to_string(),
        );
        let mut cache = self.session_cache.write().await;
        cache.insert(key, StoredDecision::Allow);
    }

    pub async fn check(
        &self,
        extension_id: &str,
        requested: &Capability,
    ) -> PermissionDecision {
        let cap_key = requested.capability_key().to_string();

        // 1. Check session cache (fastest path)
        {
            let cache = self.session_cache.read().await;
            let key = (extension_id.to_string(), cap_key.clone());
            if let Some(decision) = cache.get(&key) {
                return match decision {
                    StoredDecision::Allow => PermissionDecision::Allow,
                    StoredDecision::Deny => PermissionDecision::Deny {
                        reason: "Previously denied by user".to_string(),
                    },
                };
            }
        }

        // 2. Check persistent policy store (SQLite via spawn_blocking)
        let db = self.db.clone();
        let ext_id = extension_id.to_string();
        let cap_key_clone = cap_key.clone();

        let policy_row = tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.get_permission_policy(&ext_id, &cap_key_clone)
        })
        .await
        .unwrap_or(Err(omni_core::error::OmniError::Other(
            "spawn_blocking failed".to_string(),
        )));

        if let Ok(Some(row)) = policy_row {
            let decision = StoredDecision::parse(&row.decision)
                .unwrap_or(StoredDecision::Deny);

            match row.duration.as_str() {
                "once" => {
                    // Consume one-time decision
                    let db = self.db.clone();
                    let policy_id = row.id.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        let db = db.lock().unwrap();
                        db.delete_permission_policy(&policy_id)
                    })
                    .await;
                }
                "session" | "always" => {
                    // Cache for future lookups
                    let mut cache = self.session_cache.write().await;
                    cache.insert(
                        (extension_id.to_string(), cap_key.clone()),
                        decision.clone(),
                    );
                }
                _ => {}
            }

            // Update usage stats
            let db = self.db.clone();
            let policy_id = row.id.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let db = db.lock().unwrap();
                db.update_policy_usage(&policy_id)
            })
            .await;

            return match decision {
                StoredDecision::Allow => PermissionDecision::Allow,
                StoredDecision::Deny => PermissionDecision::Deny {
                    reason: "Previously denied by user".to_string(),
                },
            };
        }

        // 3. Fall back to default policy
        match self.default_policy {
            DefaultPolicy::Deny => PermissionDecision::Deny {
                reason: "No permission policy found; default is deny".to_string(),
            },
            DefaultPolicy::Prompt => PermissionDecision::Prompt {
                reason: "Extension requires permission".to_string(),
                capability: requested.clone(),
            },
        }
    }

    pub async fn record_decision(
        &self,
        extension_id: &str,
        capability: &Capability,
        decision: StoredDecision,
        duration: PermissionDuration,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let cap_key = capability.capability_key().to_string();
        let scope_json = serde_json::to_string(capability).ok();
        let decision_str = decision.as_str().to_string();
        let duration_str = duration.as_str().to_string();
        let ext_id = extension_id.to_string();

        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.insert_permission_policy(
                &id,
                &ext_id,
                &cap_key,
                scope_json.as_deref(),
                &decision_str,
                &duration_str,
            )
        })
        .await
        .unwrap_or(Err(omni_core::error::OmniError::Other(
            "spawn_blocking failed".to_string(),
        )))?;

        // Cache if session or always
        if matches!(duration, PermissionDuration::Session | PermissionDuration::Always) {
            let mut cache = self.session_cache.write().await;
            cache.insert(
                (extension_id.to_string(), capability.capability_key().to_string()),
                decision,
            );
        }

        Ok(())
    }

    pub async fn revoke_all(&self, extension_id: &str) -> Result<u64> {
        let db = self.db.clone();
        let ext_id = extension_id.to_string();
        let count = tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.delete_policies_for_extension(&ext_id)
        })
        .await
        .unwrap_or(Err(omni_core::error::OmniError::Other(
            "spawn_blocking failed".to_string(),
        )))?;

        let mut cache = self.session_cache.write().await;
        cache.retain(|(ext, _), _| ext != extension_id);

        Ok(count)
    }

    pub async fn revoke_everything(&self) -> Result<u64> {
        let db = self.db.clone();
        let count = tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.delete_all_policies()
        })
        .await
        .unwrap_or(Err(omni_core::error::OmniError::Other(
            "spawn_blocking failed".to_string(),
        )))?;

        let mut cache = self.session_cache.write().await;
        cache.clear();

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_engine(default: DefaultPolicy) -> (PolicyEngine, Arc<Mutex<Database>>) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path, "test-key").unwrap();
        let db = Arc::new(Mutex::new(db));
        // Leak tempdir so it doesn't get dropped
        std::mem::forget(dir);
        let engine = PolicyEngine::new(db.clone(), default);
        (engine, db)
    }

    #[tokio::test]
    async fn test_default_deny() {
        let (engine, _db) = test_engine(DefaultPolicy::Deny);
        let cap = Capability::NetworkHttp(None);
        let decision = engine.check("ext-1", &cap).await;
        assert!(matches!(decision, PermissionDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn test_default_prompt() {
        let (engine, _db) = test_engine(DefaultPolicy::Prompt);
        let cap = Capability::NetworkHttp(None);
        let decision = engine.check("ext-1", &cap).await;
        assert!(matches!(decision, PermissionDecision::Prompt { .. }));
    }

    #[tokio::test]
    async fn test_record_and_check_allow() {
        let (engine, _db) = test_engine(DefaultPolicy::Deny);
        let cap = Capability::NetworkHttp(None);

        // Initially denied
        let decision = engine.check("ext-1", &cap).await;
        assert!(matches!(decision, PermissionDecision::Deny { .. }));

        // Record allow with session duration
        engine
            .record_decision("ext-1", &cap, StoredDecision::Allow, PermissionDuration::Session)
            .await
            .unwrap();

        // Now allowed
        let decision = engine.check("ext-1", &cap).await;
        assert!(matches!(decision, PermissionDecision::Allow));
    }

    #[tokio::test]
    async fn test_once_duration_consumed() {
        let (engine, _db) = test_engine(DefaultPolicy::Deny);
        let cap = Capability::ClipboardRead;

        engine
            .record_decision("ext-1", &cap, StoredDecision::Allow, PermissionDuration::Once)
            .await
            .unwrap();

        // First check consumes it
        let decision = engine.check("ext-1", &cap).await;
        assert!(matches!(decision, PermissionDecision::Allow));

        // Second check: policy was deleted, falls back to deny
        let decision = engine.check("ext-1", &cap).await;
        assert!(matches!(decision, PermissionDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn test_revoke_all() {
        let (engine, _db) = test_engine(DefaultPolicy::Deny);
        let cap = Capability::NetworkHttp(None);

        engine
            .record_decision("ext-1", &cap, StoredDecision::Allow, PermissionDuration::Always)
            .await
            .unwrap();

        let decision = engine.check("ext-1", &cap).await;
        assert!(matches!(decision, PermissionDecision::Allow));

        engine.revoke_all("ext-1").await.unwrap();

        let decision = engine.check("ext-1", &cap).await;
        assert!(matches!(decision, PermissionDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn test_revoke_everything() {
        let (engine, _db) = test_engine(DefaultPolicy::Deny);

        engine
            .record_decision(
                "ext-1",
                &Capability::NetworkHttp(None),
                StoredDecision::Allow,
                PermissionDuration::Always,
            )
            .await
            .unwrap();
        engine
            .record_decision(
                "ext-2",
                &Capability::ClipboardRead,
                StoredDecision::Allow,
                PermissionDuration::Always,
            )
            .await
            .unwrap();

        engine.revoke_everything().await.unwrap();

        let d1 = engine.check("ext-1", &Capability::NetworkHttp(None)).await;
        let d2 = engine.check("ext-2", &Capability::ClipboardRead).await;
        assert!(matches!(d1, PermissionDecision::Deny { .. }));
        assert!(matches!(d2, PermissionDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn test_cached_lookup_speed() {
        let (engine, _db) = test_engine(DefaultPolicy::Deny);
        let cap = Capability::NetworkHttp(None);

        engine
            .record_decision("ext-1", &cap, StoredDecision::Allow, PermissionDuration::Session)
            .await
            .unwrap();

        // Warm the cache
        engine.check("ext-1", &cap).await;

        // Measure cached lookup
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = engine.check("ext-1", &cap).await;
        }
        let elapsed = start.elapsed();
        let per_check = elapsed / 1000;

        // Should be well under 0.1ms (100 microseconds) per check
        assert!(
            per_check.as_micros() < 100,
            "Cached check took {:?} per call, expected < 100us",
            per_check
        );
    }

    #[tokio::test]
    async fn test_check_sync_cached() {
        let (engine, _db) = test_engine(DefaultPolicy::Deny);
        let cap = Capability::NetworkHttp(None);

        engine
            .record_decision("ext-1", &cap, StoredDecision::Allow, PermissionDuration::Session)
            .await
            .unwrap();

        // Warm cache via async check
        engine.check("ext-1", &cap).await;

        // Sync check should hit cache
        let decision = engine.check_sync("ext-1", &cap);
        assert!(matches!(decision, PermissionDecision::Allow));
    }

    #[tokio::test]
    async fn test_check_sync_no_cache_defaults() {
        let (engine_deny, _db1) = test_engine(DefaultPolicy::Deny);
        let (engine_prompt, _db2) = test_engine(DefaultPolicy::Prompt);
        let cap = Capability::NetworkHttp(None);

        let decision = engine_deny.check_sync("ext-1", &cap);
        assert!(matches!(decision, PermissionDecision::Deny { .. }));

        let decision = engine_prompt.check_sync("ext-1", &cap);
        assert!(matches!(decision, PermissionDecision::Prompt { .. }));
    }
}
