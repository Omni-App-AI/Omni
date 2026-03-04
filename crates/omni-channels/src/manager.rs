//! Channel Manager -- manages lifecycle of channel plugins and routes messages.
//!
//! The ChannelManager:
//! - Registers channel plugin factories (one per channel type)
//! - Creates and manages channel plugin instances (multiple per type)
//! - Routes incoming messages to the agent loop via a broadcast channel
//! - Routes agent responses back to the originating channel
//! - Integrates with EventBus for UI notifications
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, RwLock};

use omni_core::events::{EventBus, OmniEvent};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelInfo, ChannelInstanceId,
    ChannelPlugin, ChannelPluginFactory, ChannelTypeInfo, ConnectionStatus, IncomingMessage,
    LoginStatus, OutgoingMessage, Result,
};

/// Manages all registered channel plugin instances.
pub struct ChannelManager {
    /// Registered channel plugin instances, keyed by compound key (e.g., "discord:production").
    channels: RwLock<HashMap<String, ChannelEntry>>,
    /// Factories for creating new instances, keyed by channel type (e.g., "discord").
    factories: RwLock<HashMap<String, Arc<dyn ChannelPluginFactory>>>,
    /// Sender for incoming messages -- listeners (like AgentLoop) subscribe via `incoming_rx()`.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver for incoming messages -- handed out once via `take_incoming_rx()`.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Event bus for UI notifications.
    event_bus: EventBus,
    /// Shared webhook server for channels that receive via HTTP webhooks.
    webhook_server: Arc<crate::webhook_server::WebhookServer>,
}

/// Internal entry for a registered channel instance.
struct ChannelEntry {
    plugin: Arc<Mutex<Box<dyn ChannelPlugin>>>,
    instance_id: ChannelInstanceId,
}

impl ChannelManager {
    /// Create a new ChannelManager with the default webhook server config (127.0.0.1:8900).
    pub fn new(event_bus: EventBus) -> Self {
        Self::new_with_webhook_config(event_bus, 8900, [127, 0, 0, 1])
    }

