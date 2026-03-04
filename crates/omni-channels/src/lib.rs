//! Omni Channel Plugin System
//!
//! Enables messaging platform integrations (WhatsApp, Telegram, Discord, etc.)
//! through a plugin trait. Channel plugins run natively (not in WASM) because
//! they need persistent connections, WebSockets, and background threads.
//!
//! All channel operations are permission-gated and audited.

pub mod common;
pub mod discord;
pub mod manager;
pub mod telegram;
pub mod webhook_server;
pub mod whatsapp_web;

// --- Binding & routing ---
pub mod bindings;
pub mod router;
pub mod slack;
pub mod mattermost;
pub mod line;
pub mod teams;
pub mod google_chat;
pub mod feishu;
pub mod irc_channel;
pub mod twitch;
pub mod matrix;
pub mod nostr_channel;
pub mod nextcloud_talk;
pub mod synology_chat;
pub mod zalo;
pub mod bluebubbles;
pub mod imessage;
pub mod signal;
pub mod urbit;
pub mod webchat;
pub mod twitter;

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ── Channel Instance Identification ─────────────────────────────────

/// Compound identifier for a channel instance.
///
/// Channels are identified by `{channel_type}:{instance_id}`, e.g.
/// `"discord:production"`, `"telegram:support"`. A bare type name like
/// `"discord"` is treated as `"discord:default"` for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelInstanceId {
    /// The channel type (e.g., "discord", "telegram").
    pub channel_type: String,
    /// User-assigned instance label (e.g., "production", "default").
    pub instance_id: String,
}

impl ChannelInstanceId {
    pub fn new(channel_type: &str, instance_id: &str) -> Self {
        Self {
            channel_type: channel_type.to_string(),
            instance_id: instance_id.to_string(),
        }
    }

    /// Parse a compound key like `"discord:production"` or a bare type like `"discord"`
    /// (which becomes `"discord:default"`).
    pub fn parse(s: &str) -> Self {
        match s.split_once(':') {
            Some((ct, iid)) if !iid.is_empty() => Self::new(ct, iid),
            _ => Self::new(s, "default"),
        }
    }

    /// The canonical compound key string, e.g. `"discord:production"`.
    pub fn key(&self) -> String {
        format!("{}:{}", self.channel_type, self.instance_id)
    }

    /// Whether this is the default instance.
    pub fn is_default(&self) -> bool {
        self.instance_id == "default"
    }
}

impl fmt::Display for ChannelInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.channel_type, self.instance_id)
    }
}

/// Summary of a channel type (from a factory), without any instance state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelTypeInfo {
    pub channel_type: String,
    pub name: String,
    pub features: ChannelFeatures,
}

/// Errors that can occur in channel operations.
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Channel not found: {0}")]
    NotFound(String),

    #[error("Channel not connected: {0}")]
    NotConnected(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ChannelError>;

/// Features supported by a channel plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelFeatures {
    pub direct_messages: bool,
    pub group_messages: bool,
    pub media_attachments: bool,
    pub reactions: bool,
    pub read_receipts: bool,
    pub typing_indicators: bool,
    #[serde(default)]
    pub threads: bool,
}

impl Default for ChannelFeatures {
    fn default() -> Self {
        Self {
            direct_messages: true,
            group_messages: false,
            media_attachments: false,
            reactions: false,
            read_receipts: false,
            typing_indicators: false,
            threads: false,
        }
    }
}

/// Trust level of the message source.
///
/// Used by the Guardian and agent loop to apply appropriate security policies.
/// Lower-trust sources may face stricter scanning or tool restrictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceTrustLevel {
    /// Fully trusted: Tauri IPC, direct CLI input.
    Trusted,
    /// Authenticated via API key or OAuth -- identity verified but external.
    Authenticated,
    /// Unauthenticated or unknown source (if allowed by config).
    Unauthenticated,
}

impl Default for SourceTrustLevel {
    fn default() -> Self {
        Self::Authenticated
    }
}

impl SourceTrustLevel {
    /// Convert to a Guardian threshold modifier.
    ///
    /// Returns a multiplier for Guardian scanning thresholds:
    /// - `Trusted` / `Authenticated` → `1.0` (default thresholds)
    /// - `Unauthenticated` → `0.8` (20% stricter -- lower thresholds block more)
    pub fn guardian_threshold_modifier(self) -> f64 {
        match self {
            Self::Trusted | Self::Authenticated => 1.0,
            Self::Unauthenticated => 0.8,
        }
    }
}

/// Connection status of a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "disconnected"),
            Self::Connecting => write!(f, "connecting"),
            Self::Connected => write!(f, "connected"),
            Self::Reconnecting => write!(f, "reconnecting"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Login status returned from authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoginStatus {
    /// Successfully authenticated.
    Success,
    /// Waiting for QR code scan or external approval.
    PendingApproval { qr_code_data: Option<String> },
    /// Authentication failed.
    Failed { reason: String },
}

/// Configuration for connecting to a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel-specific configuration key-value pairs.
    pub settings: HashMap<String, serde_json::Value>,
}

