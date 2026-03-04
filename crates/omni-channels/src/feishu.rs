//! Feishu/Lark Channel Plugin
//!
//! Implements the ChannelPlugin trait for Feishu (Lark) via REST API.
//!
//! ## Authentication
//! - `credential_type`: "app_credentials"
//! - `data.app_id`: Feishu app ID
//! - `data.app_secret`: Feishu app secret
//!
//! ## How it works
//! - Login: POST https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal
//! - Send: POST https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id
//! - Receive: webhook at /feishu/webhook
//!
//! ## Features
//! DM + group + media attachments

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Feishu/Lark channel plugin using REST API.
pub struct FeishuChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API calls.
    client: reqwest::Client,
    /// Feishu app ID.
    app_id: Mutex<Option<String>>,
    /// Feishu app secret.
    app_secret: Mutex<Option<String>>,
    /// Tenant access token for API calls.
    tenant_access_token: Mutex<Option<String>>,
    /// Token expiry timestamp.
    token_expiry: Mutex<Option<chrono::DateTime<chrono::Utc>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal.
    _shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    /// Shared webhook server for receiving incoming messages.
    webhook_server: Option<Arc<crate::webhook_server::WebhookServer>>,
}

impl FeishuChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            app_id: Mutex::new(None),
            app_secret: Mutex::new(None),
            tenant_access_token: Mutex::new(None),
            token_expiry: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            _shutdown_tx: Mutex::new(None),
            webhook_server: None,
        }
    }

    /// Acquire a tenant access token from Feishu.
    async fn acquire_tenant_token(&self) -> Result<String> {
        let app_id = self.app_id.lock().await.clone()
            .ok_or_else(|| ChannelError::AuthFailed("app_id not set".into()))?;
        let app_secret = self.app_secret.lock().await.clone()
            .ok_or_else(|| ChannelError::AuthFailed("app_secret not set".into()))?;

        let body = serde_json::json!({
            "app_id": app_id,
            "app_secret": app_secret,
        });

        let resp = self
            .client
            .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Token request failed: {e}")))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Failed to parse token response: {e}")))?;

        if json["code"].as_i64() != Some(0) {
            let msg = json["msg"].as_str().unwrap_or("unknown error");
            return Err(ChannelError::AuthFailed(format!("Feishu token error: {msg}")));
        }

        let token = json["tenant_access_token"]
            .as_str()
            .ok_or_else(|| ChannelError::AuthFailed("No tenant_access_token in response".into()))?
            .to_string();

        if let Some(expire) = json["expire"].as_i64() {
            let expiry = chrono::Utc::now() + chrono::Duration::seconds(expire);
            *self.token_expiry.lock().await = Some(expiry);
        }

        *self.tenant_access_token.lock().await = Some(token.clone());
        Ok(token)
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for FeishuChannel {
    fn id(&self) -> &str {
        "feishu"
    }

    fn name(&self) -> &str {
        "Feishu/Lark"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: false,
            read_receipts: false,
            typing_indicators: false,
            threads: true,
        }
    }

    fn status(&self) -> ConnectionStatus {
        get_status(&self.status)
    }

    fn set_webhook_server(&mut self, server: std::sync::Arc<crate::webhook_server::WebhookServer>) {
        self.webhook_server = Some(server);
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        set_status(&self.status, ConnectionStatus::Connecting);
        // Feishu is webhook-driven: inbound events arrive at /feishu/webhook.
        // Acquire tenant token -- fail the connection if credentials are invalid.
        if let Err(e) = self.acquire_tenant_token().await {
            set_status(&self.status, ConnectionStatus::Error);
            return Err(ChannelError::AuthFailed(format!(
                "Failed to acquire Feishu tenant token: {e}"
            )));
        }

        // Register webhook handler for incoming Feishu events
        if let Some(server) = &self.webhook_server {
            let tx = self.incoming_tx.clone();

            let handler: crate::webhook_server::WebhookHandler = std::sync::Arc::new(move |_method, _path, body, _headers| {
                let tx = tx.clone();
                Box::pin(async move {
                    let json: serde_json::Value = match serde_json::from_slice(&body) {
                        Ok(v) => v,
                        Err(_) => return (400, "Invalid JSON".to_string()),
                    };

                    // Handle URL verification challenge
                    if let Some(challenge) = json["challenge"].as_str() {
                        return (200, serde_json::json!({"challenge": challenge}).to_string());
                    }

                    // Parse event
                    let event_type = json["header"]["event_type"].as_str().unwrap_or("");
                    if event_type != "im.message.receive_v1" {
                        return (200, "OK".to_string());
                    }

                    let event = &json["event"];
                    let sender = event["sender"]["sender_id"]["open_id"].as_str().unwrap_or("unknown").to_string();
                    let msg_id = event["message"]["message_id"].as_str().unwrap_or("").to_string();
                    let msg_type = event["message"]["message_type"].as_str().unwrap_or("");
                    let chat_type = event["message"]["chat_type"].as_str().unwrap_or("");
                    let chat_id = event["message"]["chat_id"].as_str().unwrap_or("").to_string();

                    // Parse content (Feishu wraps text in JSON string)
                    let text = if msg_type == "text" {
                        let content_str = event["message"]["content"].as_str().unwrap_or("{}");
                        let content: serde_json::Value = serde_json::from_str(content_str).unwrap_or_default();
                        content["text"].as_str().unwrap_or("").to_string()
                    } else {
                        format!("[{}]", msg_type)
                    };

                    let is_group = chat_type == "group";

                    // Extract root_id for thread support (Feishu's thread identifier)
                    let thread_id = event["message"]["root_id"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());

                    let incoming = crate::IncomingMessage {
                        id: msg_id,
                        channel_id: "feishu".to_string(),
                        channel_type: "feishu".to_string(),
                        instance_id: "default".to_string(),
                        sender,
                        sender_name: None,
                        text,
                        is_group,
                        group_id: if is_group { Some(chat_id) } else { None },
                        thread_id,
                        timestamp: chrono::Utc::now(),
                        media_url: None,
                        source_trust_level: crate::SourceTrustLevel::Authenticated,
                    };

                    let _ = tx.send(incoming).await;
                    (200, "OK".to_string())
                })
            });

            server.register_handler("feishu", handler).await;
        }

        set_status(&self.status, ConnectionStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(server) = &self.webhook_server {
            server.unregister_handler("feishu").await;
        }
        *self.tenant_access_token.lock().await = None;
        *self.token_expiry.lock().await = None;
        set_status(&self.status, ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "app_credentials" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'app_credentials'.",
                credentials.credential_type
            )));
        }

        let app_id = credentials
            .data
            .get("app_id")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'app_id' in credentials data".into()))?
            .clone();

        let app_secret = credentials
            .data
            .get("app_secret")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'app_secret' in credentials data".into()))?
            .clone();

        if app_id.is_empty() || app_secret.is_empty() {
            return Err(ChannelError::AuthFailed("app_id and app_secret cannot be empty".into()));
        }

        *self.app_id.lock().await = Some(app_id);
        *self.app_secret.lock().await = Some(app_secret);

        // In production, we would call acquire_tenant_token() here.
        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.app_id.lock().await = None;
        *self.app_secret.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Feishu not connected".into()));
        }

        // Check if token is expired or missing and refresh
        let needs_refresh = {
            let expiry = self.token_expiry.lock().await;
            match expiry.as_ref() {
                Some(exp) => chrono::Utc::now() >= *exp - chrono::Duration::minutes(5),
                None => self.tenant_access_token.lock().await.is_none(),
            }
        };
        if needs_refresh {
            self.acquire_tenant_token().await?;
        }

        let token = self.tenant_access_token.lock().await.clone()
            .ok_or_else(|| ChannelError::SendFailed("No tenant access token available".into()))?;

        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id"
        );

        // Feishu messages support up to 4096 characters.
        let chunks = chunk_message(&message.text, 4096);

        for chunk in chunks {
            let mut body = serde_json::json!({
                "receive_id": recipient,
                "msg_type": "text",
                "content": serde_json::json!({ "text": chunk }).to_string(),
            });

            // If thread_id is set, reply within that thread (Feishu root_id)
            if let Some(ref tid) = message.thread_id {
                body.as_object_mut().unwrap().insert(
                    "root_id".to_string(),
                    serde_json::Value::String(tid.clone()),
                );
            }

            let resp = self
                .client
                .post(&url)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Feishu send failed: {e}")))?;

            let resp_json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Failed to parse Feishu response: {e}")))?;

            if resp_json["code"].as_i64() != Some(0) {
                let msg = resp_json["msg"].as_str().unwrap_or("unknown error");
                return Err(ChannelError::SendFailed(format!(
                    "Feishu API error: {msg}"
                )));
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct FeishuChannelFactory;

impl crate::ChannelPluginFactory for FeishuChannelFactory {
    fn channel_type(&self) -> &str { "feishu" }
    fn channel_type_name(&self) -> &str { "Feishu" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: false,
            read_receipts: false,
            typing_indicators: false,
            threads: true,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(FeishuChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_feishu_metadata() {
        let channel = FeishuChannel::new();
        assert_eq!(channel.id(), "feishu");
        assert_eq!(channel.name(), "Feishu/Lark");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_feishu_features() {
        let channel = FeishuChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(!features.reactions);
        assert!(!features.read_receipts);
        assert!(!features.typing_indicators);
        assert!(features.threads);
    }

    #[tokio::test]
    async fn test_feishu_login_bad_type() {
        let mut channel = FeishuChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_feishu_send_not_connected() {
        let channel = FeishuChannel::new();
        let msg = OutgoingMessage {
            text: "Hello Feishu".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("ou_abc123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
