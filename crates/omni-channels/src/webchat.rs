//! WebChat Channel Plugin
//!
//! Built-in web chat interface served over HTTP on the shared webhook server.
//! Allows users to interact with Omni directly from a browser.
//!
//! ## Authentication
//! - Requires `login()` with `credential_type: "api_key"` before `connect()`.
//! - If no API key is configured, `connect()` auto-generates a random key.
//!   Retrieve it via `get_api_key()` or the UI. This prevents unauthenticated
//!   access by default (defense against OpenClaw-style cross-origin attacks).
//!
//! ## Connection
//! Registers an HTTP handler at `/webchat` on the shared webhook server.
//! Messages are exchanged as JSON:
//! - POST Incoming: `{"text": "...", "sender": "...", "session_id": "...", "nonce": "..."}`
//! - GET Status: returns session count and status
//!
//! ## CSRF Protection
//! Each session is assigned a random nonce on creation. Subsequent requests for
//! that session must include the correct nonce, preventing cross-origin replay.
//!
//! ## Features
//! - Direct messages (one user per HTTP session)
//! - Typing indicators (client sends typing events)

use std::collections::HashMap;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Session TTL -- sessions inactive for longer than this are cleaned up.
const SESSION_TTL: Duration = Duration::from_secs(30 * 60); // 30 minutes

/// How often to run the session cleanup sweep.
const SESSION_CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

/// Per-session state including the CSRF nonce.
struct SessionEntry {
    /// Sender for outgoing messages.
    tx: mpsc::Sender<String>,
    /// CSRF nonce -- must be provided in subsequent requests.
    nonce: String,
    /// Last time this session was active (request received).
    last_activity: Instant,
}

/// WebChat channel plugin using the shared webhook server.
pub struct WebChatChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// API key for authenticating clients. Required before connect().
    api_key: Mutex<Option<String>>,
    /// Active sessions: session_id -> SessionEntry (sender + nonce).
    sessions: Arc<RwLock<HashMap<String, SessionEntry>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shared webhook server for registering the handler.
    webhook_server: Option<Arc<crate::webhook_server::WebhookServer>>,
    /// Handle for the session cleanup background task.
    cleanup_handle: Option<JoinHandle<()>>,
}

