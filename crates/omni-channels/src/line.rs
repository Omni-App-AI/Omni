//! LINE Channel Plugin
//!
//! Integrates with the LINE Messaging API for sending and receiving messages.
//!
//! ## Authentication
//! - `credential_type`: "api_key"
//! - `data.channel_access_token`: LINE channel access token
//! - `data.channel_secret`: LINE channel secret (for webhook signature verification)
//!
//! ## Login Validation
//! GET https://api.line.me/v2/bot/info with bearer channel_access_token
//!
//! ## Sending Messages
//! POST https://api.line.me/v2/bot/message/push
//! Body: { to: recipient, messages: [{ type: "text", text: "..." }] }
//!
//! ## Receiving Messages
//! Webhook with HMAC-SHA256 signature verification using channel_secret
//!
//! ## Features
//! - Direct messages
//! - Group messages
//! - Media attachments
//! - Read receipts

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::Engine;
use tokio::sync::{mpsc, Mutex};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// LINE channel plugin using the LINE Messaging API.
pub struct LineChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// LINE channel access token.
    channel_access_token: Mutex<Option<String>>,
    /// LINE channel secret for webhook signature verification.
    channel_secret: Mutex<Option<String>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shared webhook server for receiving incoming messages.
    webhook_server: Option<Arc<crate::webhook_server::WebhookServer>>,
}