/// Credentials for authenticating with a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCredentials {
    /// Type of credentials (api_key, oauth, qr_code, etc.)
    pub credential_type: String,
    /// Credential data (API key, token, etc.)
    pub data: HashMap<String, String>,
}

/// A message to send to a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub text: String,
    pub media_url: Option<String>,
    pub reply_to: Option<String>,
    #[serde(default)]
    pub thread_id: Option<String>,
}

/// A message received from a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    /// Unique message ID from the channel.
    pub id: String,
    /// Compound channel instance key (e.g., "discord:production").
    /// For default instances, this equals `"{channel_type}:default"`.
    /// Set by ChannelManager when forwarding; plugins set it to the bare type name.
    pub channel_id: String,
    /// Bare channel type (e.g., "discord", "telegram").
    pub channel_type: String,
    /// Instance label (e.g., "production", "default").
    pub instance_id: String,
    /// Sender identifier (phone number, username, etc.)
    pub sender: String,
    /// Sender display name.
    pub sender_name: Option<String>,
    /// Message content.
    pub text: String,
    /// Whether this is a group message.
    pub is_group: bool,
    /// Group ID if applicable.
    pub group_id: Option<String>,
    /// Thread ID if this message is part of a thread.
    #[serde(default)]
    pub thread_id: Option<String>,
    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Media attachment URL if any.
    pub media_url: Option<String>,
    /// Trust level of the source that produced this message.
    /// Defaults to `Authenticated` for backward compatibility.
    #[serde(default)]
    pub source_trust_level: SourceTrustLevel,
}

/// Channel plugin summary for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    /// Compound instance key (e.g., "discord:production").
    pub id: String,
    /// Bare channel type (e.g., "discord").
    pub channel_type: String,
    /// Instance label (e.g., "production", "default").
    pub instance_id: String,
    /// Human-readable name (e.g., "Discord (Production)").
    pub name: String,
    pub status: ConnectionStatus,
    pub features: ChannelFeatures,
}

/// Factory that creates new instances of a channel plugin type.
///
/// Each channel type (Discord, Telegram, etc.) provides a factory so the
/// `ChannelManager` can create multiple instances of the same type, each
/// with independent credentials and connections.
pub trait ChannelPluginFactory: Send + Sync {
    /// The channel type identifier (e.g., "discord", "telegram").
    fn channel_type(&self) -> &str;

    /// Human-readable channel type name (e.g., "Discord", "Telegram").
    fn channel_type_name(&self) -> &str;

    /// Features supported by this channel type.
    fn features(&self) -> ChannelFeatures;

    /// Create a new plugin instance. The `instance_id` is the user-chosen label
    /// (e.g., "production", "staging"). Plugins don't need to store this --
    /// the `ChannelManager` tracks it externally.
    fn create_instance(&self, instance_id: &str) -> Box<dyn ChannelPlugin>;
}

/// Trait that all channel plugins must implement.
///
/// Channel plugins run natively (not in WASM) because they need:
/// - Persistent connections (WebSockets, long polling)
/// - Background threads for receiving messages
/// - Access to system resources (network, storage)
///
/// All operations are still permission-gated via the PolicyEngine.
#[async_trait]
pub trait ChannelPlugin: Send + Sync {
    /// Unique identifier for this channel type (e.g., "whatsapp", "telegram").
    fn id(&self) -> &str;

    /// Human-readable name (e.g., "WhatsApp", "Telegram").
    fn name(&self) -> &str;

    /// Features supported by this channel.
    fn features(&self) -> ChannelFeatures;

    /// Current connection status.
    fn status(&self) -> ConnectionStatus;

    // --- Lifecycle ---

    /// Connect to the channel service.
    async fn connect(&mut self, config: ChannelConfig) -> Result<()>;

    /// Disconnect from the channel service.
    async fn disconnect(&mut self) -> Result<()>;

    // --- Authentication ---

    /// Authenticate with the channel service.
    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus>;

    /// Log out from the channel service.
    async fn logout(&mut self) -> Result<()>;

    // --- Messaging ---

    /// Send a message to a recipient.
    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()>;

    /// Get a receiver for incoming messages.
    /// The channel plugin pushes messages to this channel.
    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>>;

    /// Set the shared webhook server for receiving incoming messages.
    /// Channels that use webhooks should override this to store the reference.
    fn set_webhook_server(&mut self, _server: Arc<crate::webhook_server::WebhookServer>) {}

    /// Set the event bus for emitting async events (e.g., QR codes).
    /// Channels that push events asynchronously should override this.
    fn set_event_bus(&mut self, _event_bus: omni_core::events::EventBus, _channel_id: String) {}

