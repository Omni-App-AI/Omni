//! Zalo Channel Plugin
//!
//! Integrates with the Zalo Open API for messaging.
//!
//! ## Authentication
//! - `credential_type`: "api_key"
//! - `data.access_token`: Zalo OA access token
//! - `data.oa_secret_key` (optional): OA secret key for webhook HMAC verification
//!
//! ## Login Validation
//! GET https://openapi.zalo.me/v2.0/oa/getoa with access_token header
//!
//! ## Sending Messages
//! POST https://openapi.zalo.me/v3.0/oa/message/cs with bearer token
//!
//! ## Receiving Messages
//! Via shared webhook server at /zalo, verifying HMAC signature with OA secret key
//!
//! ## Features
//! - Direct messages
//! - Media attachments

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Zalo channel plugin using the Zalo Open API.
pub struct ZaloChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// Zalo OA access token.
    access_token: Mutex<Option<String>>,
    /// OA secret key for webhook HMAC verification.
    oa_secret_key: Arc<Mutex<Option<String>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shared webhook server for receiving incoming messages.
    webhook_server: Option<Arc<crate::webhook_server::WebhookServer>>,
}

impl ZaloChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            access_token: Mutex::new(None),
            oa_secret_key: Arc::new(Mutex::new(None)),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            webhook_server: None,
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for ZaloChannel {
    fn id(&self) -> &str {
        "zalo"
    }

    fn name(&self) -> &str {
        "Zalo"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: false,
            media_attachments: true,
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
        let token = self.access_token.lock().await;
        if token.is_none() {
            return Err(ChannelError::Config(
                "Access token not set. Call login() first.".into(),
            ));
        }
        drop(token);

        // Register incoming webhook handler
        if let Some(server) = &self.webhook_server {
            let tx = self.incoming_tx.clone();
            let secret_key = self.oa_secret_key.clone();

            let handler: crate::webhook_server::WebhookHandler = Arc::new(move |_method, _path, body, headers| {
                let tx = tx.clone();
                let secret_key = secret_key.clone();
                Box::pin(async move {
                    let json: serde_json::Value = match serde_json::from_slice(&body) {
                        Ok(v) => v,
                        Err(_) => return (400, "Invalid JSON".to_string()),
                    };

                    // Verify HMAC signature if OA secret key is configured
                    if let Some(key) = secret_key.lock().await.as_ref() {
                        if let Some(mac_header) = headers.get("x-zalosignature")
                            .or_else(|| headers.get("x-zalo-signature"))
                        {
                            use hmac::{Hmac, Mac};
                            use sha2::Sha256;

                            type HmacSha256 = Hmac<Sha256>;
                            if let Ok(mut mac) = HmacSha256::new_from_slice(key.as_bytes()) {
                                mac.update(&body);
                                let result = mac.finalize();
                                let expected = base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    result.into_bytes(),
                                );
                                if expected != *mac_header {
                                    tracing::warn!("Zalo webhook: HMAC verification failed");
                                    return (403, "Invalid signature".to_string());
                                }
                            }
                        }
                    }

                    // Only process user_send_text events
                    let event_name = json.get("event_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if event_name != "user_send_text" {
                        return (200, "OK".to_string());
                    }

                    let sender = json.get("sender")
                        .and_then(|v| v.get("id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let text = json.get("message")
                        .and_then(|v| v.get("text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if text.is_empty() {
                        return (200, "OK".to_string());
                    }

                    let msg_id = json.get("message")
                        .and_then(|v| v.get("msg_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let timestamp_ms = json.get("timestamp")
                        .and_then(|v| v.as_i64())
                        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

                    let timestamp = chrono::DateTime::from_timestamp_millis(timestamp_ms)
                        .unwrap_or_else(chrono::Utc::now);

                    let incoming = IncomingMessage {
                        id: if msg_id.is_empty() { uuid::Uuid::new_v4().to_string() } else { msg_id },
                        channel_id: "zalo".to_string(),
                        channel_type: "zalo".to_string(),
                        instance_id: "default".to_string(),
                        sender,
                        sender_name: None,
                        text,
                        is_group: false,
                        group_id: None,
                        thread_id: None,
                        timestamp,
                        media_url: None,
                        source_trust_level: crate::SourceTrustLevel::Authenticated,
                    };

                    let _ = tx.send(incoming).await;
                    (200, "OK".to_string())
                })
            });

            server.register_handler("zalo", handler).await;
        }

        set_status(&self.status, ConnectionStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(server) = &self.webhook_server {
            server.unregister_handler("zalo").await;
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

        let access_token = credentials
            .data
            .get("access_token")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'access_token' in credentials data".into())
            })?
            .clone();

        if access_token.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Access token cannot be empty".into(),
            ));
        }

        // Store optional OA secret key for webhook verification
        if let Some(key) = credentials.data.get("oa_secret_key") {
            if !key.is_empty() {
                *self.oa_secret_key.lock().await = Some(key.clone());
            }
        }

        // Validate token by calling the getoa endpoint
        let resp = self
            .client
            .get("https://openapi.zalo.me/v2.0/oa/getoa")
            .header("access_token", &access_token)
            .send()
            .await;

        match resp {
            Ok(r) => {
                if r.status() == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(ChannelError::AuthFailed("Zalo access token is invalid or expired".into()));
                }
                if r.status().is_success() {
                    if let Ok(body) = r.json::<serde_json::Value>().await {
                        if let Some(err_code) = body.get("error").and_then(|v| v.as_i64()) {
                            if err_code != 0 {
                                let msg = body.get("message").and_then(|v| v.as_str()).unwrap_or("unknown error");
                                return Err(ChannelError::AuthFailed(format!("Zalo OA validation failed: {msg}")));
                            }
                        }
                    }
                    tracing::info!("Zalo OA token validated successfully");
                } else {
                    tracing::warn!("Zalo OA validation returned HTTP {} -- accepting token with caution", r.status());
                }
            }
            Err(e) => {
                tracing::warn!("Zalo OA validation request failed: {e} -- accepting token (may be offline)");
            }
        }

        *self.access_token.lock().await = Some(access_token);
        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.access_token.lock().await = None;
        *self.oa_secret_key.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Zalo not connected".into()));
        }

        let token = self
            .access_token
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Access token not set".into()))?;

        let chunks = chunk_message(&message.text, 2000);

        for chunk in chunks {
            let body = serde_json::json!({
                "recipient": {
                    "user_id": recipient
                },
                "message": {
                    "text": chunk
                }
            });

            let resp = self
                .client
                .post("https://openapi.zalo.me/v3.0/oa/message/cs")
                .header("access_token", &token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Zalo send failed: {e}")))?;

            if !resp.status().is_success() {
                return Err(ChannelError::SendFailed(format!(
                    "Zalo returned HTTP {}",
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


pub struct ZaloChannelFactory;

impl crate::ChannelPluginFactory for ZaloChannelFactory {
    fn channel_type(&self) -> &str { "zalo" }
    fn channel_type_name(&self) -> &str { "Zalo" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: false,
            media_attachments: true,
            reactions: false,
            read_receipts: false,
            typing_indicators: false,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(ZaloChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_zalo_metadata() {
        let channel = ZaloChannel::new();
        assert_eq!(channel.id(), "zalo");
        assert_eq!(channel.name(), "Zalo");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_zalo_features() {
        let channel = ZaloChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(!features.group_messages);
        assert!(features.media_attachments);
        assert!(!features.reactions);
        assert!(!features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_zalo_login_bad_type() {
        let mut channel = ZaloChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_zalo_send_not_connected() {
        let channel = ZaloChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("user123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
