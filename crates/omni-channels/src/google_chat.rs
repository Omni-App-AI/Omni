//! Google Chat Channel Plugin
//!
//! Implements the ChannelPlugin trait for Google Chat via REST API with JWT authentication.
//!
//! ## Authentication
//! - `credential_type`: "service_account"
//! - `data.service_account_json`: JSON string of the service account key file
//!
//! ## How it works
//! - Login: Parse service account key, generate JWT, exchange for OAuth2 token
//! - Send: POST https://chat.googleapis.com/v1/{space}/messages
//! - Receive: webhook at /google-chat/webhook
//!
//! ## Features
//! DM + group messaging

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Google Chat channel plugin using REST API + service account JWT.
pub struct GoogleChatChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API calls.
    client: reqwest::Client,
    /// Parsed service account key JSON.
    service_account_key: Mutex<Option<serde_json::Value>>,
    /// OAuth2 bearer token obtained from JWT exchange.
    oauth_token: Mutex<Option<String>>,
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

impl GoogleChatChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            service_account_key: Mutex::new(None),
            oauth_token: Mutex::new(None),
            token_expiry: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            _shutdown_tx: Mutex::new(None),
            webhook_server: None,
        }
    }

    /// Build a JWT and exchange it for an OAuth2 access token.
    async fn acquire_oauth_token(&self) -> Result<String> {
        let (client_email, private_key_pem) = {
            let guard = self.service_account_key.lock().await;
            let key = guard.as_ref()
                .ok_or_else(|| ChannelError::AuthFailed("Service account key not set".into()))?;

            let email = key["client_email"].as_str()
                .ok_or_else(|| ChannelError::AuthFailed("Missing client_email in service account".into()))?
                .to_string();
            let pem = key["private_key"].as_str()
                .ok_or_else(|| ChannelError::AuthFailed("Missing private_key in service account".into()))?
                .to_string();
            (email, pem)
        };

        let now = chrono::Utc::now().timestamp();
        let claims = serde_json::json!({
            "iss": client_email,
            "scope": "https://www.googleapis.com/auth/chat.bot",
            "aud": "https://oauth2.googleapis.com/token",
            "iat": now,
            "exp": now + 3600,
        });

        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
            .map_err(|e| ChannelError::AuthFailed(format!("Invalid RSA private key: {e}")))?;
        let jwt = jsonwebtoken::encode(&header, &claims, &encoding_key)
            .map_err(|e| ChannelError::AuthFailed(format!("JWT encoding failed: {e}")))?;

        let resp = self.client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Token exchange request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(ChannelError::AuthFailed(format!(
                "Token exchange failed with HTTP {}",
                resp.status()
            )));
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| ChannelError::AuthFailed(format!("Failed to parse token response: {e}")))?;
        let token = body["access_token"].as_str()
            .ok_or_else(|| ChannelError::AuthFailed("No access_token in token response".into()))?
            .to_string();

        if let Some(expires_in) = body["expires_in"].as_i64() {
            *self.token_expiry.lock().await = Some(chrono::Utc::now() + chrono::Duration::seconds(expires_in));
        }
        *self.oauth_token.lock().await = Some(token.clone());
        Ok(token)
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for GoogleChatChannel {
    fn id(&self) -> &str {
        "google-chat"
    }

    fn name(&self) -> &str {
        "Google Chat"
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

    fn set_webhook_server(&mut self, server: std::sync::Arc<crate::webhook_server::WebhookServer>) {
        self.webhook_server = Some(server);
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        set_status(&self.status, ConnectionStatus::Connecting);
        // Google Chat is webhook-driven: inbound messages arrive at /google-chat/webhook.
        // Attempt to acquire an OAuth token; if it fails, log a warning but still connect
        // (token can be retried on send).
        if let Err(e) = self.acquire_oauth_token().await {
            tracing::warn!("Google Chat: failed to acquire OAuth token during connect: {e}");
        }

        // Register webhook handler for incoming Google Chat events
        if let Some(server) = &self.webhook_server {
            let tx = self.incoming_tx.clone();

            let handler: crate::webhook_server::WebhookHandler = std::sync::Arc::new(move |_method, _path, body, _headers| {
                let tx = tx.clone();
                Box::pin(async move {
                    let json: serde_json::Value = match serde_json::from_slice(&body) {
                        Ok(v) => v,
                        Err(_) => return (400, "Invalid JSON".to_string()),
                    };

                    if json["type"].as_str() != Some("MESSAGE") {
                        return (200, "OK".to_string());
                    }

                    let text = json["message"]["text"].as_str().unwrap_or("").to_string();
                    let sender = json["message"]["sender"]["name"].as_str().unwrap_or("unknown").to_string();
                    let sender_name = json["message"]["sender"]["displayName"].as_str().map(|s| s.to_string());
                    let space = json["message"]["space"]["name"].as_str().unwrap_or("").to_string();
                    let msg_name = json["message"]["name"].as_str().unwrap_or("").to_string();
                    let is_group = json["message"]["space"]["type"].as_str() == Some("ROOM");

                    let incoming = crate::IncomingMessage {
                        id: msg_name,
                        channel_id: "google-chat".to_string(),
                        channel_type: "google-chat".to_string(),
                        instance_id: "default".to_string(),
                        sender,
                        sender_name,
                        text,
                        is_group,
                        group_id: if is_group { Some(space) } else { None },
                        thread_id: None,
                        timestamp: chrono::Utc::now(),
                        media_url: None,
                        source_trust_level: crate::SourceTrustLevel::Authenticated,
                    };

                    let _ = tx.send(incoming).await;
                    (200, "OK".to_string())
                })
            });

            server.register_handler("google-chat", handler).await;
        }

        set_status(&self.status, ConnectionStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(server) = &self.webhook_server {
            server.unregister_handler("google-chat").await;
        }
        *self.oauth_token.lock().await = None;
        *self.token_expiry.lock().await = None;
        set_status(&self.status, ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "service_account" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'service_account'.",
                credentials.credential_type
            )));
        }

        let sa_json_str = credentials
            .data
            .get("service_account_json")
            .ok_or_else(|| {
                ChannelError::AuthFailed(
                    "Missing 'service_account_json' in credentials data".into(),
                )
            })?
            .clone();

        let sa_key: serde_json::Value = serde_json::from_str(&sa_json_str).map_err(|e| {
            ChannelError::AuthFailed(format!("Invalid service account JSON: {e}"))
        })?;

        // Validate required fields exist
        if sa_key.get("client_email").is_none() {
            return Err(ChannelError::AuthFailed(
                "Service account JSON missing 'client_email'".into(),
            ));
        }
        if sa_key.get("private_key").is_none() {
            return Err(ChannelError::AuthFailed(
                "Service account JSON missing 'private_key'".into(),
            ));
        }

        *self.service_account_key.lock().await = Some(sa_key);

        // In production, we would call acquire_oauth_token() here.
        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.service_account_key.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Google Chat not connected".into()));
        }

        // Check if token is expired or missing and refresh
        let needs_refresh = {
            let expiry = self.token_expiry.lock().await;
            match expiry.as_ref() {
                Some(exp) => chrono::Utc::now() >= *exp - chrono::Duration::minutes(5),
                None => self.oauth_token.lock().await.is_none(),
            }
        };
        if needs_refresh {
            self.acquire_oauth_token().await?;
        }

        let token = self.oauth_token.lock().await.clone()
            .ok_or_else(|| ChannelError::SendFailed("No OAuth token available".into()))?;

        // recipient should be a space name like "spaces/AAAA"
        let url = format!("https://chat.googleapis.com/v1/{}/messages", recipient);

        // Google Chat messages have a 4096 character limit.
        let chunks = chunk_message(&message.text, 4096);

        for chunk in chunks {
            let body = serde_json::json!({
                "text": chunk,
            });

            let resp = self
                .client
                .post(&url)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Google Chat send failed: {e}")))?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed(format!(
                    "Google Chat API error: {body}"
                )));
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct GoogleChatChannelFactory;

impl crate::ChannelPluginFactory for GoogleChatChannelFactory {
    fn channel_type(&self) -> &str { "google-chat" }
    fn channel_type_name(&self) -> &str { "Google Chat" }
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
        Box::new(GoogleChatChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_google_chat_metadata() {
        let channel = GoogleChatChannel::new();
        assert_eq!(channel.id(), "google-chat");
        assert_eq!(channel.name(), "Google Chat");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_google_chat_features() {
        let channel = GoogleChatChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(!features.media_attachments);
        assert!(!features.reactions);
        assert!(!features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_google_chat_login_bad_type() {
        let mut channel = GoogleChatChannel::new();
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_google_chat_send_not_connected() {
        let channel = GoogleChatChannel::new();
        let msg = OutgoingMessage {
            text: "Hello Google Chat".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("spaces/AAAA", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