    /// Create a new ChannelManager with custom webhook server bind address and port.
    pub fn new_with_webhook_config(
        event_bus: EventBus,
        webhook_port: u16,
        webhook_bind_address: [u8; 4],
    ) -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(256);
        Self {
            channels: RwLock::new(HashMap::new()),
            factories: RwLock::new(HashMap::new()),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            event_bus,
            webhook_server: Arc::new(crate::webhook_server::WebhookServer::new_with_bind_address(
                webhook_port,
                webhook_bind_address,
            )),
        }
    }

    /// Take the incoming message receiver. Can only be called once.
    /// The caller (typically AgentLoop or a message router) uses this to
    /// receive messages from all connected channels.
    pub async fn take_incoming_rx(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.lock().await.take()
    }

    // ── Factory Registration ─────────────────────────────────────────

    /// Register a channel plugin factory (one per channel type).
    /// Factories are used to create new instances via `create_instance()`.
    pub async fn register_factory(&self, factory: Arc<dyn ChannelPluginFactory>) {
        let channel_type = factory.channel_type().to_string();
        tracing::info!(channel_type = channel_type.as_str(), "Registering channel factory");
        self.factories
            .write()
            .await
            .insert(channel_type, factory);
    }

    /// List all available channel types (from registered factories).
    pub async fn list_channel_types(&self) -> Vec<ChannelTypeInfo> {
        let factories = self.factories.read().await;
        factories
            .values()
            .map(|f| ChannelTypeInfo {
                channel_type: f.channel_type().to_string(),
                name: f.channel_type_name().to_string(),
                features: f.features(),
            })
            .collect()
    }

    // ── Instance Management ──────────────────────────────────────────

    /// Create and register a new channel instance from a factory.
    /// Returns the compound key (e.g., "discord:production").
    pub async fn create_instance(
        &self,
        channel_type: &str,
        instance_id: &str,
    ) -> Result<String> {
        let factories = self.factories.read().await;
        let factory = factories.get(channel_type).ok_or_else(|| {
            ChannelError::NotFound(format!("No factory for channel type: {}", channel_type))
        })?;

        let plugin = factory.create_instance(instance_id);
        let cid = ChannelInstanceId::new(channel_type, instance_id);
        let key = cid.key();

        drop(factories);

        let mut channels = self.channels.write().await;
        if channels.contains_key(&key) {
            return Err(ChannelError::Config(format!(
                "Instance already exists: {}",
                key
            )));
        }

        channels.insert(
            key.clone(),
            ChannelEntry {
                plugin: Arc::new(Mutex::new(plugin)),
                instance_id: cid,
            },
        );

        tracing::info!(compound_key = key.as_str(), "Created channel instance");
        Ok(key)
    }

    /// Remove a channel instance. Disconnects it first if connected.
    pub async fn remove_instance(&self, compound_key: &str) -> Result<()> {
        let entry = self
            .channels
            .write()
            .await
            .remove(compound_key)
            .ok_or_else(|| ChannelError::NotFound(compound_key.to_string()))?;

        let mut plugin = entry.plugin.lock().await;
        if plugin.status() == ConnectionStatus::Connected {
            let _ = plugin.disconnect().await;
        }

        self.event_bus.emit(OmniEvent::ChannelDisconnected {
            channel_id: compound_key.to_string(),
        });

        tracing::info!(compound_key, "Removed channel instance");
        Ok(())
    }

    // ── Legacy Registration (backward compat) ────────────────────────

    /// Register a channel plugin directly (legacy API).
    /// Creates an entry with instance_id "default".
    /// Prefer `register_factory()` + `create_instance()` for new code.
    pub async fn register(&self, plugin: Box<dyn ChannelPlugin>) {
        let channel_type = plugin.id().to_string();
        let cid = ChannelInstanceId::new(&channel_type, "default");
        let key = cid.key();
        tracing::info!(compound_key = key.as_str(), "Registering channel plugin (legacy)");
        self.channels.write().await.insert(
            key,
            ChannelEntry {
                plugin: Arc::new(Mutex::new(plugin)),
                instance_id: cid,
            },
        );
    }

    /// Unregister a channel plugin by key. Disconnects it first if connected.
    /// Accepts either compound key ("discord:default") or bare type ("discord")
    /// for backward compatibility.
    pub async fn unregister(&self, channel_id: &str) -> Result<()> {
        let key = self.resolve_key(channel_id).await?;
        let entry = self
            .channels
            .write()
            .await
            .remove(&key)
            .ok_or_else(|| ChannelError::NotFound(channel_id.to_string()))?;

        let mut plugin = entry.plugin.lock().await;
        if plugin.status() == ConnectionStatus::Connected {
            let _ = plugin.disconnect().await;
        }

        tracing::info!(channel_id, "Unregistered channel plugin");
        Ok(())
    }

    // ── Queries ──────────────────────────────────────────────────────

    /// List all registered channel instances with their current status.
    pub async fn list_channels(&self) -> Vec<ChannelInfo> {
        let channels = self.channels.read().await;
        let mut infos = Vec::with_capacity(channels.len());

        for (key, entry) in channels.iter() {
            let plugin = entry.plugin.lock().await;
            infos.push(ChannelInfo {
                id: key.clone(),
                channel_type: entry.instance_id.channel_type.clone(),
                instance_id: entry.instance_id.instance_id.clone(),
                name: plugin.name().to_string(),
                status: plugin.status(),
                features: plugin.features(),
            });
        }

        infos
    }

    /// Get info for a specific channel instance.
    /// Accepts compound key or bare type name.
    pub async fn get_channel(&self, channel_id: &str) -> Result<ChannelInfo> {
        let key = self.resolve_key(channel_id).await?;
        let channels = self.channels.read().await;
        let entry = channels
            .get(&key)
            .ok_or_else(|| ChannelError::NotFound(channel_id.to_string()))?;

        let plugin = entry.plugin.lock().await;
        Ok(ChannelInfo {
            id: key,
            channel_type: entry.instance_id.channel_type.clone(),
            instance_id: entry.instance_id.instance_id.clone(),
            name: plugin.name().to_string(),
            status: plugin.status(),
            features: plugin.features(),
        })
    }

    /// Retrieve the API key for a channel instance (if applicable).
    /// Currently only WebChat channels return a key.
    pub async fn get_channel_api_key(&self, channel_id: &str) -> Result<Option<String>> {
        let key = self.resolve_key(channel_id).await?;
        let channels = self.channels.read().await;
        let entry = channels
            .get(&key)
            .ok_or_else(|| ChannelError::NotFound(channel_id.to_string()))?;

        let plugin = entry.plugin.lock().await;
        Ok(plugin.get_api_key())
    }

    // ── Connection Lifecycle ─────────────────────────────────────────

    /// Connect a channel plugin instance with the given configuration.
    pub async fn connect(
        &self,
        channel_id: &str,
        config: ChannelConfig,
    ) -> Result<()> {
        let key = self.resolve_key(channel_id).await?;
        let (plugin_arc, cid) = {
            let channels = self.channels.read().await;
            let entry = channels
                .get(&key)
                .ok_or_else(|| ChannelError::NotFound(channel_id.to_string()))?;
            (entry.plugin.clone(), entry.instance_id.clone())
        };

        let mut plugin = plugin_arc.lock().await;

        // Inject event bus for async events (QR codes, etc.)
        plugin.set_event_bus(self.event_bus.clone(), key.clone());

        // Inject webhook server and ensure it's running
        plugin.set_webhook_server(self.webhook_server.clone());
        if let Err(e) = self.webhook_server.start().await {
            tracing::warn!("Failed to start webhook server: {e}");
        }

        plugin.connect(config).await?;

        // NOTE: We do NOT emit ChannelConnected here. For async channels (like
        // WhatsApp), connect() only starts the sidecar -- the real connection
        // happens later. The plugin itself emits ChannelConnected when actually
        // connected. The frontend refreshes on that event.

        tracing::info!(compound_key = key.as_str(), "Channel connect initiated");

        // Start listening for incoming messages from this channel instance
        if let Some(mut rx) = plugin.incoming_messages() {
            let tx = self.incoming_tx.clone();
            let event_bus = self.event_bus.clone();
            let compound_key = key.clone();
            let channel_type = cid.channel_type.clone();
            let instance_id = cid.instance_id.clone();

            tokio::spawn(async move {
                while let Some(mut msg) = rx.recv().await {
                    // Override instance routing fields so the message carries
                    // the correct compound key and instance info.
                    msg.channel_id = compound_key.clone();
                    msg.channel_type = channel_type.clone();
                    msg.instance_id = instance_id.clone();

                    event_bus.emit(OmniEvent::ChannelMessageReceived {
                        channel_id: compound_key.clone(),
                        sender: msg.sender.clone(),
                        text: msg.text.clone(),
                    });

                    if tx.send(msg).await.is_err() {
                        tracing::warn!(
                            channel_id = compound_key.as_str(),
                            "Incoming message channel closed"
                        );
                        break;
                    }
                }
                tracing::debug!(
                    channel_id = compound_key.as_str(),
                    "Channel incoming message listener ended"
                );
            });
        }

        Ok(())
    }

    /// Disconnect a channel plugin instance.
    pub async fn disconnect(&self, channel_id: &str) -> Result<()> {
        let key = self.resolve_key(channel_id).await?;
        let plugin_arc = {
            let channels = self.channels.read().await;
            channels
                .get(&key)
                .ok_or_else(|| ChannelError::NotFound(channel_id.to_string()))?
                .plugin
                .clone()
        };

        let mut plugin = plugin_arc.lock().await;
        plugin.disconnect().await?;

        self.event_bus.emit(OmniEvent::ChannelDisconnected {
            channel_id: key.clone(),
        });

        tracing::info!(compound_key = key.as_str(), "Channel disconnected");
        Ok(())
    }

    // ── Authentication ───────────────────────────────────────────────

    /// Login/authenticate with a channel instance.
    pub async fn login(
        &self,
        channel_id: &str,
        credentials: ChannelCredentials,
    ) -> Result<LoginStatus> {
        let key = self.resolve_key(channel_id).await?;
        let plugin_arc = {
            let channels = self.channels.read().await;
            channels
                .get(&key)
                .ok_or_else(|| ChannelError::NotFound(channel_id.to_string()))?
                .plugin
                .clone()
        };

        let mut plugin = plugin_arc.lock().await;
        let status = plugin.login(credentials).await?;

        tracing::info!(compound_key = key.as_str(), ?status, "Channel login attempt");
        Ok(status)
    }

    /// Logout from a channel instance.
    pub async fn logout(&self, channel_id: &str) -> Result<()> {
        let key = self.resolve_key(channel_id).await?;
        let plugin_arc = {
            let channels = self.channels.read().await;
            channels
                .get(&key)
                .ok_or_else(|| ChannelError::NotFound(channel_id.to_string()))?
                .plugin
                .clone()
        };

        let mut plugin = plugin_arc.lock().await;
        plugin.logout().await?;

        tracing::info!(compound_key = key.as_str(), "Channel logged out");
        Ok(())
    }

    // ── Messaging ────────────────────────────────────────────────────

    /// Send a message to a recipient via a channel instance.
    /// Permission is enforced at the extension level via bindings and capabilities.
    /// Accepts compound key ("discord:production") or bare type ("discord").
    pub async fn send_message(
        &self,
        channel_id: &str,
        recipient: &str,
        message: OutgoingMessage,
    ) -> Result<()> {
        let key = self.resolve_key(channel_id).await?;
        let plugin_arc = {
            let channels = self.channels.read().await;
            channels
                .get(&key)
                .ok_or_else(|| ChannelError::NotFound(channel_id.to_string()))?
                .plugin
                .clone()
        };

        let plugin = plugin_arc.lock().await;

        if plugin.status() != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected(key.clone()));
        }

        plugin.send_message(recipient, message).await?;

        self.event_bus.emit(OmniEvent::ChannelMessageSent {
            channel_id: key,
            recipient: recipient.to_string(),
        });

        Ok(())
    }

    // ── Internal Helpers ─────────────────────────────────────────────

    /// Resolve a channel_id (which may be a bare type name like "discord" or
    /// a compound key like "discord:production") to the actual HashMap key.
    ///
    /// For bare type names, tries `{type}:default` first, then falls back to
    /// looking for any instance of that type.
    async fn resolve_key(&self, channel_id: &str) -> Result<String> {
        let channels = self.channels.read().await;

        // Direct match (compound key or legacy bare key from old register())
        if channels.contains_key(channel_id) {
            return Ok(channel_id.to_string());
        }

        // Try compound key with :default
        let default_key = ChannelInstanceId::parse(channel_id).key();
        if channels.contains_key(&default_key) {
            return Ok(default_key);
        }

        Err(ChannelError::NotFound(channel_id.to_string()))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChannelFeatures;

    /// A mock channel plugin for testing.
    struct MockChannel {
        id: String,
        name: String,
        status: std::sync::atomic::AtomicU8,
    }

    impl MockChannel {
        fn new(id: &str, name: &str) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                status: std::sync::atomic::AtomicU8::new(0), // Disconnected
            }
        }

        fn status_from_u8(v: u8) -> ConnectionStatus {
            match v {
                0 => ConnectionStatus::Disconnected,
                1 => ConnectionStatus::Connecting,
                2 => ConnectionStatus::Connected,
                3 => ConnectionStatus::Reconnecting,
                _ => ConnectionStatus::Error,
            }
        }
    }

    #[async_trait::async_trait]
    impl ChannelPlugin for MockChannel {
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn features(&self) -> ChannelFeatures {
            ChannelFeatures::default()
        }
        fn status(&self) -> ConnectionStatus {
            Self::status_from_u8(self.status.load(std::sync::atomic::Ordering::Relaxed))
        }
        async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
            self.status
                .store(2, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
        async fn disconnect(&mut self) -> Result<()> {
            self.status
                .store(0, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
        async fn login(&mut self, _credentials: ChannelCredentials) -> Result<LoginStatus> {
            Ok(LoginStatus::Success)
        }
        async fn logout(&mut self) -> Result<()> {
            Ok(())
        }
        async fn send_message(&self, _recipient: &str, _message: OutgoingMessage) -> Result<()> {
            Ok(())
        }
        fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
            None
        }
    }

    /// A mock channel factory for testing.
    struct MockChannelFactory {
        channel_type: String,
        name: String,
    }

    impl MockChannelFactory {
        fn new(channel_type: &str, name: &str) -> Self {
            Self {
                channel_type: channel_type.to_string(),
                name: name.to_string(),
            }
        }
    }

    impl ChannelPluginFactory for MockChannelFactory {
        fn channel_type(&self) -> &str {
            &self.channel_type
        }
        fn channel_type_name(&self) -> &str {
            &self.name
        }
        fn features(&self) -> ChannelFeatures {
            ChannelFeatures::default()
        }
        fn create_instance(&self, _instance_id: &str) -> Box<dyn ChannelPlugin> {
            Box::new(MockChannel::new(&self.channel_type, &self.name))
        }
    }

    fn test_manager() -> ChannelManager {
        let event_bus = EventBus::new(64);
        ChannelManager::new(event_bus)
    }

    // ── Legacy register() tests ──────────────────────────────────────

    #[tokio::test]
    async fn test_register_and_list() {
        let mgr = test_manager();
        assert!(mgr.list_channels().await.is_empty());

        mgr.register(Box::new(MockChannel::new("whatsapp", "WhatsApp")))
            .await;
        mgr.register(Box::new(MockChannel::new("telegram", "Telegram")))
            .await;

        let channels = mgr.list_channels().await;
        assert_eq!(channels.len(), 2);

        let ids: Vec<&str> = channels.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"whatsapp:default"));
        assert!(ids.contains(&"telegram:default"));
    }

    #[tokio::test]
    async fn test_get_channel() {
        let mgr = test_manager();
        mgr.register(Box::new(MockChannel::new("whatsapp", "WhatsApp")))
            .await;

        // Both bare type and compound key should work
        let info = mgr.get_channel("whatsapp").await.unwrap();
        assert_eq!(info.name, "WhatsApp");
        assert_eq!(info.channel_type, "whatsapp");
        assert_eq!(info.instance_id, "default");
        assert_eq!(info.status, ConnectionStatus::Disconnected);

        let info2 = mgr.get_channel("whatsapp:default").await.unwrap();
        assert_eq!(info2.name, "WhatsApp");

        let err = mgr.get_channel("nonexistent").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_send_message_not_connected() {
        let mgr = test_manager();
        mgr.register(Box::new(MockChannel::new("whatsapp", "WhatsApp")))
            .await;

        let result = mgr
            .send_message(
                "whatsapp",
                "+1234567890",
                OutgoingMessage {
                    text: "Hello".to_string(),
                    media_url: None,
                    reply_to: None,
                    thread_id: None,
                },
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not connected"),
            "Expected not-connected error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_unregister() {
        let mgr = test_manager();
        mgr.register(Box::new(MockChannel::new("whatsapp", "WhatsApp")))
            .await;
        assert_eq!(mgr.list_channels().await.len(), 1);

        mgr.unregister("whatsapp").await.unwrap();
        assert!(mgr.list_channels().await.is_empty());

        // Unregister nonexistent
        let err = mgr.unregister("whatsapp").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_channel_not_found() {
        let mgr = test_manager();

        let result = mgr
            .connect(
                "nonexistent",
                ChannelConfig {
                    settings: HashMap::new(),
                },
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_take_incoming_rx() {
        let mgr = test_manager();

        // First call should return Some
        let rx = mgr.take_incoming_rx().await;
        assert!(rx.is_some());

        // Second call should return None (already taken)
        let rx2 = mgr.take_incoming_rx().await;
        assert!(rx2.is_none());
    }

    // ── Multi-instance tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_factory_and_create_instance() {
        let mgr = test_manager();

        mgr.register_factory(Arc::new(MockChannelFactory::new("discord", "Discord")))
            .await;

        let types = mgr.list_channel_types().await;
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].channel_type, "discord");

        // Create two instances
        let key1 = mgr.create_instance("discord", "production").await.unwrap();
        assert_eq!(key1, "discord:production");

        let key2 = mgr.create_instance("discord", "staging").await.unwrap();
        assert_eq!(key2, "discord:staging");

        let channels = mgr.list_channels().await;
        assert_eq!(channels.len(), 2);

        let ids: Vec<&str> = channels.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"discord:production"));
        assert!(ids.contains(&"discord:staging"));

        // Channel types are correct
        for ch in &channels {
            assert_eq!(ch.channel_type, "discord");
        }
    }

    #[tokio::test]
    async fn test_create_instance_duplicate() {
        let mgr = test_manager();
        mgr.register_factory(Arc::new(MockChannelFactory::new("discord", "Discord")))
            .await;

        mgr.create_instance("discord", "prod").await.unwrap();
        let err = mgr.create_instance("discord", "prod").await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_instance_unknown_type() {
        let mgr = test_manager();

        let err = mgr.create_instance("unknown", "default").await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("No factory"));
    }

    #[tokio::test]
    async fn test_remove_instance() {
        let mgr = test_manager();
        mgr.register_factory(Arc::new(MockChannelFactory::new("discord", "Discord")))
            .await;

        mgr.create_instance("discord", "prod").await.unwrap();
        assert_eq!(mgr.list_channels().await.len(), 1);

        mgr.remove_instance("discord:prod").await.unwrap();
        assert!(mgr.list_channels().await.is_empty());

        // Remove nonexistent
        let err = mgr.remove_instance("discord:prod").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_resolve_key_backward_compat() {
        let mgr = test_manager();

        // Legacy register with bare type name
        mgr.register(Box::new(MockChannel::new("discord", "Discord")))
            .await;

        // Should resolve bare "discord" to "discord:default"
        let info = mgr.get_channel("discord").await.unwrap();
        assert_eq!(info.id, "discord:default");

        // Should also work with explicit compound key
        let info2 = mgr.get_channel("discord:default").await.unwrap();
        assert_eq!(info2.id, "discord:default");
    }

    #[tokio::test]
    async fn test_mixed_legacy_and_factory() {
        let mgr = test_manager();

        // Legacy register
        mgr.register(Box::new(MockChannel::new("telegram", "Telegram")))
            .await;

        // Factory-based
        mgr.register_factory(Arc::new(MockChannelFactory::new("discord", "Discord")))
            .await;
        mgr.create_instance("discord", "bot1").await.unwrap();
        mgr.create_instance("discord", "bot2").await.unwrap();

        let channels = mgr.list_channels().await;
        assert_eq!(channels.len(), 3);
    }
}
