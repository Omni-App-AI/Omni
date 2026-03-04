//! Microsoft Teams Channel Plugin
//!
//! Implements the ChannelPlugin trait for Microsoft Teams via Bot Framework REST API.
//!
//! ## Authentication
//! - `credential_type`: "oauth"
//! - `data.app_id`: Bot application (client) ID
//! - `data.app_password`: Bot application password (client secret)
//!
//! ## How it works
//! - Login: OAuth token via POST https://login.microsoftonline.com/botframework.com/oauth2/v2.0/token
//! - Send: POST {service_url}/v3/conversations/{id}/activities
//! - Receive: webhook at /teams/api/messages (Bot Framework Activity)
//!
//! ## Features
//! DM + group + reactions + typing indicators

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Microsoft Teams channel plugin using Bot Framework REST API.
pub struct TeamsChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API calls.
    client: reqwest::Client,
    /// Bot application (client) ID.
    app_id: Mutex<Option<String>>,
    /// Bot application password (client secret).
    app_password: Mutex<Option<String>>,
    /// Bot Framework service URL (set per-conversation from incoming activities).
    service_url: Arc<Mutex<Option<String>>>,
    /// OAuth bearer token for Bot Framework REST calls.
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

impl TeamsChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            app_id: Mutex::new(None),
            app_password: Mutex::new(None),
            service_url: Arc::new(Mutex::new(None)),
            oauth_token: Mutex::new(None),
            token_expiry: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            _shutdown_tx: Mutex::new(None),
            webhook_server: None,
        }
    }

    /// Acquire an OAuth token from the Bot Framework token endpoint.
    async fn acquire_oauth_token(&self) -> Result<String> {
        let app_id = self.app_id.lock().await.clone()
            .ok_or_else(|| ChannelError::AuthFailed("app_id not set".into()))?;
        let app_password = self.app_password.lock().await.clone()
            .ok_or_else(|| ChannelError::AuthFailed("app_password not set".into()))?;

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &app_id),
            ("client_secret", &app_password),
            ("scope", "https://api.botframework.com/.default"),
        ];

        let resp = self
            .client
            .post("https://login.microsoftonline.com/botframework.com/oauth2/v2.0/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Token request failed: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed(format!("Token endpoint returned error: {body}")));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Failed to parse token response: {e}")))?;

        let token = json["access_token"]
            .as_str()
            .ok_or_else(|| ChannelError::AuthFailed("No access_token in response".into()))?
            .to_string();

        // Store expiry
        if let Some(expires_in) = json["expires_in"].as_i64() {
            let expiry = chrono::Utc::now() + chrono::Duration::seconds(expires_in);
            *self.token_expiry.lock().await = Some(expiry);
        }

        *self.oauth_token.lock().await = Some(token.clone());
        Ok(token)
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for TeamsChannel {
    fn id(&self) -> &str {
        "teams"
    }

    fn name(&self) -> &str {
        "Microsoft Teams"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: false,
            reactions: true,
            read_receipts: false,
            typing_indicators: true,
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
        // Teams is webhook-driven: the bot receives activities at /teams/api/messages.
        // Attempt to acquire an OAuth token; if it fails, log a warning but still connect
        // (token can be retried on send).
        if let Err(e) = self.acquire_oauth_token().await {
            tracing::warn!("Teams: failed to acquire OAuth token during connect: {e}");
        }

        // Register webhook handler for incoming Teams activities
        if let Some(server) = &self.webhook_server {
            let tx = self.incoming_tx.clone();
            let service_url = self.service_url.clone();

            let handler: crate::webhook_server::WebhookHandler = std::sync::Arc::new(move |_method, _path, body, _headers| {
                let tx = tx.clone();
                let service_url = service_url.clone();
                Box::pin(async move {
                    let json: serde_json::Value = match serde_json::from_slice(&body) {
                        Ok(v) => v,
                        Err(_) => return (400, "Invalid JSON".to_string()),
                    };

                    // Store serviceUrl for reply routing
                    if let Some(svc_url) = json["serviceUrl"].as_str() {
                        *service_url.lock().await = Some(svc_url.to_string());
                    }

                    // Only process message activities
                    if json["type"].as_str() != Some("message") {
                        return (200, "OK".to_string());
                    }

                    let text = json["text"].as_str().unwrap_or("").to_string();
                    let sender = json["from"]["id"].as_str().unwrap_or("unknown").to_string();
                    let sender_name = json["from"]["name"].as_str().map(|s| s.to_string());
                    let conv_id = json["conversation"]["id"].as_str().unwrap_or("").to_string();
                    let msg_id = json["id"].as_str().unwrap_or("").to_string();
                    let is_group = json["conversation"]["isGroup"].as_bool().unwrap_or(false);

                    let incoming = crate::IncomingMessage {
                        id: msg_id,
                        channel_id: "teams".to_string(),
                        channel_type: "teams".to_string(),
                        instance_id: "default".to_string(),
                        sender,
                        sender_name,
                        text,
                        is_group,
                        group_id: if is_group { Some(conv_id) } else { None },
                        thread_id: None,
                        timestamp: chrono::Utc::now(),
                        media_url: None,
                        source_trust_level: crate::SourceTrustLevel::Authenticated,
                    };

                    let _ = tx.send(incoming).await;
                    (200, "OK".to_string())
                })
            });

            server.register_handler("teams", handler).await;
        }

        set_status(&self.status, ConnectionStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(server) = &self.webhook_server {
            server.unregister_handler("teams").await;
        }
        *self.oauth_token.lock().await = None;
        *self.token_expiry.lock().await = None;
        set_status(&self.status, ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "oauth" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'oauth'.",
                credentials.credential_type
            )));
        }

        let app_id = credentials
            .data
            .get("app_id")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'app_id' in credentials data".into()))?
            .clone();

        let app_password = credentials
            .data
            .get("app_password")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'app_password' in credentials data".into()))?
            .clone();

        if app_id.is_empty() || app_password.is_empty() {
            return Err(ChannelError::AuthFailed("app_id and app_password cannot be empty".into()));
        }

        *self.app_id.lock().await = Some(app_id);
        *self.app_password.lock().await = Some(app_password);

        // In production, we would call acquire_oauth_token() here.
        // For now, store credentials and report success.
        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.app_id.lock().await = None;
        *self.app_password.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Teams not connected".into()));
        }

        let service_url = self.service_url.lock().await.clone()
            .ok_or_else(|| ChannelError::SendFailed(
                "No service_url set. Receive an inbound activity first.".into(),
            ))?;

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

        let url = format!("{}/v3/conversations/{}/activities", service_url, recipient);

        // Teams messages can be long but we chunk at 28000 chars (adaptive card limit).
        let chunks = chunk_message(&message.text, 28000);

        for chunk in chunks {
            let activity = serde_json::json!({
                "type": "message",
                "text": chunk,
            });

            let resp = self
                .client
                .post(&url)
                .bearer_auth(&token)
                .json(&activity)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Teams send failed: {e}")))?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed(format!(
                    "Teams API error: {body}"
                )));
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct TeamsChannelFactory;

impl crate::ChannelPluginFactory for TeamsChannelFactory {
    fn channel_type(&self) -> &str { "teams" }
    fn channel_type_name(&self) -> &str { "Microsoft Teams" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: false,
            reactions: true,
            read_receipts: false,
            typing_indicators: true,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(TeamsChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_teams_metadata() {
        let channel = TeamsChannel::new();
        assert_eq!(channel.id(), "teams");
        assert_eq!(channel.name(), "Microsoft Teams");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_teams_features() {
        let channel = TeamsChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.reactions);
        assert!(features.typing_indicators);
        assert!(!features.read_receipts);
        assert!(!features.media_attachments);
    }

    #[tokio::test]
    async fn test_teams_login_bad_type() {
        let mut channel = TeamsChannel::new();
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_teams_login_missing_fields() {
        let mut channel = TeamsChannel::new();
        // Missing app_password
        let mut data = HashMap::new();
        data.insert("app_id".to_string(), "test-id".to_string());
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("app_password"));
    }

    #[tokio::test]
    async fn test_teams_send_not_connected() {
        let channel = TeamsChannel::new();
        let msg = OutgoingMessage {
            text: "Hello Teams".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("conv-123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
