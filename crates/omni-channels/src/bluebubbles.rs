//! BlueBubbles Channel Plugin
//!
//! Integrates with the BlueBubbles REST API for iMessage on non-Apple platforms.
//!
//! ## Authentication
//! - `credential_type`: "api_key"
//! - `data.server_url`: BlueBubbles server URL (e.g., "http://192.168.1.100:1234")
//! - `data.password`: BlueBubbles server password
//!
//! ## Login Validation
//! GET {server_url}/api/v1/server/info?password={pw}
//!
//! ## Sending Messages
//! POST {server_url}/api/v1/message/text?password={pw} with JSON body
//!
//! ## Receiving Messages
//! Polling GET {server_url}/api/v1/message?after={timestamp}&password={pw}
//!
//! ## Features
//! - Direct messages
//! - Group messages
//! - Media attachments
//! - Reactions
//! - Read receipts

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// BlueBubbles channel plugin using the BlueBubbles REST API.
pub struct BlueBubblesChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// BlueBubbles server URL.
    server_url: Arc<Mutex<Option<String>>>,
    /// BlueBubbles server password.
    password: Arc<Mutex<Option<String>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the polling task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl BlueBubblesChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            server_url: Arc::new(Mutex::new(None)),
            password: Arc::new(Mutex::new(None)),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for BlueBubblesChannel {
    fn id(&self) -> &str {
        "bluebubbles"
    }

    fn name(&self) -> &str {
        "BlueBubbles"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: true,
            typing_indicators: false,
            threads: false,
        }
    }

    fn status(&self) -> ConnectionStatus {
        get_status(&self.status)
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        let server_url = self.server_url.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Server URL not set. Call login() first.".into()))?;
        let password = self.password.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Password not set. Call login() first.".into()))?;

        set_status(&self.status, ConnectionStatus::Connecting);

        // Spawn background polling task for incoming messages
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.incoming_tx.clone();
        let status = self.status.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let mut last_timestamp = chrono::Utc::now().timestamp_millis();

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("BlueBubbles: polling task shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
                        let poll_url = format!(
                            "{}/api/v1/message",
                            server_url.trim_end_matches('/')
                        );

                        let resp = client.get(&poll_url)
                            .query(&[
                                ("password", password.as_str()),
                                ("after", &last_timestamp.to_string()),
                                ("sort", "ASC"),
                                ("limit", "100"),
                            ])
                            .send()
                            .await;

                        let resp = match resp {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::debug!("BlueBubbles poll error: {e}");
                                continue;
                            }
                        };

                        if !resp.status().is_success() {
                            tracing::debug!("BlueBubbles poll returned HTTP {}", resp.status());
                            continue;
                        }

                        let json: serde_json::Value = match resp.json().await {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        let messages = match json.get("data").and_then(|v| v.as_array()) {
                            Some(arr) => arr,
                            None => continue,
                        };

                        for msg in messages {
                            // Skip messages we sent
                            if msg.get("isFromMe").and_then(|v| v.as_bool()).unwrap_or(false) {
                                continue;
                            }

                            let text = msg.get("text")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            if text.is_empty() {
                                continue;
                            }

                            let sender = msg.get("handle")
                                .and_then(|v| v.get("address"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let msg_id = msg.get("guid")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let date_created = msg.get("dateCreated")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            if date_created > last_timestamp {
                                last_timestamp = date_created;
                            }

                            let timestamp = chrono::DateTime::from_timestamp_millis(date_created)
                                .unwrap_or_else(chrono::Utc::now);

                            let chat_id = msg.get("chats")
                                .and_then(|v| v.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|c| c.get("chatIdentifier"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let is_group = msg.get("chats")
                                .and_then(|v| v.as_array())
                                .and_then(|arr| arr.first())
                                .and_then(|c| c.get("participants"))
                                .and_then(|v| v.as_array())
                                .map(|arr| arr.len() > 2)
                                .unwrap_or(false);

                            let incoming = IncomingMessage {
                                id: if msg_id.is_empty() { uuid::Uuid::new_v4().to_string() } else { msg_id },
                                channel_id: "bluebubbles".to_string(),
                                channel_type: "bluebubbles".to_string(),
                                instance_id: "default".to_string(),
                                sender,
                                sender_name: None,
                                text,
                                is_group,
                                group_id: if is_group { chat_id } else { None },
                                thread_id: None,
                                timestamp,
                                media_url: None,
                                source_trust_level: crate::SourceTrustLevel::Authenticated,
                            };

                            let _ = tx.send(incoming).await;
                        }
                    }
                }
            }

            set_status(&status, ConnectionStatus::Disconnected);
        });

        set_status(&self.status, ConnectionStatus::Connected);
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

        let password = credentials
            .data
            .get("password")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'password' in credentials data".into())
            })?
            .clone();

        if server_url.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Server URL cannot be empty".into(),
            ));
        }

        if password.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Password cannot be empty".into(),
            ));
        }

        // Validate credentials by calling server info endpoint
        let info_url = format!(
            "{}/api/v1/server/info",
            server_url.trim_end_matches('/')
        );

        let resp = self.client.get(&info_url)
            .query(&[("password", &password)])
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                tracing::info!("BlueBubbles server validated successfully");
            }
            Ok(r) => {
                tracing::warn!("BlueBubbles server info returned HTTP {}", r.status());
            }
            Err(e) => {
                tracing::warn!("BlueBubbles server validation request failed: {e}");
            }
        }

        *self.server_url.lock().await = Some(server_url);
        *self.password.lock().await = Some(password);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.server_url.lock().await = None;
        *self.password.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected(
                "BlueBubbles not connected".into(),
            ));
        }

        let server_url = self.server_url.lock().await.clone()
            .ok_or_else(|| ChannelError::NotConnected("Server URL not set".into()))?;

        let password = self.password.lock().await.clone()
            .ok_or_else(|| ChannelError::NotConnected("Password not set".into()))?;

        let send_url = format!(
            "{}/api/v1/message/text",
            server_url.trim_end_matches('/')
        );

        let chunks = chunk_message(&message.text, 8000);

        for chunk in chunks {
            let body = serde_json::json!({
                "chatGuid": recipient,
                "message": chunk,
                "method": "private-api"
            });

            let resp = self
                .client
                .post(&send_url)
                .query(&[("password", &password)])
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    ChannelError::SendFailed(format!("BlueBubbles send failed: {e}"))
                })?;

            if !resp.status().is_success() {
                return Err(ChannelError::SendFailed(format!(
                    "BlueBubbles returned HTTP {}",
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


pub struct BlueBubblesChannelFactory;

impl crate::ChannelPluginFactory for BlueBubblesChannelFactory {
    fn channel_type(&self) -> &str { "bluebubbles" }
    fn channel_type_name(&self) -> &str { "BlueBubbles" }
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
        Box::new(BlueBubblesChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_bluebubbles_metadata() {
        let channel = BlueBubblesChannel::new();
        assert_eq!(channel.id(), "bluebubbles");
        assert_eq!(channel.name(), "BlueBubbles");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_bluebubbles_features() {
        let channel = BlueBubblesChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_bluebubbles_login_bad_type() {
        let mut channel = BlueBubblesChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_bluebubbles_login_missing_fields() {
        let mut channel = BlueBubblesChannel::new();

        // Missing password
        let mut data = HashMap::new();
        data.insert("server_url".to_string(), "http://localhost:1234".to_string());
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("password"));

        // Missing server_url
        let mut data = HashMap::new();
        data.insert("password".to_string(), "secret".to_string());
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("server_url"));
    }

    #[tokio::test]
    async fn test_bluebubbles_send_not_connected() {
        let channel = BlueBubblesChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("iMessage;+;chat123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
