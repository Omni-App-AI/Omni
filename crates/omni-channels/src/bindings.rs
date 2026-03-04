//! Channel-to-Extension Binding System
//!
//! Maps channel instances to extensions for message routing.
//! Bindings are enforced: bound extensions can ONLY send through
//! their bound channel accounts.

use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::IncomingMessage;

/// A binding between a channel instance and an extension.
///
/// When an extension is bound to a channel instance, incoming messages from
/// that instance are routed to the extension, and the extension can ONLY
/// send messages through its bound channel instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelBinding {
    /// Unique binding identifier (UUID).
    pub id: String,
    /// Compound channel instance key (e.g., "discord:production").
    pub channel_instance: String,
    /// Extension that handles messages for this channel instance.
    pub extension_id: String,
    /// Optional filter: only match messages from specific peers (glob pattern).
    pub peer_filter: Option<String>,
    /// Optional filter: only match messages from specific groups (glob pattern).
    pub group_filter: Option<String>,
    /// Priority for conflict resolution (higher = preferred).
    pub priority: i32,
    /// Whether this binding is active.
    pub enabled: bool,
}

impl ChannelBinding {
    /// Check if a message matches this binding's filters.
    pub fn matches(&self, msg: &IncomingMessage) -> bool {
        if !self.enabled {
            return false;
        }

        // Check channel instance match
        if msg.channel_id != self.channel_instance {
            return false;
        }

        // Check peer filter (glob-like matching)
        if let Some(ref filter) = self.peer_filter {
            if !glob_match(filter, &msg.sender) {
                return false;
            }
        }

        // Check group filter
        if let Some(ref filter) = self.group_filter {
            if msg.is_group {
                if let Some(ref gid) = msg.group_id {
                    if !glob_match(filter, gid) {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                // Group filter set but message is not from a group
                return false;
            }
        }

        true
    }

    /// Specificity score for tie-breaking: more filters = more specific.
    fn specificity(&self) -> u32 {
        let mut score = 0;
        if self.peer_filter.is_some() {
            score += 1;
        }
        if self.group_filter.is_some() {
            score += 1;
        }
        score
    }
}

/// Simple glob matching supporting `*` as wildcard.
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if !pattern.contains('*') {
        return pattern == text;
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 2 {
        // Single wildcard: prefix*suffix
        let prefix = parts[0];
        let suffix = parts[1];
        return text.starts_with(prefix) && text.ends_with(suffix);
    }

    // Multiple wildcards: fall back to simple check
    // For most use cases, prefix* or *suffix is enough
    let first = parts[0];
    let last = parts[parts.len() - 1];
    if !first.is_empty() && !text.starts_with(first) {
        return false;
    }
    if !last.is_empty() && !text.ends_with(last) {
        return false;
    }
    true
}

/// Registry of channel-extension bindings with thread-safe access.
///
/// Supports:
/// - Adding/removing bindings
/// - Resolving which extension(s) should handle an incoming message
/// - Looking up which channel instance(s) an extension is allowed to send through
pub struct BindingRegistry {
    bindings: RwLock<Vec<ChannelBinding>>,
}

impl BindingRegistry {
    pub fn new() -> Self {
        Self {
            bindings: RwLock::new(Vec::new()),
        }
    }

    /// Load bindings from a pre-existing list (e.g., from database).
    pub fn load(&self, bindings: Vec<ChannelBinding>) {
        let mut lock = self.bindings.write().unwrap();
        *lock = bindings;
    }

    /// Add a new binding. Returns the binding ID.
    pub fn add(&self, binding: ChannelBinding) -> String {
        let id = binding.id.clone();
        let mut lock = self.bindings.write().unwrap();
        // Remove any existing binding with the same ID (upsert semantics)
        lock.retain(|b| b.id != id);
        lock.push(binding);
        id
    }

    /// Remove a binding by ID. Returns true if found and removed.
    pub fn remove(&self, binding_id: &str) -> bool {
        let mut lock = self.bindings.write().unwrap();
        let before = lock.len();
        lock.retain(|b| b.id != binding_id);
        lock.len() < before
    }

    /// List all bindings.
    pub fn list(&self) -> Vec<ChannelBinding> {
        self.bindings.read().unwrap().clone()
    }

    /// List bindings for a specific extension.
    pub fn list_for_extension(&self, extension_id: &str) -> Vec<ChannelBinding> {
        self.bindings
            .read()
            .unwrap()
            .iter()
            .filter(|b| b.extension_id == extension_id)
            .cloned()
            .collect()
    }

    /// List bindings for a specific channel instance.
    pub fn list_for_channel(&self, channel_instance: &str) -> Vec<ChannelBinding> {
        self.bindings
            .read()
            .unwrap()
            .iter()
            .filter(|b| b.channel_instance == channel_instance)
            .cloned()
            .collect()
    }

    /// Resolve which extension(s) should handle an incoming message.
    ///
    /// Returns matching bindings sorted by priority (highest first),
    /// then by specificity (most specific first).
    pub fn resolve(&self, msg: &IncomingMessage) -> Vec<ChannelBinding> {
        let lock = self.bindings.read().unwrap();
        let mut matches: Vec<ChannelBinding> = lock
            .iter()
            .filter(|b| b.matches(msg))
            .cloned()
            .collect();

        // Sort by priority (descending), then specificity (descending)
        matches.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| b.specificity().cmp(&a.specificity()))
        });

