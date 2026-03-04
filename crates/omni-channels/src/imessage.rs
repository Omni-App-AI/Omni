//! iMessage Channel Plugin
//!
//! Thin wrapper around [`BlueBubblesChannel`] with different id/name.
//! Requires a BlueBubbles macOS server for iMessage relay.
//!
//! This plugin presents as "iMessage" to users while delegating all
//! protocol-level work to the BlueBubbles REST API integration.
//!
//! ## Authentication
//! Same as BlueBubbles:
//! - `credential_type`: "api_key"
//! - `data.server_url`: BlueBubbles server URL
//! - `data.password`: BlueBubbles server password

use tokio::sync::mpsc;

use crate::{
    ChannelConfig, ChannelCredentials, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    bluebubbles::BlueBubblesChannel,
};

/// iMessage channel plugin -- delegates to BlueBubbles internally.
pub struct IMessageChannel {
    inner: BlueBubblesChannel,
}

impl IMessageChannel {
    pub fn new() -> Self {
        Self {
            inner: BlueBubblesChannel::new(),
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for IMessageChannel {
    fn id(&self) -> &str {
        "imessage"
    }

    fn name(&self) -> &str {
        "iMessage"
    }

    fn features(&self) -> ChannelFeatures {
        self.inner.features()
    }

    fn status(&self) -> ConnectionStatus {
        self.inner.status()
    }

    async fn connect(&mut self, config: ChannelConfig) -> Result<()> {
        self.inner.connect(config).await
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.inner.disconnect().await
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        self.inner.login(credentials).await
    }

    async fn logout(&mut self) -> Result<()> {
        self.inner.logout().await
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        self.inner.send_message(recipient, message).await
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.inner.incoming_messages()
    }
}


pub struct IMessageChannelFactory;

impl crate::ChannelPluginFactory for IMessageChannelFactory {
    fn channel_type(&self) -> &str { "imessage" }
    fn channel_type_name(&self) -> &str { "iMessage" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: true,
            typing_indicators: false,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(IMessageChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_is_imessage() {
        let channel = IMessageChannel::new();
        assert_eq!(channel.id(), "imessage");
        assert_eq!(channel.name(), "iMessage");
        // Confirm it differs from the inner BlueBubbles identity
        assert_ne!(channel.id(), "bluebubbles");
    }

    #[test]
    fn test_features_match_bluebubbles() {
        let imessage = IMessageChannel::new();
        let bb = BlueBubblesChannel::new();

        let im_features = imessage.features();
        let bb_features = bb.features();

        assert_eq!(im_features.direct_messages, bb_features.direct_messages);
        assert_eq!(im_features.group_messages, bb_features.group_messages);
        assert_eq!(im_features.media_attachments, bb_features.media_attachments);
        assert_eq!(im_features.reactions, bb_features.reactions);
        assert_eq!(im_features.read_receipts, bb_features.read_receipts);
        assert_eq!(im_features.typing_indicators, bb_features.typing_indicators);
    }

    #[tokio::test]
    async fn test_send_not_connected() {
        let channel = IMessageChannel::new();
        let msg = OutgoingMessage {
            text: "Hello from iMessage".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("iMessage;+;chat123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