    /// Retrieve the current API key for this channel, if applicable.
    /// Only channels that use API key authentication (e.g., WebChat) override this.
    fn get_api_key(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_features_default() {
        let features = ChannelFeatures::default();
        assert!(features.direct_messages);
        assert!(!features.group_messages);
        assert!(!features.media_attachments);
    }

    #[test]
    fn test_connection_status_display() {
        assert_eq!(ConnectionStatus::Connected.to_string(), "connected");
        assert_eq!(ConnectionStatus::Disconnected.to_string(), "disconnected");
    }

    #[test]
    fn test_incoming_message_serialization() {
        let msg = IncomingMessage {
            id: "msg1".to_string(),
            channel_id: "whatsapp:default".to_string(),
            channel_type: "whatsapp".to_string(),
            instance_id: "default".to_string(),
            sender: "+1234567890".to_string(),
            sender_name: Some("John".to_string()),
            text: "Hello!".to_string(),
            is_group: false,
            group_id: None,
            thread_id: None,
            timestamp: chrono::Utc::now(),
            media_url: None,
            source_trust_level: SourceTrustLevel::Authenticated,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: IncomingMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sender, "+1234567890");
        assert_eq!(deserialized.text, "Hello!");
        assert_eq!(deserialized.channel_type, "whatsapp");
        assert_eq!(deserialized.instance_id, "default");
        assert_eq!(deserialized.source_trust_level, SourceTrustLevel::Authenticated);
    }

    #[test]
    fn test_source_trust_level_default() {
        // Verify that deserializing without source_trust_level defaults to Authenticated
        let json = r#"{
            "id": "msg1",
            "channel_id": "test:default",
            "channel_type": "test",
            "instance_id": "default",
            "sender": "user",
            "sender_name": null,
            "text": "hello",
            "is_group": false,
            "group_id": null,
            "timestamp": "2024-01-01T00:00:00Z",
            "media_url": null
        }"#;
        let msg: IncomingMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.source_trust_level, SourceTrustLevel::Authenticated);
    }

    #[test]
    fn test_outgoing_message() {
        let msg = OutgoingMessage {
            text: "Hello from Omni!".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["text"], "Hello from Omni!");
    }

    #[test]
    fn test_login_status_variants() {
        let success = LoginStatus::Success;
        let json = serde_json::to_string(&success).unwrap();
        assert!(json.contains("Success"));

        let pending = LoginStatus::PendingApproval {
            qr_code_data: Some("data:image/png;base64,...".to_string()),
        };
        let json = serde_json::to_string(&pending).unwrap();
        assert!(json.contains("qr_code_data"));
    }

    #[test]
    fn test_channel_info() {
        let info = ChannelInfo {
            id: "whatsapp:default".to_string(),
            channel_type: "whatsapp".to_string(),
            instance_id: "default".to_string(),
            name: "WhatsApp".to_string(),
            status: ConnectionStatus::Connected,
            features: ChannelFeatures::default(),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], "whatsapp:default");
        assert_eq!(json["channel_type"], "whatsapp");
        assert_eq!(json["status"], "Connected");
    }

    // ── ChannelInstanceId tests ───────────────────────────────────────

    #[test]
    fn test_instance_id_parse_compound() {
        let cid = ChannelInstanceId::parse("discord:production");
        assert_eq!(cid.channel_type, "discord");
        assert_eq!(cid.instance_id, "production");
        assert_eq!(cid.key(), "discord:production");
        assert!(!cid.is_default());
    }

    #[test]
    fn test_instance_id_parse_bare() {
        let cid = ChannelInstanceId::parse("discord");
        assert_eq!(cid.channel_type, "discord");
        assert_eq!(cid.instance_id, "default");
        assert_eq!(cid.key(), "discord:default");
        assert!(cid.is_default());
    }

    #[test]
    fn test_instance_id_parse_empty_instance() {
        // "discord:" with empty instance should default
        let cid = ChannelInstanceId::parse("discord:");
        assert_eq!(cid.channel_type, "discord:");
        assert_eq!(cid.instance_id, "default");
    }

    #[test]
    fn test_instance_id_display() {
        let cid = ChannelInstanceId::new("telegram", "bot-1");
        assert_eq!(format!("{}", cid), "telegram:bot-1");
    }

    #[test]
    fn test_instance_id_equality() {
        let a = ChannelInstanceId::new("discord", "prod");
        let b = ChannelInstanceId::parse("discord:prod");
        assert_eq!(a, b);
    }

    #[test]
    fn test_instance_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ChannelInstanceId::new("discord", "prod"));
        assert!(set.contains(&ChannelInstanceId::parse("discord:prod")));
        assert!(!set.contains(&ChannelInstanceId::parse("discord:staging")));
    }
}
