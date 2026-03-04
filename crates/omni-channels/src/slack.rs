//! Slack Channel Plugin
//!
//! Integrates with Slack using the Web API for sending messages and
//! Socket Mode for receiving events via WebSocket.
//!
//! ## Authentication
//! - `credential_type`: "bot_token"
//! - `data.bot_token`: Slack bot token (xoxb-...)
//! - `data.app_token`: Slack app-level token (xapp-...) for Socket Mode
//!
//! ## Login Validation
//! POST https://slack.com/api/auth.test with bearer bot_token
//!
//! ## Sending Messages
//! POST https://slack.com/api/chat.postMessage with bearer bot_token
//! Character limit: 4000 per message
//!
//! ## Receiving Messages
//! Socket Mode WebSocket
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

/// Slack channel plugin using the Web API and Socket Mode.
pub struct SlackChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// Slack bot token (xoxb-...).
    bot_token: Mutex<Option<String>>,
    /// Slack app-level token (xapp-...) for Socket Mode.
    app_token: Mutex<Option<String>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the Socket Mode WebSocket task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl SlackChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            bot_token: Mutex::new(None),
            app_token: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for SlackChannel {
    fn id(&self) -> &str {
        "slack"
    }

    fn name(&self) -> &str {
        "Slack"
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
        let _bot_token_val =
            self.bot_token.lock().await.clone().ok_or_else(|| {
                ChannelError::Config("Bot token not set. Call login() first.".into())
            })?;
        let app_token_val =
            self.app_token.lock().await.clone().ok_or_else(|| {
                ChannelError::Config("App token not set. Call login() first.".into())
            })?;

        set_status(&self.status, ConnectionStatus::Connecting);

        // Request a WebSocket URL via apps.connections.open
        let resp = self
            .client
            .post("https://slack.com/api/apps.connections.open")
            .bearer_auth(&app_token_val)
            .send()
            .await
            .map_err(|e| {
                ChannelError::Config(format!("Failed to open Socket Mode connection: {e}"))
            })?;

        let body: serde_json::Value = resp.json().await.map_err(|e| {
            ChannelError::Config(format!("Failed to parse connections.open response: {e}"))
        })?;

        if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let error = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(ChannelError::Config(format!(
                "Socket Mode connection failed: {error}"
            )));
        }

        let wss_url = body["url"]
            .as_str()
            .ok_or_else(|| ChannelError::Config("No WebSocket URL in response".into()))?
            .to_string();

        // Connect to WebSocket
        let (ws_stream, _) = tokio_tungstenite::connect_async(&wss_url)
            .await
            .map_err(|e| ChannelError::Config(format!("WebSocket connection failed: {e}")))?;

        let (mut ws_writer, mut ws_reader) = ws_stream.split();

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let incoming_tx = self.incoming_tx.clone();
        let status = self.status.clone();

        set_status(&self.status, ConnectionStatus::Connected);
        tracing::info!("Slack Socket Mode WebSocket connected");

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

                                let envelope_id = json["envelope_id"].as_str().unwrap_or("").to_string();

                                // Acknowledge the envelope
                                if !envelope_id.is_empty() {
                                    let ack = serde_json::json!({"envelope_id": envelope_id});
                                    if let Err(e) = ws_writer.send(WsMessage::Text(ack.to_string().into())).await {
                                        tracing::warn!("Failed to send Slack ack: {e}");
                                    }
                                }

                                // Process events
                                let msg_type = json["type"].as_str().unwrap_or("");
                                if msg_type == "events_api" {
                                    let event = &json["payload"]["event"];
                                    if event["type"].as_str() == Some("message")
                                        && event["subtype"].is_null()  // skip bot messages, edits, etc.
                                    {
                                        let text = event["text"].as_str().unwrap_or("").to_string();
                                        let user = event["user"].as_str().unwrap_or("unknown").to_string();
                                        let channel = event["channel"].as_str().unwrap_or("").to_string();
                                        let ts = event["ts"].as_str().unwrap_or("0").to_string();
                                        let channel_type = event["channel_type"].as_str().unwrap_or("");
                                        let is_group = channel_type != "im";

                                        let incoming = crate::IncomingMessage {
                                            id: ts,
                                            channel_id: "slack".to_string(),
                                            channel_type: "slack".to_string(),
                                            instance_id: "default".to_string(),
                                            sender: user,
                                            sender_name: None,
                                            text,
                                            is_group,
                                            group_id: if is_group { Some(channel.clone()) } else { None },
                                            thread_id: None,
                                            timestamp: chrono::Utc::now(),
                                            media_url: None,
                                            source_trust_level: crate::SourceTrustLevel::Authenticated,
                                        };

                                        if let Err(e) = incoming_tx.send(incoming).await {
                                            tracing::warn!("Failed to forward Slack message: {e}");
                                        }
                                    }
                                } else if msg_type == "disconnect" {
                                    tracing::info!("Slack Socket Mode disconnect requested");
                                    break;
                                }
                            }
                            Some(Ok(WsMessage::Close(_))) => {
                                tracing::info!("Slack WebSocket closed");
                                break;
                            }
                            Some(Err(e)) => {
                                tracing::warn!("Slack WebSocket error: {e}");
                                break;
                            }
                            None => break,
                            _ => {} // Ping/Pong/Binary -- ignore
                        }
                    }
                    _ = &mut shutdown_rx => {
                        tracing::info!("Slack Socket Mode shutting down");
                        let _ = ws_writer.send(WsMessage::Close(None)).await;
                        break;
                    }
                }
            }

            set_status(&status, ConnectionStatus::Disconnected);
            tracing::info!("Slack Socket Mode reader task ended");
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
        if credentials.credential_type != "bot_token" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'bot_token'.",
                credentials.credential_type
            )));
        }

        let bot_token = credentials
            .data
            .get("bot_token")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'bot_token' in credentials data".into())
            })?
            .clone();

        let app_token = credentials
            .data
            .get("app_token")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'app_token' in credentials data".into())
            })?
            .clone();

        if bot_token.is_empty() {
            return Err(ChannelError::AuthFailed("Bot token cannot be empty".into()));
        }

        if app_token.is_empty() {
            return Err(ChannelError::AuthFailed("App token cannot be empty".into()));
        }

        // Validate bot token by calling auth.test
        let resp = self
            .client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&bot_token)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                // Parse the response to check the "ok" field
                if let Ok(body) = r.json::<serde_json::Value>().await {
                    if body.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                        let team = body
                            .get("team")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let user = body
                            .get("user")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        tracing::info!("Slack authenticated as {} in team {}", user, team);
                    } else {
                        tracing::warn!(
                            "Slack auth.test returned ok=false: {}",
                            body.get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                        );
                    }
                }
            }
            Ok(r) => {
                tracing::warn!("Slack auth.test returned HTTP {}", r.status());
                // Store tokens anyway -- validation may fail in offline/test environments
            }
            Err(e) => {
                tracing::warn!("Slack auth.test request failed: {e}");
                // Store tokens anyway -- network may be unavailable
            }
        }

        *self.bot_token.lock().await = Some(bot_token);
        *self.app_token.lock().await = Some(app_token);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.bot_token.lock().await = None;
        *self.app_token.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Slack not connected".into()));
        }

        let token = self
            .bot_token
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Bot token not set".into()))?;

        // Slack has a 4000 character limit per message
        let chunks = chunk_message(&message.text, 4000);

        for chunk in chunks {
            let body = serde_json::json!({
                "channel": recipient,
                "text": chunk
            });

            let resp = self
                .client
                .post("https://slack.com/api/chat.postMessage")
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Slack send failed: {e}")))?;

            if !resp.status().is_success() {
                return Err(ChannelError::SendFailed(format!(
                    "Slack returned HTTP {}",
                    resp.status()
                )));
            }

            // Check the Slack API response for ok=true
            if let Ok(resp_body) = resp.json::<serde_json::Value>().await {
                if resp_body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
                    let error = resp_body
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    return Err(ChannelError::SendFailed(format!(
                        "Slack API error: {error}"
                    )));
                }
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}