        matches
    }

    /// Get channel instances an extension is bound to (for send validation).
    ///
    /// Returns the list of channel instance keys this extension can send through.
    /// If the extension has no bindings at all, returns empty (meaning the
    /// extension is unbound and can use any channel -- backward compat).
    pub fn get_bound_instances(&self, extension_id: &str) -> Vec<String> {
        self.bindings
            .read()
            .unwrap()
            .iter()
            .filter(|b| b.extension_id == extension_id && b.enabled)
            .map(|b| b.channel_instance.clone())
            .collect()
    }

    /// Check if an extension has any bindings at all.
    pub fn has_bindings(&self, extension_id: &str) -> bool {
        self.bindings
            .read()
            .unwrap()
            .iter()
            .any(|b| b.extension_id == extension_id)
    }

    /// Validate whether an extension is allowed to send through a given channel.
    ///
    /// Rules:
    /// - If the extension has no bindings → allowed (unbound = unrestricted)
    /// - If the extension has bindings → only allowed for bound channel instances
    pub fn can_send(&self, extension_id: &str, channel_instance: &str) -> bool {
        let lock = self.bindings.read().unwrap();
        let bound: Vec<&ChannelBinding> = lock
            .iter()
            .filter(|b| b.extension_id == extension_id)
            .collect();

        if bound.is_empty() {
            // Unbound extension -- no restrictions
            return true;
        }

        // Bound extension -- must match one of its enabled channel instances
        bound
            .iter()
            .any(|b| b.enabled && b.channel_instance == channel_instance)
    }

    /// Get a mapping of channel instance → list of bound extension IDs.
    pub fn channel_to_extensions(&self) -> HashMap<String, Vec<String>> {
        let lock = self.bindings.read().unwrap();
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for b in lock.iter() {
            if b.enabled {
                map.entry(b.channel_instance.clone())
                    .or_default()
                    .push(b.extension_id.clone());
            }
        }
        map
    }
}

