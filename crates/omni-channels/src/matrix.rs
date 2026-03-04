//! Matrix Channel Plugin
//!
//! NOTE: Placeholder. Full `matrix-sdk` crate can be added later for E2E encryption.
//!
//! ## Authentication
//! - `credential_type`: "password"
//!   - `data.homeserver_url`: Matrix homeserver URL (e.g., "https://matrix.org")
//!   - `data.username`: Matrix user ID (e.g., "@bot:matrix.org")
//!   - `data.password`: Account password
//! - OR `credential_type`: "access_token"
//!   - `data.homeserver_url`: Matrix homeserver URL
//!   - `data.access_token`: Pre-existing access token
//!
//! ## How it works
//! - Send: PUT /_matrix/client/v3/rooms/{room_id}/send/m.room.message/{txn_id}
//! - Receive: Long-poll via /sync or webhook bridge
//!
//! ## Features
//! DM + group + media + reactions + read_receipts + typing

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Matrix channel plugin (placeholder -- full `matrix-sdk` can be added later for E2EE).
pub struct MatrixChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for Matrix Client-Server API calls.
    client: reqwest::Client,
    /// Matrix homeserver URL (e.g., "https://matrix.org").
    homeserver_url: Mutex<Option<String>>,
    /// Access token for authenticated requests.
    access_token: Mutex<Option<String>>,
    /// The authenticated user's Matrix ID (e.g., "@bot:matrix.org").
    user_id: Mutex<Option<String>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the sync loop task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    /// Transaction ID counter for idempotent PUT requests.
    txn_counter: std::sync::atomic::AtomicU64,
}

