use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

/// Manages provider rotation and fallback with exponential backoff.
///
/// Inspired by OpenClaw's `AuthRoundRobin` system, this supports
/// multiple credentials per provider with automatic rotation and
/// cooldown on failures.
pub struct ProviderRotation {
    providers: Vec<ProviderEntry>,
    current: AtomicUsize,
    cooldowns: RwLock<HashMap<String, CooldownState>>,
}

#[derive(Debug, Clone)]
pub struct ProviderEntry {
    pub provider_id: String,
    pub credential_id: String,
    pub priority: u32,
}

#[derive(Debug, Clone)]
struct CooldownState {
    failed_at: DateTime<Utc>,
    cooldown_until: DateTime<Utc>,
    consecutive_failures: u32,
}

impl ProviderRotation {
    /// Create a new rotation with the given provider entries.
    /// Entries are sorted by priority (lower priority number = higher precedence).
    pub fn new(mut providers: Vec<ProviderEntry>) -> Self {
        providers.sort_by_key(|p| p.priority);
        Self {
            providers,
            current: AtomicUsize::new(0),
            cooldowns: RwLock::new(HashMap::new()),
        }
    }

    /// Get the next available provider, skipping those on cooldown.
    /// Returns None if all providers are on cooldown.
    pub async fn next(&self) -> Option<(&str, &str)> {
        if self.providers.is_empty() {
            return None;
        }

        let cooldowns = self.cooldowns.read().await;
        let now = Utc::now();

        for _ in 0..self.providers.len() {
            let idx = self.current.fetch_add(1, Ordering::Relaxed) % self.providers.len();
            let entry = &self.providers[idx];

            let key = format!("{}.{}", entry.provider_id, entry.credential_id);
            if let Some(cd) = cooldowns.get(&key) {
                if now < cd.cooldown_until {
                    continue; // Skip -- still on cooldown
                }
            }

            return Some((&entry.provider_id, &entry.credential_id));
        }

        None // All providers on cooldown
    }

    /// Report a failure for a provider (triggers cooldown with exponential backoff).
    pub async fn report_failure(&self, provider_id: &str, credential_id: &str) {
        let key = format!("{}.{}", provider_id, credential_id);
        let mut cooldowns = self.cooldowns.write().await;

        let entry = cooldowns.entry(key).or_insert(CooldownState {
            failed_at: Utc::now(),
            cooldown_until: Utc::now(),
            consecutive_failures: 0,
        });

        entry.consecutive_failures += 1;
        entry.failed_at = Utc::now();

        // Exponential backoff: 5s, 15s, 60s, 300s
        let cooldown_secs = match entry.consecutive_failures {
            1 => 5,
            2 => 15,
            3 => 60,
            _ => 300,
        };
        entry.cooldown_until = Utc::now() + chrono::Duration::seconds(cooldown_secs);
    }

    /// Report success for a provider (resets cooldown).
    pub async fn report_success(&self, provider_id: &str, credential_id: &str) {
        let key = format!("{}.{}", provider_id, credential_id);
        self.cooldowns.write().await.remove(&key);
    }