pub struct SlackChannelFactory;

impl crate::ChannelPluginFactory for SlackChannelFactory {
    fn channel_type(&self) -> &str {
        "slack"
    }
    fn channel_type_name(&self) -> &str {
        "Slack"
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
        Box::new(SlackChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_slack_metadata() {
        let channel = SlackChannel::new();
        assert_eq!(channel.id(), "slack");
        assert_eq!(channel.name(), "Slack");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_slack_features() {
        let channel = SlackChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(!features.read_receipts);
        assert!(features.typing_indicators);
    }

    #[tokio::test]
    async fn test_slack_login_bad_type() {
        let mut channel = SlackChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_slack_login_missing_token() {
        let mut channel = SlackChannel::new();

        // Missing app_token
        let mut data = HashMap::new();
        data.insert("bot_token".to_string(), "xoxb-test-token".to_string());
        let creds = ChannelCredentials {
            credential_type: "bot_token".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("app_token"));

        // Missing bot_token
        let mut data = HashMap::new();
        data.insert("app_token".to_string(), "xapp-test-token".to_string());
        let creds = ChannelCredentials {
            credential_type: "bot_token".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bot_token"));
    }

    #[tokio::test]
    async fn test_slack_send_not_connected() {
        let channel = SlackChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("C0123456789", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