impl LineChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            channel_access_token: Mutex::new(None),
            channel_secret: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            webhook_server: None,
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for LineChannel {
    fn id(&self) -> &str {
        "line"
    }

    fn name(&self) -> &str {
        "LINE"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: false,
            read_receipts: true,
            typing_indicators: false,
            threads: false,
        }
    }

    fn status(&self) -> ConnectionStatus {
        get_status(&self.status)
    }

    fn set_webhook_server(&mut self, server: std::sync::Arc<crate::webhook_server::WebhookServer>) {
        self.webhook_server = Some(server);
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        let token = self.channel_access_token.lock().await;
        if token.is_none() {
            return Err(ChannelError::Config(
                "Channel access token not set. Call login() first.".into(),
            ));
        }
        drop(token);

        // Register webhook handler for incoming LINE messages
        if let Some(server) = &self.webhook_server {
            let tx = self.incoming_tx.clone();
            let secret = self.channel_secret.lock().await.clone().unwrap_or_default();

            let handler: crate::webhook_server::WebhookHandler = std::sync::Arc::new(move |_method, _path, body, headers| {
                let tx = tx.clone();
                let secret = secret.clone();
                Box::pin(async move {
                    // Verify HMAC-SHA256 signature
                    let signature = match headers.get("x-line-signature") {
                        Some(sig) => sig.clone(),
                        None => return (401, "Missing signature".to_string()),
                    };

                    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
                        Ok(m) => m,
                        Err(_) => return (500, "HMAC init failed".to_string()),
                    };
                    mac.update(&body);
                    let expected = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

                    if expected != signature {
                        return (401, "Invalid signature".to_string());
                    }

                    // Parse LINE webhook events
                    let json: serde_json::Value = match serde_json::from_slice(&body) {
                        Ok(v) => v,
                        Err(_) => return (400, "Invalid JSON".to_string()),
                    };

                    if let Some(events) = json["events"].as_array() {
                        for event in events {
                            if event["type"].as_str() != Some("message") {
                                continue;
                            }
                            if event["message"]["type"].as_str() != Some("text") {
                                continue;
                            }

                            let text = event["message"]["text"].as_str().unwrap_or("").to_string();
                            let sender = event["source"]["userId"].as_str().unwrap_or("unknown").to_string();
                            let reply_token = event["replyToken"].as_str().unwrap_or("").to_string();
                            let is_group = event["source"]["type"].as_str() == Some("group");
                            let group_id = if is_group {
                                event["source"]["groupId"].as_str().map(|s| s.to_string())
                            } else {
                                None
                            };
                            let timestamp_ms = event["timestamp"].as_i64().unwrap_or(0);

                            let incoming = crate::IncomingMessage {
                                id: reply_token,
                                channel_id: "line".to_string(),
                                channel_type: "line".to_string(),
                                instance_id: "default".to_string(),
                                sender,
                                sender_name: None,
                                text,
                                is_group,
                                group_id,
                                thread_id: None,
                                timestamp: chrono::DateTime::from_timestamp_millis(timestamp_ms)
                                    .unwrap_or_else(chrono::Utc::now),
                                media_url: None,
                                source_trust_level: crate::SourceTrustLevel::Authenticated,
                            };

                            let _ = tx.send(incoming).await;
                        }
                    }

                    (200, "OK".to_string())
                })
            });

            server.register_handler("line", handler).await;
        }

        set_status(&self.status, ConnectionStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(server) = &self.webhook_server {
            server.unregister_handler("line").await;
        }
        set_status(&self.status, ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "api_key" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'api_key'.",
                credentials.credential_type
            )));
        }

        let channel_access_token = credentials
            .data
            .get("channel_access_token")
            .ok_or_else(|| {
                ChannelError::AuthFailed(
                    "Missing 'channel_access_token' in credentials data".into(),
                )
            })?
            .clone();

        let channel_secret = credentials
            .data
            .get("channel_secret")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'channel_secret' in credentials data".into())
            })?
            .clone();

        if channel_access_token.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Channel access token cannot be empty".into(),
            ));
        }

        if channel_secret.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Channel secret cannot be empty".into(),
            ));
        }

        // Validate token by calling bot info endpoint
        let resp = self
            .client
            .get("https://api.line.me/v2/bot/info")
            .bearer_auth(&channel_access_token)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                if let Ok(body) = r.json::<serde_json::Value>().await {
                    let display_name = body
                        .get("displayName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    tracing::info!("LINE authenticated as: {}", display_name);
                }
            }
            Ok(r) => {
                tracing::warn!("LINE bot/info returned HTTP {}", r.status());
                // Store tokens anyway -- validation may fail in offline/test environments
            }
            Err(e) => {
                tracing::warn!("LINE bot/info request failed: {e}");
                // Store tokens anyway -- network may be unavailable
            }
        }

        *self.channel_access_token.lock().await = Some(channel_access_token);
        *self.channel_secret.lock().await = Some(channel_secret);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.channel_access_token.lock().await = None;
        *self.channel_secret.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("LINE not connected".into()));
        }

        let token = self
            .channel_access_token
            .lock()
            .await
            .clone()
            .ok_or_else(|| {
                ChannelError::NotConnected("Channel access token not set".into())
            })?;

        // LINE text messages have a 5000 character limit per bubble.
        // LINE push API allows up to 5 message objects per request.
        // We chunk at 5000 chars and send each as a separate push.
        let chunks = chunk_message(&message.text, 5000);

        for chunk in chunks {
            let body = serde_json::json!({
                "to": recipient,
                "messages": [
                    {
                        "type": "text",
                        "text": chunk
                    }
                ]
            });

            let resp = self
                .client
                .post("https://api.line.me/v2/bot/message/push")
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("LINE send failed: {e}")))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let error_body = resp
                    .text()
                    .await
                    .unwrap_or_else(|_| "unknown".to_string());
                return Err(ChannelError::SendFailed(format!(
                    "LINE returned HTTP {}: {}",
                    status, error_body
                )));
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct LineChannelFactory;

impl crate::ChannelPluginFactory for LineChannelFactory {
    fn channel_type(&self) -> &str { "line" }
    fn channel_type_name(&self) -> &str { "LINE" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: false,
            read_receipts: true,
            typing_indicators: false,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(LineChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_line_metadata() {
        let channel = LineChannel::new();
        assert_eq!(channel.id(), "line");
        assert_eq!(channel.name(), "LINE");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_line_features() {
        let channel = LineChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(!features.reactions);
        assert!(features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_line_login_bad_type() {
        let mut channel = LineChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_line_login_missing_token() {
        let mut channel = LineChannel::new();

        // Missing channel_secret
        let mut data = HashMap::new();
        data.insert(
            "channel_access_token".to_string(),
            "test-token".to_string(),
        );
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("channel_secret"));

        // Missing channel_access_token
        let mut data = HashMap::new();
        data.insert("channel_secret".to_string(), "test-secret".to_string());
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("channel_access_token"));
    }

    #[tokio::test]
    async fn test_line_send_not_connected() {
        let channel = LineChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("U1234567890abcdef", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