impl WebChatChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            api_key: Mutex::new(None),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            webhook_server: None,
            cleanup_handle: None,
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for WebChatChannel {
    fn id(&self) -> &str {
        "webchat"
    }

    fn name(&self) -> &str {
        "WebChat"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: false,
            media_attachments: false,
            reactions: false,
            read_receipts: false,
            typing_indicators: true,
            threads: false,
        }
    }

    fn status(&self) -> ConnectionStatus {
        get_status(&self.status)
    }

    fn set_webhook_server(&mut self, server: Arc<crate::webhook_server::WebhookServer>) {
        self.webhook_server = Some(server);
    }

    fn get_api_key(&self) -> Option<String> {
        self.api_key.try_lock().ok()?.clone()
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        // Enforce authentication: API key must be set before connecting.
        // If no key was set via login(), auto-generate one.
        {
            let mut key_guard = self.api_key.lock().await;
            if key_guard.is_none() {
                let generated_key = uuid::Uuid::new_v4().to_string();
                tracing::info!(
                    "WebChat: No API key configured -- auto-generated one. \
                     Retrieve it via the UI or the channel_get_api_key command."
                );
                *key_guard = Some(generated_key);
            }
        }

        if let Some(server) = &self.webhook_server {
            let tx = self.incoming_tx.clone();
            let sessions = self.sessions.clone();
            let api_key = self.api_key.lock().await.clone();

            let handler: crate::webhook_server::WebhookHandler = Arc::new(move |method, _path, body, headers| {
                let tx = tx.clone();
                let sessions = sessions.clone();
                let api_key = api_key.clone();
                Box::pin(async move {
                    // API key is always required (enforced by connect())
                    if let Some(ref key) = api_key {
                        let provided = headers.get("x-api-key")
                            .or_else(|| headers.get("authorization"))
                            .cloned()
                            .unwrap_or_default();
                        // Support "Bearer <key>" or raw key
                        let provided = provided.strip_prefix("Bearer ").unwrap_or(&provided);
                        if provided != key {
                            return (401, "Unauthorized: valid API key required".to_string());
                        }
                    }

                    if method == "POST" {
                        // Incoming message from browser client
                        let json: serde_json::Value = match serde_json::from_slice(&body) {
                            Ok(v) => v,
                            Err(_) => return (400, "Invalid JSON".to_string()),
                        };

                        let text = json.get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        if text.is_empty() {
                            return (200, "OK".to_string());
                        }

                        let sender = json.get("sender")
                            .and_then(|v| v.as_str())
                            .unwrap_or("anonymous")
                            .to_string();

                        let session_id = json.get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let provided_nonce = json.get("nonce")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let session_id = if session_id.is_empty() {
                            uuid::Uuid::new_v4().to_string()
                        } else {
                            session_id
                        };

                        // Session + CSRF nonce management
                        // Use a single write lock to avoid TOCTOU races.
                        let nonce_for_response;
                        {
                            let mut sessions_guard = sessions.write().await;
                            if let Some(entry) = sessions_guard.get_mut(&session_id) {
                                // Existing session -- verify CSRF nonce
                                if provided_nonce != entry.nonce {
                                    return (403, "Invalid or missing session nonce".to_string());
                                }
                                entry.last_activity = Instant::now();
                                nonce_for_response = entry.nonce.clone();
                            } else {
                                // New session -- generate CSRF nonce
                                let nonce = uuid::Uuid::new_v4().to_string();
                                let (session_tx, _session_rx) = mpsc::channel::<String>(64);
                                nonce_for_response = nonce.clone();
                                sessions_guard.insert(session_id.clone(), SessionEntry {
                                    tx: session_tx,
                                    nonce,
                                    last_activity: Instant::now(),
                                });
                            }
                        }

                        let incoming = IncomingMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            channel_id: "webchat".to_string(),
                            channel_type: "webchat".to_string(),
                            instance_id: "default".to_string(),
                            sender: sender.clone(),
                            sender_name: Some(sender),
                            text,
                            is_group: false,
                            group_id: None,
                            thread_id: None,
                            timestamp: chrono::Utc::now(),
                            media_url: None,
                            source_trust_level: crate::SourceTrustLevel::Authenticated,
                        };

                        let _ = tx.send(incoming).await;

                        let resp = serde_json::json!({
                            "session_id": session_id,
                            "nonce": nonce_for_response,
                        });
                        (200, resp.to_string())
                    } else {
                        // GET -- return status info
                        let session_count = sessions.read().await.len();
                        let resp = serde_json::json!({
                            "status": "connected",
                            "sessions": session_count,
                        });
                        (200, resp.to_string())
                    }
                })
            });

            server.register_handler("webchat", handler).await;
        }

        // Abort any previous cleanup task (guards against double-connect without disconnect)
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }

        // Spawn background task to clean up stale sessions
        let sessions_cleanup = self.sessions.clone();
        self.cleanup_handle = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(SESSION_CLEANUP_INTERVAL);
            loop {
                interval.tick().await;
                let mut guard = sessions_cleanup.write().await;
                let before = guard.len();
                guard.retain(|_, entry| entry.last_activity.elapsed() < SESSION_TTL);
                let removed = before - guard.len();
                if removed > 0 {
                    tracing::debug!(removed, "Cleaned up stale WebChat sessions");
                }
            }
        }));

        set_status(&self.status, ConnectionStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
        if let Some(server) = &self.webhook_server {
            server.unregister_handler("webchat").await;
        }
        self.sessions.write().await.clear();
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

        let api_key = credentials
            .data
            .get("api_key")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'api_key' in credentials data".into())
            })?
            .clone();

        if api_key.is_empty() {
            return Err(ChannelError::AuthFailed(
                "API key cannot be empty".into(),
            ));
        }

        *self.api_key.lock().await = Some(api_key);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.api_key.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected(
                "WebChat not connected".into(),
            ));
        }

        let sessions = self.sessions.read().await;
        let entry = sessions.get(recipient)
            .ok_or_else(|| ChannelError::SendFailed(format!(
                "No WebChat session found for '{recipient}'"
            )))?;

        let chunks = chunk_message(&message.text, 8000);

        for chunk in chunks {
            let frame = serde_json::json!({
                "text": chunk,
                "sender": "omni",
            });
            let _ = entry.tx.send(frame.to_string()).await;
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct WebChatChannelFactory;

impl crate::ChannelPluginFactory for WebChatChannelFactory {
    fn channel_type(&self) -> &str { "webchat" }
    fn channel_type_name(&self) -> &str { "WebChat" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: false,
            media_attachments: false,
            reactions: false,
            read_receipts: false,
            typing_indicators: true,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(WebChatChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webchat_metadata() {
        let channel = WebChatChannel::new();
        assert_eq!(channel.id(), "webchat");
        assert_eq!(channel.name(), "WebChat");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_webchat_features() {
        let channel = WebChatChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(!features.group_messages);
        assert!(!features.media_attachments);
        assert!(!features.reactions);
        assert!(!features.read_receipts);
        assert!(features.typing_indicators);
    }

    #[tokio::test]
    async fn test_webchat_login_with_key() {
        let mut channel = WebChatChannel::new();
        let mut data = std::collections::HashMap::new();
        data.insert("api_key".to_string(), "my-secret-key-123".to_string());
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_ok());
        match result.unwrap() {
            LoginStatus::Success => {}
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_webchat_login_bad_type() {
        let mut channel = WebChatChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: std::collections::HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_webchat_send_not_connected() {
        let channel = WebChatChannel::new();
        let msg = OutgoingMessage {
            text: "Hello from WebChat".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("session-abc-123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
