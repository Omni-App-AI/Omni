//! Synology Chat Channel Plugin
//!
//! Simple webhook-based integration with Synology Chat.
//!
//! ## Authentication
//! - `credential_type`: "webhook"
//! - `data.outgoing_url`: webhook URL for sending messages to Synology Chat
//! - `data.incoming_token`: token for verifying incoming webhooks from Synology Chat
//!
//! ## Sending Messages
//! POST to outgoing_url with `payload={"text":"..."}`
//!
//! ## Receiving Messages
//! Via shared webhook server at /synology-chat, verifying incoming_token.
//!
//! ## Features
//! - Direct messages
//! - Group messages

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Synology Chat channel plugin using webhooks.
pub struct SynologyChatChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for sending webhook requests.
    client: reqwest::Client,
    /// Outgoing webhook URL for posting messages.
    outgoing_url: Mutex<Option<String>>,
    /// Token for verifying incoming webhooks.
    incoming_token: Arc<Mutex<Option<String>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shared webhook server for receiving incoming messages.
    webhook_server: Option<Arc<crate::webhook_server::WebhookServer>>,
}

impl SynologyChatChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            outgoing_url: Mutex::new(None),
            incoming_token: Arc::new(Mutex::new(None)),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            webhook_server: None,
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for SynologyChatChannel {
    fn id(&self) -> &str {
        "synology-chat"
    }

    fn name(&self) -> &str {
        "Synology Chat"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: false,
            reactions: false,
            read_receipts: false,
            typing_indicators: false,
            threads: false,
        }
    }

    fn status(&self) -> ConnectionStatus {
        get_status(&self.status)
    }

    fn set_webhook_server(&mut self, server: Arc<crate::webhook_server::WebhookServer>) {
        self.webhook_server = Some(server);
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        let url = self.outgoing_url.lock().await;
        if url.is_none() {
            return Err(ChannelError::Config(
                "Outgoing URL not set. Call login() first.".into(),
            ));
        }
        drop(url);

        // Register incoming webhook handler
        if let Some(server) = &self.webhook_server {
            let tx = self.incoming_tx.clone();
            let token = self.incoming_token.clone();

            let handler: crate::webhook_server::WebhookHandler = Arc::new(move |_method, _path, body, _headers| {
                let tx = tx.clone();
                let token = token.clone();
                Box::pin(async move {
                    // Synology Chat sends webhooks as form-encoded or JSON
                    let json: serde_json::Value = match serde_json::from_slice(&body) {
                        Ok(v) => v,
                        Err(_) => {
                            // Try parsing as form-encoded payload=JSON
                            let body_str = String::from_utf8_lossy(&body);
                            if let Some(payload) = body_str.strip_prefix("payload=") {
                                match serde_json::from_str(payload) {
                                    Ok(v) => v,
                                    Err(_) => return (400, "Invalid payload".to_string()),
                                }
                            } else {
                                return (400, "Invalid body format".to_string());
                            }
                        }
                    };

                    // Verify token if configured
                    if let Some(expected_token) = token.lock().await.as_ref() {
                        let received_token = json.get("token")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if received_token != expected_token {
                            return (403, "Invalid token".to_string());
                        }
                    }

                    let text = json.get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if text.is_empty() {
                        return (200, "OK".to_string());
                    }

                    let sender = json.get("user_id")
                        .and_then(|v| v.as_str())
                        .or_else(|| json.get("username").and_then(|v| v.as_str()))
                        .unwrap_or("unknown")
                        .to_string();

                    let sender_name = json.get("username")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let msg_id = json.get("post_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let incoming = IncomingMessage {
                        id: if msg_id.is_empty() { uuid::Uuid::new_v4().to_string() } else { msg_id },
                        channel_id: "synology-chat".to_string(),
                        channel_type: "synology-chat".to_string(),
                        instance_id: "default".to_string(),
                        sender,
                        sender_name,
                        text,
                        is_group: false,
                        group_id: None,
                        thread_id: None,
                        timestamp: chrono::Utc::now(),
                        media_url: None,
                        source_trust_level: crate::SourceTrustLevel::Authenticated,
                    };

                    let _ = tx.send(incoming).await;
                    (200, "OK".to_string())
                })
            });

            server.register_handler("synology-chat", handler).await;
        }

        set_status(&self.status, ConnectionStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(server) = &self.webhook_server {
            server.unregister_handler("synology-chat").await;
        }
        set_status(&self.status, ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "webhook" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'webhook'.",
                credentials.credential_type
            )));
        }

        let outgoing_url = credentials
            .data
            .get("outgoing_url")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'outgoing_url' in credentials data".into())
            })?
            .clone();

        let incoming_token = credentials
            .data
            .get("incoming_token")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'incoming_token' in credentials data".into())
            })?
            .clone();

        if outgoing_url.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Outgoing URL cannot be empty".into(),
            ));
        }

        if incoming_token.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Incoming token cannot be empty".into(),
            ));
        }

        *self.outgoing_url.lock().await = Some(outgoing_url);
        *self.incoming_token.lock().await = Some(incoming_token);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.outgoing_url.lock().await = None;
        *self.incoming_token.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, _recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected(
                "Synology Chat not connected".into(),
            ));
        }

        let url = self
            .outgoing_url
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Outgoing URL not set".into()))?;

        let chunks = chunk_message(&message.text, 4000);

        for chunk in chunks {
            let payload = serde_json::json!({ "text": chunk });
            let payload_str = format!("payload={}", serde_json::to_string(&payload).map_err(
                |e| ChannelError::SendFailed(format!("Failed to serialize payload: {e}")),
            )?);

            let resp = self
                .client
                .post(&url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(payload_str)
                .send()
                .await
                .map_err(|e| {
                    ChannelError::SendFailed(format!("Synology Chat send failed: {e}"))
                })?;

            if !resp.status().is_success() {
                return Err(ChannelError::SendFailed(format!(
                    "Synology Chat returned HTTP {}",
                    resp.status()
                )));
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct SynologyChatChannelFactory;

impl crate::ChannelPluginFactory for SynologyChatChannelFactory {
    fn channel_type(&self) -> &str { "synology-chat" }
    fn channel_type_name(&self) -> &str { "Synology Chat" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: false,
            reactions: false,
            read_receipts: false,
            typing_indicators: false,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(SynologyChatChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_synology_chat_metadata() {
        let channel = SynologyChatChannel::new();
        assert_eq!(channel.id(), "synology-chat");
        assert_eq!(channel.name(), "Synology Chat");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_synology_chat_features() {
        let channel = SynologyChatChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(!features.media_attachments);
        assert!(!features.reactions);
        assert!(!features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_synology_chat_login_bad_type() {
        let mut channel = SynologyChatChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_synology_chat_send_not_connected() {
        let channel = SynologyChatChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("some-channel", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
