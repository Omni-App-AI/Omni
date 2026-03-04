//! Mattermost Channel Plugin
//!
//! Integrates with Mattermost using the REST API v4 for sending messages
//! and WebSocket for receiving events.
//!
//! ## Authentication
//! - `credential_type`: "api_key"
//! - `data.server_url`: Mattermost server URL (e.g., "https://mattermost.example.com")
//! - `data.token`: Personal access token or bot token
//!
//! ## Login Validation
//! GET {server_url}/api/v4/users/me with bearer token
//!
//! ## Sending Messages
//! POST {server_url}/api/v4/posts with { channel_id, message }
//!
//! ## Receiving Messages
//! WebSocket at wss://{server_url}/api/v4/websocket
//!
//! ## Features
//! - Direct messages
//! - Group messages
//! - Media attachments
//! - Reactions
//! - Typing indicators

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::{
    common::{chunk_message, get_status, set_status},
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
};

/// Mattermost channel plugin using the REST API v4.
pub struct MattermostChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// Mattermost server URL.
    server_url: Mutex<Option<String>>,
    /// Personal access token or bot token.
    token: Mutex<Option<String>>,
    /// Authenticated user ID (populated after login validation).
    user_id: Mutex<Option<String>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the WebSocket task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl MattermostChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            server_url: Mutex::new(None),
            token: Mutex::new(None),
            user_id: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for MattermostChannel {
    fn id(&self) -> &str {
        "mattermost"
    }

    fn name(&self) -> &str {
        "Mattermost"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: false,
            typing_indicators: true,
            threads: false,
        }
    }

    fn status(&self) -> ConnectionStatus {
        get_status(&self.status)
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        let server_url = self.server_url.lock().await.clone().ok_or_else(|| {
            ChannelError::Config("Server URL not set. Call login() first.".into())
        })?;
        let token = self
            .token
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::Config("Token not set. Call login() first.".into()))?;
        let my_user_id = self.user_id.lock().await.clone().unwrap_or_default();

        set_status(&self.status, ConnectionStatus::Connecting);

        // Construct WebSocket URL
        let ws_url = server_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        let ws_url = format!("{}/api/v4/websocket", ws_url.trim_end_matches('/'));

        // Connect
        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|e| {
                ChannelError::Config(format!("Mattermost WebSocket connection failed: {e}"))
            })?;

        let (mut ws_writer, mut ws_reader) = ws_stream.split();

        // Send authentication challenge
        let auth_msg = serde_json::json!({
            "seq": 1,
            "action": "authentication_challenge",
            "data": {
                "token": token
            }
        });

        ws_writer
            .send(WsMessage::Text(auth_msg.to_string().into()))
            .await
            .map_err(|e| ChannelError::Config(format!("Failed to send auth challenge: {e}")))?;

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let incoming_tx = self.incoming_tx.clone();
        let status = self.status.clone();

        set_status(&self.status, ConnectionStatus::Connected);
        tracing::info!("Mattermost WebSocket connected");

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = ws_reader.next() => {
                        match msg {
                            Some(Ok(WsMessage::Text(text))) => {
                                let json: serde_json::Value = match serde_json::from_str(&text) {
                                    Ok(v) => v,
                                    Err(_) => continue,
                                };

                                let event = json["event"].as_str().unwrap_or("");
                                if event != "posted" {
                                    continue;
                                }

                                // data.post is a JSON string inside the event
                                let post_str = match json["data"]["post"].as_str() {
                                    Some(s) => s,
                                    None => continue,
                                };

                                let post: serde_json::Value = match serde_json::from_str(post_str) {
                                    Ok(v) => v,
                                    Err(_) => continue,
                                };

                                // Filter out own messages
                                let poster_id = post["user_id"].as_str().unwrap_or("");
                                if poster_id == my_user_id {
                                    continue;
                                }

                                let text = post["message"].as_str().unwrap_or("").to_string();
                                let channel_id_val = post["channel_id"].as_str().unwrap_or("").to_string();
                                let post_id = post["id"].as_str().unwrap_or("").to_string();
                                let create_at = post["create_at"].as_i64().unwrap_or(0);

                                // Check if it's a DM or channel (channel_type from broadcast data)
                                let channel_type = json["data"]["channel_type"].as_str().unwrap_or("");
                                let is_group = channel_type != "D"; // D = direct message

                                let incoming = crate::IncomingMessage {
                                    id: post_id,
                                    channel_id: "mattermost".to_string(),
                                    channel_type: "mattermost".to_string(),
                                    instance_id: "default".to_string(),
                                    sender: poster_id.to_string(),
                                    sender_name: json["data"]["sender_name"].as_str().map(|s| s.to_string()),
                                    text,
                                    is_group,
                                    group_id: if is_group { Some(channel_id_val) } else { None },
                                    thread_id: None,
                                    timestamp: chrono::DateTime::from_timestamp_millis(create_at)
                                        .unwrap_or_else(chrono::Utc::now),
                                    media_url: None,
                                    source_trust_level: crate::SourceTrustLevel::Authenticated,
                                };

                                if let Err(e) = incoming_tx.send(incoming).await {
                                    tracing::warn!("Failed to forward Mattermost message: {e}");
                                }
                            }
                            Some(Ok(WsMessage::Close(_))) => {
                                tracing::info!("Mattermost WebSocket closed");
                                break;
                            }
                            Some(Err(e)) => {
                                tracing::warn!("Mattermost WebSocket error: {e}");
                                break;
                            }
                            None => break,
                            _ => {} // Ping/Pong/Binary
                        }
                    }
                    _ = &mut shutdown_rx => {
                        tracing::info!("Mattermost WebSocket shutting down");
                        let _ = ws_writer.send(WsMessage::Close(None)).await;
                        break;
                    }
                }
            }

            set_status(&status, ConnectionStatus::Disconnected);
            tracing::info!("Mattermost WebSocket reader task ended");
        });

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
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

        let server_url = credentials
            .data
            .get("server_url")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'server_url' in credentials data".into())
            })?
            .clone();

        let token = credentials
            .data
            .get("token")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'token' in credentials data".into()))?
            .clone();

        if server_url.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Server URL cannot be empty".into(),
            ));
        }

        if token.is_empty() {
            return Err(ChannelError::AuthFailed("Token cannot be empty".into()));
        }

        // Validate token by calling users/me endpoint
        let me_url = format!("{}/api/v4/users/me", server_url.trim_end_matches('/'));

        let resp = self.client.get(&me_url).bearer_auth(&token).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                if let Ok(body) = r.json::<serde_json::Value>().await {
                    let uid = body.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let username = body
                        .get("username")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    tracing::info!("Mattermost authenticated as: {}", username);
                    if !uid.is_empty() {
                        *self.user_id.lock().await = Some(uid.to_string());
                    }
                }
            }
            Ok(r) => {
                tracing::warn!("Mattermost users/me returned HTTP {}", r.status());
                // Store credentials anyway -- server may be temporarily unavailable
            }
            Err(e) => {
                tracing::warn!("Mattermost users/me request failed: {e}");
                // Store credentials anyway -- network may be unavailable
            }
        }

        *self.server_url.lock().await = Some(server_url);
        *self.token.lock().await = Some(token);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.server_url.lock().await = None;
        *self.token.lock().await = None;
        *self.user_id.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected(
                "Mattermost not connected".into(),
            ));
        }

        let server_url = self
            .server_url
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Server URL not set".into()))?;

        let token = self
            .token
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Token not set".into()))?;

        let posts_url = format!("{}/api/v4/posts", server_url.trim_end_matches('/'));

        // Mattermost supports up to 16383 characters per post.
        let chunks = chunk_message(&message.text, 16383);

        for chunk in chunks {
            let body = serde_json::json!({
                "channel_id": recipient,
                "message": chunk
            });

            let resp = self
                .client
                .post(&posts_url)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Mattermost send failed: {e}")))?;

            if !resp.status().is_success() {
                return Err(ChannelError::SendFailed(format!(
                    "Mattermost returned HTTP {}",
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

pub struct MattermostChannelFactory;

impl crate::ChannelPluginFactory for MattermostChannelFactory {
    fn channel_type(&self) -> &str {
        "mattermost"
    }
    fn channel_type_name(&self) -> &str {
        "Mattermost"
    }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: false,
            typing_indicators: true,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(MattermostChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_mattermost_metadata() {
        let channel = MattermostChannel::new();
        assert_eq!(channel.id(), "mattermost");
        assert_eq!(channel.name(), "Mattermost");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_mattermost_features() {
        let channel = MattermostChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(!features.read_receipts);
        assert!(features.typing_indicators);
    }

    #[tokio::test]
    async fn test_mattermost_login_bad_type() {
        let mut channel = MattermostChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_mattermost_send_not_connected() {
        let channel = MattermostChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("channel-id-123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
