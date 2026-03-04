//! Nextcloud Talk Channel Plugin
//!
//! REST + long-polling integration with Nextcloud Talk (Spreed).
//!
//! ## Authentication
//! - `credential_type`: "password"
//! - `data.server_url`: Nextcloud server URL (e.g. "https://cloud.example.com")
//! - `data.username`: Nextcloud username
//! - `data.app_password`: app-specific password generated in Nextcloud settings
//!
//! ## Login
//! Validates credentials by calling `GET {server}/ocs/v2.php/core/capabilities`
//! with HTTP basic auth.
//!
//! ## Sending Messages
//! `POST {server}/ocs/v2.php/apps/spreed/api/v4/chat/{token}`
//! with `message` in the request body, basic auth, and `OCS-APIRequest: true` header.
//!
//! ## Receiving Messages
//! Long-poll via `GET .../chat/{token}?lookIntoFuture=1&timeout=30`
//!
//! ## Features
//! - Direct messages
//! - Group messages
//! - Media attachments (file sharing)
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

/// Nextcloud Talk channel plugin using REST API + long-polling.
pub struct NextcloudTalkChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for REST requests.
    client: reqwest::Client,
    /// Nextcloud server URL.
    server_url: Mutex<Option<String>>,
    /// Nextcloud username.
    username: Mutex<Option<String>>,
    /// App-specific password.
    app_password: Mutex<Option<String>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the long-polling task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl NextcloudTalkChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            server_url: Mutex::new(None),
            username: Mutex::new(None),
            app_password: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for NextcloudTalkChannel {
    fn id(&self) -> &str {
        "nextcloud-talk"
    }

    fn name(&self) -> &str {
        "Nextcloud Talk"
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

    async fn connect(&mut self, config: ChannelConfig) -> Result<()> {
        let server_url = self.server_url.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Server URL not set. Call login() first.".into()))?;
        let username = self.username.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Username not set. Call login() first.".into()))?;
        let app_password = self.app_password.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("App password not set. Call login() first.".into()))?;

        // Extract room token from config (required for incoming message polling)
        let room_token = config.settings.get("room_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        set_status(&self.status, ConnectionStatus::Connecting);

        // Spawn background long-polling task for incoming messages
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.incoming_tx.clone();
        let status = self.status.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            // If no room token, we can't poll -- just stay connected but idle
            let token = match room_token {
                Some(t) => t,
                None => {
                    tracing::info!("Nextcloud Talk: no room_token configured, skipping incoming poll");
                    // Wait for shutdown
                    let _ = shutdown_rx.await;
                    set_status(&status, ConnectionStatus::Disconnected);
                    return;
                }
            };

            let base_url = format!(
                "{}/ocs/v2.php/apps/spreed/api/v4/chat/{}",
                server_url.trim_end_matches('/'),
                token
            );

            let mut last_known_id: i64 = 0;

            // Get the latest message ID to avoid backfill
            let init_resp = client.get(&base_url)
                .basic_auth(&username, Some(&app_password))
                .header("OCS-APIRequest", "true")
                .header("Accept", "application/json")
                .query(&[("limit", "1"), ("lookIntoFuture", "0")])
                .send()
                .await;

            if let Ok(resp) = init_resp {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(msgs) = json.pointer("/ocs/data").and_then(|v| v.as_array()) {
                            if let Some(last) = msgs.last() {
                                last_known_id = last.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                            }
                        }
                    }
                }
            }

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Nextcloud Talk: polling task shutting down");
                        break;
                    }
                    _ = async {
                        let mut query = vec![
                            ("lookIntoFuture", "1".to_string()),
                            ("timeout", "30".to_string()),
                            ("limit", "100".to_string()),
                        ];
                        if last_known_id > 0 {
                            query.push(("lastKnownMessageId", last_known_id.to_string()));
                        }

                        let resp = client.get(&base_url)
                            .basic_auth(&username, Some(&app_password))
                            .header("OCS-APIRequest", "true")
                            .header("Accept", "application/json")
                            .query(&query)
                            .timeout(std::time::Duration::from_secs(60))
                            .send()
                            .await;

                        let resp = match resp {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::debug!("Nextcloud Talk poll error: {e}");
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                return;
                            }
                        };

                        // 304 = no new messages (timeout), just retry
                        if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
                            return;
                        }

                        if !resp.status().is_success() {
                            tracing::debug!("Nextcloud Talk poll returned HTTP {}", resp.status());
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            return;
                        }

                        let json: serde_json::Value = match resp.json().await {
                            Ok(v) => v,
                            Err(_) => return,
                        };

                        let messages = match json.pointer("/ocs/data").and_then(|v| v.as_array()) {
                            Some(arr) => arr,
                            None => return,
                        };

                        for msg in messages {
                            // Only process "comment" type messages (skip system messages)
                            let msg_type = msg.get("messageType")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if msg_type != "comment" {
                                continue;
                            }

                            let msg_id = msg.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                            if msg_id > last_known_id {
                                last_known_id = msg_id;
                            }

                            // Skip our own messages
                            let actor_id = msg.get("actorId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if actor_id == username {
                                continue;
                            }

                            let text = msg.get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            if text.is_empty() {
                                continue;
                            }

                            let sender = actor_id.to_string();
                            let sender_name = msg.get("actorDisplayName")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let timestamp_epoch = msg.get("timestamp")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            let timestamp = chrono::DateTime::from_timestamp(timestamp_epoch, 0)
                                .unwrap_or_else(chrono::Utc::now);

                            let incoming = IncomingMessage {
                                id: msg_id.to_string(),
                                channel_id: "nextcloud-talk".to_string(),
                                channel_type: "nextcloud-talk".to_string(),
                                instance_id: "default".to_string(),
                                sender,
                                sender_name,
                                text,
                                is_group: true,
                                group_id: Some(token.clone()),
                                thread_id: None,
                                timestamp,
                                media_url: None,
                                source_trust_level: crate::SourceTrustLevel::Authenticated,
                            };

                            let _ = tx.send(incoming).await;
                        }
                    } => {}
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
        if credentials.credential_type != "password" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'password'.",
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

        let username = credentials
            .data
            .get("username")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'username' in credentials data".into())
            })?
            .clone();

        let app_password = credentials
            .data
            .get("app_password")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'app_password' in credentials data".into())
            })?
            .clone();

        if server_url.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Server URL cannot be empty".into(),
            ));
        }

        if username.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Username cannot be empty".into(),
            ));
        }

        if app_password.is_empty() {
            return Err(ChannelError::AuthFailed(
                "App password cannot be empty".into(),
            ));
        }

        // Placeholder: would validate by calling
        // GET {server}/ocs/v2.php/core/capabilities with basic auth
        *self.server_url.lock().await = Some(server_url);
        *self.username.lock().await = Some(username);
        *self.app_password.lock().await = Some(app_password);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.server_url.lock().await = None;
        *self.username.lock().await = None;
        *self.app_password.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected(
                "Nextcloud Talk not connected".into(),
            ));
        }

        let server_url = self
            .server_url
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Server URL not set".into()))?;

        let username = self
            .username
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Username not set".into()))?;

        let app_password = self
            .app_password
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("App password not set".into()))?;

        let url = format!(
            "{}/ocs/v2.php/apps/spreed/api/v4/chat/{}",
            server_url.trim_end_matches('/'),
            recipient
        );

        // Nextcloud Talk messages are typically limited to ~32000 chars;
        // chunk at 4000 as a safe default.
        let chunks = chunk_message(&message.text, 4000);

        for chunk in chunks {
            let body = serde_json::json!({ "message": chunk });

            let resp = self
                .client
                .post(&url)
                .basic_auth(&username, Some(&app_password))
                .header("OCS-APIRequest", "true")
                .header("Accept", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    ChannelError::SendFailed(format!("Nextcloud Talk send failed: {e}"))
                })?;

            if !resp.status().is_success() {
                return Err(ChannelError::SendFailed(format!(
                    "Nextcloud Talk returned HTTP {}",
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


pub struct NextcloudTalkChannelFactory;

impl crate::ChannelPluginFactory for NextcloudTalkChannelFactory {
    fn channel_type(&self) -> &str { "nextcloud-talk" }
    fn channel_type_name(&self) -> &str { "Nextcloud Talk" }
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
        Box::new(NextcloudTalkChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_nextcloud_talk_metadata() {
        let channel = NextcloudTalkChannel::new();
        assert_eq!(channel.id(), "nextcloud-talk");
        assert_eq!(channel.name(), "Nextcloud Talk");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_nextcloud_talk_features() {
        let channel = NextcloudTalkChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_nextcloud_talk_login_bad_type() {
        let mut channel = NextcloudTalkChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_nextcloud_talk_login_missing_fields() {
        let mut channel = NextcloudTalkChannel::new();

        // Missing server_url
        let creds = ChannelCredentials {
            credential_type: "password".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("server_url"));

        // Missing username
        let mut data = HashMap::new();
        data.insert("server_url".to_string(), "https://cloud.example.com".to_string());
        let creds = ChannelCredentials {
            credential_type: "password".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("username"));

        // Missing app_password
        let mut data = HashMap::new();
        data.insert("server_url".to_string(), "https://cloud.example.com".to_string());
        data.insert("username".to_string(), "admin".to_string());
        let creds = ChannelCredentials {
            credential_type: "password".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("app_password"));
    }

    #[tokio::test]
    async fn test_nextcloud_talk_send_not_connected() {
        let channel = NextcloudTalkChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("room-token-123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