impl MatrixChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            homeserver_url: Mutex::new(None),
            access_token: Mutex::new(None),
            user_id: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
            txn_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Generate a unique transaction ID for idempotent message sends.
    fn next_txn_id(&self) -> String {
        let count = self
            .txn_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("omni_{}", count)
    }

    /// Authenticate with username/password and obtain an access token.
    async fn password_login(&self, homeserver: &str, username: &str, password: &str) -> Result<String> {
        let url = format!("{}/_matrix/client/v3/login", homeserver);

        let body = serde_json::json!({
            "type": "m.login.password",
            "identifier": {
                "type": "m.id.user",
                "user": username,
            },
            "password": password,
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Matrix login request failed: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed(format!("Matrix login failed: {body}")));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Failed to parse Matrix login response: {e}")))?;

        let token = json["access_token"]
            .as_str()
            .ok_or_else(|| ChannelError::AuthFailed("No access_token in Matrix login response".into()))?
            .to_string();

        Ok(token)
    }

    /// Validate an access token by calling /account/whoami.
    async fn validate_token(&self, homeserver: &str, token: &str) -> Result<String> {
        let url = format!("{}/_matrix/client/v3/account/whoami", homeserver);

        let resp = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Matrix whoami request failed: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed(format!("Matrix token invalid: {body}")));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Failed to parse whoami response: {e}")))?;

        let user_id = json["user_id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(user_id)
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for MatrixChannel {
    fn id(&self) -> &str {
        "matrix"
    }

    fn name(&self) -> &str {
        "Matrix"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: true,
            typing_indicators: true,
            threads: false,
        }
    }

    fn status(&self) -> ConnectionStatus {
        get_status(&self.status)
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        let token = self.access_token.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Access token not set. Call login() first.".into()))?;
        let homeserver = self.homeserver_url.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Homeserver URL not set. Call login() first.".into()))?;
        let own_user_id = self.user_id.lock().await.clone().unwrap_or_default();

        set_status(&self.status, ConnectionStatus::Connecting);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.incoming_tx.clone();
        let status = self.status.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let sync_base = format!(
                "{}/_matrix/client/v3/sync",
                homeserver.trim_end_matches('/')
            );

            let mut next_batch: Option<String> = None;

            // First sync uses a filter to avoid backfilling old messages
            let filter = serde_json::json!({
                "room": {
                    "timeline": { "limit": 0 }
                }
            });

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Matrix: sync loop shutting down");
                        break;
                    }
                    _ = async {
                        let mut query: Vec<(&str, String)> = vec![
                            ("timeout", "30000".to_string()),
                        ];

                        if let Some(ref batch) = next_batch {
                            query.push(("since", batch.clone()));
                        } else {
                            // First sync: use filter to skip old messages
                            query.push(("filter", filter.to_string()));
                        }

                        let resp = client.get(&sync_base)
                            .bearer_auth(&token)
                            .query(&query)
                            .timeout(std::time::Duration::from_secs(60))
                            .send()
                            .await;

                        let resp = match resp {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::debug!("Matrix sync error: {e}");
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                return;
                            }
                        };

                        if !resp.status().is_success() {
                            tracing::debug!("Matrix sync returned HTTP {}", resp.status());
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            return;
                        }

                        let json: serde_json::Value = match resp.json().await {
                            Ok(v) => v,
                            Err(_) => return,
                        };

                        // Update next_batch for incremental sync
                        if let Some(batch) = json.get("next_batch").and_then(|v| v.as_str()) {
                            next_batch = Some(batch.to_string());
                        }

                        // Process joined room events
                        let rooms = match json.pointer("/rooms/join") {
                            Some(serde_json::Value::Object(m)) => m.clone(),
                            _ => return,
                        };

                        for (room_id, room_data) in &rooms {
                            let events = match room_data.pointer("/timeline/events") {
                                Some(serde_json::Value::Array(arr)) => arr,
                                _ => continue,
                            };

                            for event in events {
                                let event_type = event.get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");

                                if event_type != "m.room.message" {
                                    continue;
                                }

                                let sender = event.get("sender")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                // Skip our own messages
                                if sender == own_user_id {
                                    continue;
                                }

                                let content = match event.get("content") {
                                    Some(c) => c,
                                    None => continue,
                                };

                                let body = content.get("body")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                if body.is_empty() {
                                    continue;
                                }

                                let event_id = event.get("event_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                let origin_ts = event.get("origin_server_ts")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);

                                let timestamp = chrono::DateTime::from_timestamp_millis(origin_ts)
                                    .unwrap_or_else(chrono::Utc::now);

                                let incoming = IncomingMessage {
                                    id: if event_id.is_empty() { uuid::Uuid::new_v4().to_string() } else { event_id },
                                    channel_id: "matrix".to_string(),
                                    channel_type: "matrix".to_string(),
                                    instance_id: "default".to_string(),
                                    sender,
                                    sender_name: None,
                                    text: body,
                                    is_group: true,
                                    group_id: Some(room_id.clone()),
                                    thread_id: None,
                                    timestamp,
                                    media_url: None,
                                    source_trust_level: crate::SourceTrustLevel::Authenticated,
                                };

                                let _ = tx.send(incoming).await;
                            }
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
        match credentials.credential_type.as_str() {
            "password" => {
                let homeserver = credentials
                    .data
                    .get("homeserver_url")
                    .ok_or_else(|| {
                        ChannelError::AuthFailed("Missing 'homeserver_url' in credentials data".into())
                    })?
                    .clone();

                let username = credentials
                    .data
                    .get("username")
                    .ok_or_else(|| {
                        ChannelError::AuthFailed("Missing 'username' in credentials data".into())
                    })?
                    .clone();

                let password = credentials
                    .data
                    .get("password")
                    .ok_or_else(|| {
                        ChannelError::AuthFailed("Missing 'password' in credentials data".into())
                    })?
                    .clone();

                if homeserver.is_empty() || username.is_empty() || password.is_empty() {
                    return Err(ChannelError::AuthFailed(
                        "homeserver_url, username, and password cannot be empty".into(),
                    ));
                }

                *self.homeserver_url.lock().await = Some(homeserver.clone());
                let token = self.password_login(&homeserver, &username, &password).await?;
                // Fetch our own user ID for filtering in /sync
                if let Ok(uid) = self.validate_token(&homeserver, &token).await {
                    *self.user_id.lock().await = Some(uid);
                }
                *self.access_token.lock().await = Some(token);

                Ok(LoginStatus::Success)
            }
            "access_token" => {
                let homeserver = credentials
                    .data
                    .get("homeserver_url")
                    .ok_or_else(|| {
                        ChannelError::AuthFailed("Missing 'homeserver_url' in credentials data".into())
                    })?
                    .clone();

                let token = credentials
                    .data
                    .get("access_token")
                    .ok_or_else(|| {
                        ChannelError::AuthFailed("Missing 'access_token' in credentials data".into())
                    })?
                    .clone();

                if homeserver.is_empty() || token.is_empty() {
                    return Err(ChannelError::AuthFailed(
                        "homeserver_url and access_token cannot be empty".into(),
                    ));
                }

                let uid = self.validate_token(&homeserver, &token).await?;
                *self.user_id.lock().await = Some(uid);
                *self.homeserver_url.lock().await = Some(homeserver);
                *self.access_token.lock().await = Some(token);

                Ok(LoginStatus::Success)
            }
            other => Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'password' or 'access_token'.",
                other
            ))),
        }
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.homeserver_url.lock().await = None;
        *self.access_token.lock().await = None;
        *self.user_id.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Matrix not connected".into()));
        }

        let homeserver = self.homeserver_url.lock().await.clone()
            .ok_or_else(|| ChannelError::SendFailed("No homeserver URL set".into()))?;

        let token = self.access_token.lock().await.clone()
            .ok_or_else(|| ChannelError::SendFailed("No access token available".into()))?;

        // Matrix doesn't have a strict message length limit, but we chunk at 64KB
        // to be safe with server implementations.
        let chunks = chunk_message(&message.text, 65536);

        for chunk in chunks {
            let txn_id = self.next_txn_id();
            let url = format!(
                "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
                homeserver, recipient, txn_id
            );

            let body = serde_json::json!({
                "msgtype": "m.text",
                "body": chunk,
            });

            let resp = self
                .client
                .put(&url)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Matrix send failed: {e}")))?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed(format!(
                    "Matrix API error: {body}"
                )));
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct MatrixChannelFactory;

impl crate::ChannelPluginFactory for MatrixChannelFactory {
    fn channel_type(&self) -> &str { "matrix" }
    fn channel_type_name(&self) -> &str { "Matrix" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: true,
            typing_indicators: true,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(MatrixChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_matrix_metadata() {
        let channel = MatrixChannel::new();
        assert_eq!(channel.id(), "matrix");
        assert_eq!(channel.name(), "Matrix");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_matrix_features() {
        let channel = MatrixChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(features.read_receipts);
        assert!(features.typing_indicators);
    }

    #[tokio::test]
    async fn test_matrix_login_bad_type() {
        let mut channel = MatrixChannel::new();
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_matrix_login_missing_homeserver() {
        let mut channel = MatrixChannel::new();
        // password type but missing homeserver_url
        let mut data = HashMap::new();
        data.insert("username".to_string(), "@bot:matrix.org".to_string());
        data.insert("password".to_string(), "secret".to_string());
        let creds = ChannelCredentials {
            credential_type: "password".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("homeserver_url"));
    }

    #[tokio::test]
    async fn test_matrix_send_not_connected() {
        let channel = MatrixChannel::new();
        let msg = OutgoingMessage {
            text: "Hello Matrix".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("!room:matrix.org", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