    /// Get the number of configured providers.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Check if there are no configured providers.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Get the number of providers currently on cooldown.
    pub async fn cooldown_count(&self) -> usize {
        let cooldowns = self.cooldowns.read().await;
        let now = Utc::now();
        cooldowns
            .values()
            .filter(|cd| now < cd.cooldown_until)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entries() -> Vec<ProviderEntry> {
        vec![
            ProviderEntry {
                provider_id: "openai".to_string(),
                credential_id: "key1".to_string(),
                priority: 1,
            },
            ProviderEntry {
                provider_id: "anthropic".to_string(),
                credential_id: "key1".to_string(),
                priority: 2,
            },
            ProviderEntry {
                provider_id: "openai".to_string(),
                credential_id: "key2".to_string(),
                priority: 3,
            },
        ]
    }

    #[tokio::test]
    async fn test_rotation_basic() {
        let rotation = ProviderRotation::new(test_entries());
        assert_eq!(rotation.len(), 3);
        assert!(!rotation.is_empty());

        let (p1, c1) = rotation.next().await.unwrap();
        assert_eq!(p1, "openai");
        assert_eq!(c1, "key1");

        let (p2, c2) = rotation.next().await.unwrap();
        assert_eq!(p2, "anthropic");
        assert_eq!(c2, "key1");

        let (p3, c3) = rotation.next().await.unwrap();
        assert_eq!(p3, "openai");
        assert_eq!(c3, "key2");

        // Wraps around
        let (p4, _) = rotation.next().await.unwrap();
        assert_eq!(p4, "openai");
    }

    #[tokio::test]
    async fn test_rotation_empty() {
        let rotation = ProviderRotation::new(vec![]);
        assert!(rotation.is_empty());
        assert!(rotation.next().await.is_none());
    }

    #[tokio::test]
    async fn test_rotation_failure_cooldown() {
        let rotation = ProviderRotation::new(vec![
            ProviderEntry {
                provider_id: "openai".to_string(),
                credential_id: "key1".to_string(),
                priority: 1,
            },
            ProviderEntry {
                provider_id: "anthropic".to_string(),
                credential_id: "key1".to_string(),
                priority: 2,
            },
        ]);

        // Fail the first provider
        rotation.report_failure("openai", "key1").await;
        assert_eq!(rotation.cooldown_count().await, 1);

        // Next should skip openai and return anthropic
        let (provider, _) = rotation.next().await.unwrap();
        assert_eq!(provider, "anthropic");
    }

    #[tokio::test]
    async fn test_rotation_success_resets_cooldown() {
        let rotation = ProviderRotation::new(vec![ProviderEntry {
            provider_id: "openai".to_string(),
            credential_id: "key1".to_string(),
            priority: 1,
        }]);

        rotation.report_failure("openai", "key1").await;
        assert_eq!(rotation.cooldown_count().await, 1);

        rotation.report_success("openai", "key1").await;
        assert_eq!(rotation.cooldown_count().await, 0);
    }

    #[tokio::test]
    async fn test_all_providers_on_cooldown() {
        let rotation = ProviderRotation::new(vec![ProviderEntry {
            provider_id: "openai".to_string(),
            credential_id: "key1".to_string(),
            priority: 1,
        }]);

        rotation.report_failure("openai", "key1").await;
        assert!(rotation.next().await.is_none());
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let rotation = ProviderRotation::new(vec![
            ProviderEntry {
                provider_id: "low-priority".to_string(),
                credential_id: "key1".to_string(),
                priority: 10,
            },
            ProviderEntry {
                provider_id: "high-priority".to_string(),
                credential_id: "key1".to_string(),
                priority: 1,
            },
        ]);

        let (p, _) = rotation.next().await.unwrap();
        assert_eq!(p, "high-priority"); // sorted by priority
    }

    #[tokio::test]
    async fn test_exponential_backoff() {
        let rotation = ProviderRotation::new(vec![ProviderEntry {
            provider_id: "openai".to_string(),
            credential_id: "key1".to_string(),
            priority: 1,
        }]);

        // First failure: 5s cooldown
        rotation.report_failure("openai", "key1").await;
        let cooldowns = rotation.cooldowns.read().await;
        let cd = cooldowns.get("openai.key1").unwrap();
        assert_eq!(cd.consecutive_failures, 1);
        let cooldown_duration = cd.cooldown_until - cd.failed_at;
        assert!(cooldown_duration.num_seconds() >= 4 && cooldown_duration.num_seconds() <= 6);
        drop(cooldowns);

        // Second failure: 15s cooldown
        rotation.report_failure("openai", "key1").await;
        let cooldowns = rotation.cooldowns.read().await;
        let cd = cooldowns.get("openai.key1").unwrap();
        assert_eq!(cd.consecutive_failures, 2);
        let cooldown_duration = cd.cooldown_until - cd.failed_at;
        assert!(cooldown_duration.num_seconds() >= 14 && cooldown_duration.num_seconds() <= 16);
    }
}