impl Default for BindingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_msg(channel_id: &str, sender: &str, is_group: bool, group_id: Option<&str>) -> IncomingMessage {
        IncomingMessage {
            id: "msg-1".to_string(),
            channel_id: channel_id.to_string(),
            channel_type: channel_id.split(':').next().unwrap_or(channel_id).to_string(),
            instance_id: channel_id.split(':').nth(1).unwrap_or("default").to_string(),
            sender: sender.to_string(),
            sender_name: None,
            text: "Hello".to_string(),
            is_group,
            group_id: group_id.map(|s| s.to_string()),
            thread_id: None,
            timestamp: Utc::now(),
            media_url: None,
            source_trust_level: crate::SourceTrustLevel::Authenticated,
        }
    }

    fn make_binding(
        id: &str,
        channel_instance: &str,
        extension_id: &str,
        priority: i32,
    ) -> ChannelBinding {
        ChannelBinding {
            id: id.to_string(),
            channel_instance: channel_instance.to_string(),
            extension_id: extension_id.to_string(),
            peer_filter: None,
            group_filter: None,
            priority,
            enabled: true,
        }
    }

    #[test]
    fn test_binding_matches_basic() {
        let binding = make_binding("b1", "discord:prod", "ext-a", 10);
        let msg = make_msg("discord:prod", "user1", false, None);
        assert!(binding.matches(&msg));

        let msg2 = make_msg("discord:staging", "user1", false, None);
        assert!(!binding.matches(&msg2));
    }

    #[test]
    fn test_binding_matches_peer_filter() {
        let mut binding = make_binding("b1", "discord:prod", "ext-a", 10);
        binding.peer_filter = Some("admin-*".to_string());

        let msg1 = make_msg("discord:prod", "admin-bob", false, None);
        assert!(binding.matches(&msg1));

        let msg2 = make_msg("discord:prod", "user-alice", false, None);
        assert!(!binding.matches(&msg2));
    }

    #[test]
    fn test_binding_matches_group_filter() {
        let mut binding = make_binding("b1", "discord:prod", "ext-a", 10);
        binding.group_filter = Some("support-*".to_string());

        // Group message matching filter
        let msg1 = make_msg("discord:prod", "user1", true, Some("support-general"));
        assert!(binding.matches(&msg1));

        // Group message not matching filter
        let msg2 = make_msg("discord:prod", "user1", true, Some("random"));
        assert!(!binding.matches(&msg2));

        // DM -- group filter set but message is not a group message
        let msg3 = make_msg("discord:prod", "user1", false, None);
        assert!(!binding.matches(&msg3));
    }

    #[test]
    fn test_binding_disabled() {
        let mut binding = make_binding("b1", "discord:prod", "ext-a", 10);
        binding.enabled = false;

        let msg = make_msg("discord:prod", "user1", false, None);
        assert!(!binding.matches(&msg));
    }

    #[test]
    fn test_registry_add_remove() {
        let reg = BindingRegistry::new();
        let binding = make_binding("b1", "discord:prod", "ext-a", 10);

        reg.add(binding);
        assert_eq!(reg.list().len(), 1);

        assert!(reg.remove("b1"));
        assert_eq!(reg.list().len(), 0);

        assert!(!reg.remove("nonexistent"));
    }

    #[test]
    fn test_registry_upsert() {
        let reg = BindingRegistry::new();
        reg.add(make_binding("b1", "discord:prod", "ext-a", 10));
        reg.add(make_binding("b1", "discord:staging", "ext-b", 20));

        let bindings = reg.list();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].channel_instance, "discord:staging");
        assert_eq!(bindings[0].extension_id, "ext-b");
    }

    #[test]
    fn test_registry_resolve_priority() {
        let reg = BindingRegistry::new();
        reg.add(make_binding("b1", "discord:prod", "ext-low", 1));
        reg.add(make_binding("b2", "discord:prod", "ext-high", 10));

        let msg = make_msg("discord:prod", "user1", false, None);
        let resolved = reg.resolve(&msg);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].extension_id, "ext-high");
        assert_eq!(resolved[1].extension_id, "ext-low");
    }

    #[test]
    fn test_registry_resolve_specificity() {
        let reg = BindingRegistry::new();
        // Same priority, but one has a peer filter (more specific)
        let b1 = make_binding("b1", "discord:prod", "ext-general", 10);
        let mut b2 = make_binding("b2", "discord:prod", "ext-specific", 10);
        b2.peer_filter = Some("admin-*".to_string());

        reg.add(b1);
        reg.add(b2);

        let msg = make_msg("discord:prod", "admin-bob", false, None);
        let resolved = reg.resolve(&msg);
        assert_eq!(resolved.len(), 2);
        // More specific binding comes first
        assert_eq!(resolved[0].extension_id, "ext-specific");
    }

    #[test]
    fn test_registry_can_send_unbound() {
        let reg = BindingRegistry::new();
        // No bindings for ext-a → can send anywhere
        assert!(reg.can_send("ext-a", "discord:prod"));
        assert!(reg.can_send("ext-a", "telegram:default"));
    }

    #[test]
    fn test_registry_can_send_bound() {
        let reg = BindingRegistry::new();
        reg.add(make_binding("b1", "discord:prod", "ext-a", 10));

        // Bound extension -- can only use its bound channel
        assert!(reg.can_send("ext-a", "discord:prod"));
        assert!(!reg.can_send("ext-a", "discord:staging"));
        assert!(!reg.can_send("ext-a", "telegram:default"));

        // Unbound extension -- unrestricted
        assert!(reg.can_send("ext-b", "discord:staging"));
    }

    #[test]
    fn test_registry_get_bound_instances() {
        let reg = BindingRegistry::new();
        reg.add(make_binding("b1", "discord:prod", "ext-a", 10));
        reg.add(make_binding("b2", "twitter:brand-a", "ext-a", 5));
        reg.add(make_binding("b3", "discord:prod", "ext-b", 10));

        let instances = reg.get_bound_instances("ext-a");
        assert_eq!(instances.len(), 2);
        assert!(instances.contains(&"discord:prod".to_string()));
        assert!(instances.contains(&"twitter:brand-a".to_string()));

        let instances_b = reg.get_bound_instances("ext-b");
        assert_eq!(instances_b.len(), 1);

        let instances_c = reg.get_bound_instances("ext-c");
        assert!(instances_c.is_empty());
    }

    #[test]
    fn test_registry_list_for_extension() {
        let reg = BindingRegistry::new();
        reg.add(make_binding("b1", "discord:prod", "ext-a", 10));
        reg.add(make_binding("b2", "twitter:brand-a", "ext-a", 5));
        reg.add(make_binding("b3", "discord:prod", "ext-b", 10));

        assert_eq!(reg.list_for_extension("ext-a").len(), 2);
        assert_eq!(reg.list_for_extension("ext-b").len(), 1);
        assert_eq!(reg.list_for_extension("ext-c").len(), 0);
    }

    #[test]
    fn test_registry_list_for_channel() {
        let reg = BindingRegistry::new();
        reg.add(make_binding("b1", "discord:prod", "ext-a", 10));
        reg.add(make_binding("b2", "discord:prod", "ext-b", 5));
        reg.add(make_binding("b3", "twitter:brand-a", "ext-a", 10));

        assert_eq!(reg.list_for_channel("discord:prod").len(), 2);
        assert_eq!(reg.list_for_channel("twitter:brand-a").len(), 1);
        assert_eq!(reg.list_for_channel("telegram:default").len(), 0);
    }

    #[test]
    fn test_registry_channel_to_extensions() {
        let reg = BindingRegistry::new();
        reg.add(make_binding("b1", "discord:prod", "ext-a", 10));
        reg.add(make_binding("b2", "discord:prod", "ext-b", 5));
        reg.add(make_binding("b3", "twitter:brand-a", "ext-a", 10));

        let map = reg.channel_to_extensions();
        assert_eq!(map.len(), 2);
        assert_eq!(map["discord:prod"].len(), 2);
        assert_eq!(map["twitter:brand-a"].len(), 1);
    }

    #[test]
    fn test_registry_load() {
        let reg = BindingRegistry::new();
        reg.add(make_binding("b1", "discord:prod", "ext-a", 10));

        // Load replaces all bindings
        reg.load(vec![
            make_binding("b2", "telegram:default", "ext-b", 5),
        ]);
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "b2");
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("admin-*", "admin-bob"));
        assert!(!glob_match("admin-*", "user-bob"));
        assert!(glob_match("*-support", "team-support"));
        assert!(!glob_match("*-support", "team-general"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "different"));
    }

    #[test]
    fn test_has_bindings() {
        let reg = BindingRegistry::new();
        assert!(!reg.has_bindings("ext-a"));

        reg.add(make_binding("b1", "discord:prod", "ext-a", 10));
        assert!(reg.has_bindings("ext-a"));
        assert!(!reg.has_bindings("ext-b"));
    }
}
