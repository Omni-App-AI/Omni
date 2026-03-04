//! Message Router
//!
//! Routes incoming channel messages to extension queues based on bindings.
//! If no binding matches, messages go to the default queue (main agent loop).

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::RwLock;

use crate::bindings::BindingRegistry;
use crate::IncomingMessage;

/// Routes incoming messages from channels to the appropriate extension queue
/// based on the binding registry. Unmatched messages go to the default queue.
pub struct MessageRouter {
    binding_registry: Arc<BindingRegistry>,
    /// Per-extension message queues.
    extension_senders: RwLock<HashMap<String, mpsc::Sender<IncomingMessage>>>,
    /// Default queue for unmatched messages.
    default_tx: mpsc::Sender<IncomingMessage>,
}

impl MessageRouter {
    pub fn new(
        binding_registry: Arc<BindingRegistry>,
        default_tx: mpsc::Sender<IncomingMessage>,
    ) -> Self {
        Self {
            binding_registry,
            extension_senders: RwLock::new(HashMap::new()),
            default_tx,
        }
    }

    /// Register a per-extension message queue.
    pub async fn register_extension(&self, extension_id: &str, tx: mpsc::Sender<IncomingMessage>) {
        self.extension_senders
            .write()
            .await
            .insert(extension_id.to_string(), tx);
    }

    /// Unregister an extension's message queue.
    pub async fn unregister_extension(&self, extension_id: &str) {
        self.extension_senders.write().await.remove(extension_id);
    }

    /// Route a single incoming message based on bindings.
    ///
    /// Returns the extension ID that received the message, or None if
    /// it went to the default queue.
    pub async fn route(&self, msg: IncomingMessage) -> Option<String> {
        let matches = self.binding_registry.resolve(&msg);

        if let Some(binding) = matches.first() {
            let ext_id = &binding.extension_id;
            let senders = self.extension_senders.read().await;

            if let Some(tx) = senders.get(ext_id) {
                if tx.send(msg.clone()).await.is_ok() {
                    return Some(ext_id.clone());
                }
                // Queue full or closed -- fall through to default
                tracing::warn!(
                    extension_id = ext_id.as_str(),
                    "Extension message queue full or closed, routing to default"
                );
            } else {
                tracing::debug!(
                    extension_id = ext_id.as_str(),
                    "No registered queue for bound extension, routing to default"
                );
            }
        }

        // No binding match or send failed -- send to default queue
        if let Err(e) = self.default_tx.send(msg).await {
            tracing::error!("Default message queue closed: {}", e);
        }
        None
    }

    /// Run the router loop, consuming messages from an incoming receiver
    /// and routing them based on bindings.
    pub async fn run(self: Arc<Self>, mut rx: mpsc::Receiver<IncomingMessage>) {
        while let Some(msg) = rx.recv().await {
            let channel_id = msg.channel_id.clone();
            match self.route(msg).await {
                Some(ext_id) => {
                    tracing::debug!(
                        channel = channel_id.as_str(),
                        extension = ext_id.as_str(),
                        "Routed message to extension"
                    );
                }
                None => {
                    tracing::debug!(
                        channel = channel_id.as_str(),
                        "Routed message to default queue"
                    );
                }
            }
        }
        tracing::info!("Message router stopped -- incoming channel closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::ChannelBinding;
    use chrono::Utc;

    fn make_msg(channel_id: &str, sender: &str) -> IncomingMessage {
        IncomingMessage {
            id: "msg-1".to_string(),
            channel_id: channel_id.to_string(),
            channel_type: channel_id.split(':').next().unwrap_or(channel_id).to_string(),
            instance_id: channel_id.split(':').nth(1).unwrap_or("default").to_string(),
            sender: sender.to_string(),
            sender_name: None,
            text: "Hello".to_string(),
            is_group: false,
            group_id: None,
            thread_id: None,
            timestamp: Utc::now(),
            media_url: None,
            source_trust_level: crate::SourceTrustLevel::Authenticated,
        }
    }

    #[tokio::test]
    async fn test_route_to_bound_extension() {
        let registry = Arc::new(BindingRegistry::new());
        registry.add(ChannelBinding {
            id: "b1".to_string(),
            channel_instance: "discord:prod".to_string(),
            extension_id: "ext-a".to_string(),
            peer_filter: None,
            group_filter: None,
            priority: 10,
            enabled: true,
        });

        let (default_tx, mut default_rx) = mpsc::channel(16);
        let (ext_tx, mut ext_rx) = mpsc::channel(16);

        let router = MessageRouter::new(registry, default_tx);
        router.register_extension("ext-a", ext_tx).await;

        let result = router.route(make_msg("discord:prod", "user1")).await;
        assert_eq!(result, Some("ext-a".to_string()));

        // Message should arrive in extension queue, not default
        let msg = ext_rx.try_recv().unwrap();
        assert_eq!(msg.sender, "user1");
        assert!(default_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_route_to_default_when_no_binding() {
        let registry = Arc::new(BindingRegistry::new());
        let (default_tx, mut default_rx) = mpsc::channel(16);

        let router = MessageRouter::new(registry, default_tx);
        let result = router.route(make_msg("discord:prod", "user1")).await;
        assert!(result.is_none());

        let msg = default_rx.try_recv().unwrap();
        assert_eq!(msg.sender, "user1");
    }

    #[tokio::test]
    async fn test_route_to_default_when_no_extension_queue() {
        let registry = Arc::new(BindingRegistry::new());
        registry.add(ChannelBinding {
            id: "b1".to_string(),
            channel_instance: "discord:prod".to_string(),
            extension_id: "ext-a".to_string(),
            peer_filter: None,
            group_filter: None,
            priority: 10,
            enabled: true,
        });

        let (default_tx, mut default_rx) = mpsc::channel(16);
        let router = MessageRouter::new(registry, default_tx);

        // No extension queue registered -- falls to default
        let result = router.route(make_msg("discord:prod", "user1")).await;
        assert!(result.is_none());

        let msg = default_rx.try_recv().unwrap();
        assert_eq!(msg.sender, "user1");
    }

    #[tokio::test]
    async fn test_unregister_extension() {
        let registry = Arc::new(BindingRegistry::new());
        registry.add(ChannelBinding {
            id: "b1".to_string(),
            channel_instance: "discord:prod".to_string(),
            extension_id: "ext-a".to_string(),
            peer_filter: None,
            group_filter: None,
            priority: 10,
            enabled: true,
        });

        let (default_tx, mut default_rx) = mpsc::channel(16);
        let (ext_tx, _ext_rx) = mpsc::channel(16);

        let router = MessageRouter::new(registry, default_tx);
        router.register_extension("ext-a", ext_tx).await;
        router.unregister_extension("ext-a").await;

        // After unregister, message goes to default
        let result = router.route(make_msg("discord:prod", "user1")).await;
        assert!(result.is_none());

        let msg = default_rx.try_recv().unwrap();
        assert_eq!(msg.sender, "user1");
    }
}
