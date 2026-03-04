//! Nostr Channel Plugin
//!
//! Connects to Nostr relays via WebSocket and signs events with Schnorr (NIP-01).
//!
//! ## Authentication
//! - `credential_type`: "private_key"
//! - `data.private_key`: hex-encoded 32-byte private key
//! - `data.relays`: comma-separated list of relay WebSocket URLs
//!   (e.g., "wss://relay.damus.io,wss://nos.lol")
//!
//! ## Protocol
//! - Events signed with Schnorr (secp256k1) per NIP-01
//! - Subscribes to kind-4 DMs addressed to our pubkey
//! - Sends kind-1 text notes with p-tag (public mentions)
//! - Full NIP-04 encrypted DMs would need AES-256-CBC (noted for future)
//!
//! ## Features
//! - Direct messages (via kind-4 or tagged kind-1)
//! - Reactions (NIP-25)

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use futures::stream::StreamExt;
use futures::SinkExt;
use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Derive the public key (x-only, hex) from a hex-encoded private key.
fn derive_pubkey(private_key_hex: &str) -> std::result::Result<String, String> {
    use k256::schnorr::SigningKey;

    let sk_bytes = hex_decode(private_key_hex)
        .map_err(|e| format!("Invalid private key hex: {e}"))?;

    let signing_key = SigningKey::from_bytes(&sk_bytes)
        .map_err(|e| format!("Invalid private key: {e}"))?;

    let verifying_key = signing_key.verifying_key();
    let pk_bytes = verifying_key.to_bytes();
    Ok(hex_encode(&pk_bytes))
}

/// Create and sign a Nostr event (NIP-01).
fn create_signed_event(
    private_key_hex: &str,
    pubkey_hex: &str,
    kind: u64,
    content: &str,
    tags: Vec<Vec<String>>,
) -> std::result::Result<serde_json::Value, String> {
    use k256::schnorr::SigningKey;
    use sha2::{Sha256, Digest};

    let created_at = chrono::Utc::now().timestamp();

    // Build the serialized event for hashing (NIP-01)
    let serialized = serde_json::json!([
        0,
        pubkey_hex,
        created_at,
        kind,
        tags,
        content,
    ]);

    // SHA-256 hash of the serialized event
    let mut hasher = Sha256::new();
    hasher.update(serialized.to_string().as_bytes());
    let event_hash = hasher.finalize();
    let event_id = hex_encode(&event_hash);

    // Sign the event hash with Schnorr
    let sk_bytes = hex_decode(private_key_hex)
        .map_err(|e| format!("Invalid private key hex: {e}"))?;
    let signing_key = SigningKey::from_bytes(&sk_bytes)
        .map_err(|e| format!("Invalid signing key: {e}"))?;

    let signature = signing_key.sign_raw(&event_hash, &Default::default())
        .map_err(|e| format!("Signing failed: {e}"))?;

    let sig_hex = hex_encode(&signature.to_bytes());

    Ok(serde_json::json!({
        "id": event_id,
        "pubkey": pubkey_hex,
        "created_at": created_at,
        "kind": kind,
        "tags": tags,
        "content": content,
        "sig": sig_hex,
    }))
}

fn hex_decode(s: &str) -> std::result::Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("odd length hex string".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Nostr channel plugin with real WebSocket relay connections and Schnorr signing.
pub struct NostrChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// Hex private key.
    private_key: Mutex<Option<String>>,
    /// Derived hex public key (x-only).
    public_key: Mutex<Option<String>>,
    /// Relay WebSocket URLs.
    relays: Mutex<Vec<String>>,
    /// Sender for outgoing event JSON to broadcast to relays.
    outgoing_tx: Mutex<Option<mpsc::Sender<String>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the relay listener task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl NostrChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            private_key: Mutex::new(None),
            public_key: Mutex::new(None),
            relays: Mutex::new(Vec::new()),
            outgoing_tx: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for NostrChannel {
    fn id(&self) -> &str {
        "nostr"
    }

    fn name(&self) -> &str {
        "Nostr"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: false,
            media_attachments: false,
            reactions: true,
            read_receipts: false,
            typing_indicators: false,
            threads: false,
        }
    }

    fn status(&self) -> ConnectionStatus {
        get_status(&self.status)
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        if self.private_key.lock().await.is_none() {
            return Err(ChannelError::Config("Private key not set. Call login() first.".into()));
        }
        let pubkey = self.public_key.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Public key not derived. Call login() first.".into()))?;
        let relays = self.relays.lock().await.clone();

        if relays.is_empty() {
            return Err(ChannelError::Config("No relays configured.".into()));
        }

        set_status(&self.status, ConnectionStatus::Connecting);

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        // Create outgoing broadcast channel
        let (outgoing_tx, _) = mpsc::channel::<String>(128);
        *self.outgoing_tx.lock().await = Some(outgoing_tx.clone());

        let tx = self.incoming_tx.clone();

        // We'll use a shared shutdown signal via a broadcast
        let shutdown = Arc::new(tokio::sync::Notify::new());
        let shutdown_trigger = shutdown.clone();

        // Spawn a task that listens for the oneshot and notifies all relay tasks
        tokio::spawn(async move {
            let _ = shutdown_rx.await;
            shutdown_trigger.notify_waiters();
        });

        // Spawn a task per relay
        for relay_url in relays {
            let tx = tx.clone();
            let pubkey = pubkey.clone();
            let shutdown = shutdown.clone();

            tokio::spawn(async move {
                let ws_result = tokio_tungstenite::connect_async(&relay_url).await;
                let (ws_stream, _) = match ws_result {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("Nostr: failed to connect to relay {relay_url}: {e}");
                        return;
                    }
                };

                tracing::info!("Nostr: connected to relay {relay_url}");

                let (mut ws_writer, mut ws_reader) = ws_stream.split();

                // Subscribe to kind-4 DMs addressed to our pubkey and kind-1 mentions
                let sub_filter = serde_json::json!(["REQ", "omni-sub", {
                    "kinds": [1, 4],
                    "#p": [&pubkey],
                    "since": chrono::Utc::now().timestamp(),
                }]);

                if let Err(e) = ws_writer.send(
                    tokio_tungstenite::tungstenite::Message::Text(sub_filter.to_string().into())
                ).await {
                    tracing::warn!("Nostr: failed to send SUB to {relay_url}: {e}");
                    return;
                }

                loop {
                    tokio::select! {
                        _ = shutdown.notified() => {
                            // Send CLOSE
                            let close_msg = serde_json::json!(["CLOSE", "omni-sub"]);
                            let _ = ws_writer.send(
                                tokio_tungstenite::tungstenite::Message::Text(close_msg.to_string().into())
                            ).await;
                            break;
                        }
                        frame = ws_reader.next() => {
                            let frame = match frame {
                                Some(Ok(f)) => f,
                                Some(Err(e)) => {
                                    tracing::debug!("Nostr relay {relay_url} read error: {e}");
                                    break;
                                }
                                None => break,
                            };

                            let text = match frame {
                                tokio_tungstenite::tungstenite::Message::Text(t) => t.to_string(),
                                tokio_tungstenite::tungstenite::Message::Ping(data) => {
                                    let _ = ws_writer.send(
                                        tokio_tungstenite::tungstenite::Message::Pong(data)
                                    ).await;
                                    continue;
                                }
                                tokio_tungstenite::tungstenite::Message::Close(_) => break,
                                _ => continue,
                            };

                            // Parse relay message: ["EVENT", "sub_id", {event}]
                            let msg: serde_json::Value = match serde_json::from_str(&text) {
                                Ok(v) => v,
                                Err(_) => continue,
                            };

                            let arr = match msg.as_array() {
                                Some(a) => a,
                                None => continue,
                            };

                            if arr.len() < 3 {
                                continue;
                            }

                            let msg_type = arr[0].as_str().unwrap_or("");
                            if msg_type != "EVENT" {
                                continue;
                            }

                            let event = &arr[2];

                            let event_pubkey = event.get("pubkey")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            // Skip our own events
                            if event_pubkey == pubkey {
                                continue;
                            }

                            let content = event.get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            if content.is_empty() {
                                continue;
                            }

                            let event_id = event.get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let created_at = event.get("created_at")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            let timestamp = chrono::DateTime::from_timestamp(created_at, 0)
                                .unwrap_or_else(chrono::Utc::now);

                            let incoming = IncomingMessage {
                                id: if event_id.is_empty() { uuid::Uuid::new_v4().to_string() } else { event_id },
                                channel_id: "nostr".to_string(),
                                channel_type: "nostr".to_string(),
                                instance_id: "default".to_string(),
                                sender: event_pubkey.to_string(),
                                sender_name: None,
                                text: content,
                                is_group: false,
                                group_id: None,
                                thread_id: None,
                                timestamp,
                                media_url: None,
                                source_trust_level: crate::SourceTrustLevel::Authenticated,
                            };

                            let _ = tx.send(incoming).await;
                        }
                    }
                }

                tracing::info!("Nostr: disconnected from relay {relay_url}");
            });
        }

        set_status(&self.status, ConnectionStatus::Connected);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        *self.outgoing_tx.lock().await = None;
        set_status(&self.status, ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "private_key" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'private_key'.",
                credentials.credential_type
            )));
        }

        let private_key = credentials
            .data
            .get("private_key")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'private_key' in credentials data".into())
            })?
            .clone();

        if private_key.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Private key cannot be empty".into(),
            ));
        }

        // Derive public key
        let pubkey = derive_pubkey(&private_key)
            .map_err(|e| ChannelError::AuthFailed(format!("Failed to derive public key: {e}")))?;

        tracing::info!("Nostr: derived public key {pubkey}");

        // Parse comma-separated relay list
        let relay_str = credentials
            .data
            .get("relays")
            .cloned()
            .unwrap_or_default();
        let relays: Vec<String> = relay_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        *self.private_key.lock().await = Some(private_key);
        *self.public_key.lock().await = Some(pubkey);
        *self.relays.lock().await = relays;

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.private_key.lock().await = None;
        *self.public_key.lock().await = None;
        *self.relays.lock().await = Vec::new();
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Nostr not connected".into()));
        }

        let private_key = self.private_key.lock().await.clone()
            .ok_or_else(|| ChannelError::SendFailed("Private key not available".into()))?;
        let pubkey = self.public_key.lock().await.clone()
            .ok_or_else(|| ChannelError::SendFailed("Public key not available".into()))?;
        let relays = self.relays.lock().await.clone();

        let chunks = chunk_message(&message.text, 8000);

        for chunk in chunks {
            // Create a kind-1 text note with p-tag mentioning the recipient
            let tags = vec![vec!["p".to_string(), recipient.to_string()]];

            let event = create_signed_event(&private_key, &pubkey, 1, &chunk, tags)
                .map_err(|e| ChannelError::SendFailed(format!("Failed to create Nostr event: {e}")))?;

            let event_msg = serde_json::json!(["EVENT", event]);
            let event_str = event_msg.to_string();

            // Broadcast to all relays
            for relay_url in &relays {
                let ws_result = tokio_tungstenite::connect_async(relay_url).await;
                match ws_result {
                    Ok((mut ws, _)) => {
                        use tokio_tungstenite::tungstenite::Message;
                        if let Err(e) = ws.send(Message::Text(event_str.clone().into())).await {
                            tracing::warn!("Nostr: failed to send event to {relay_url}: {e}");
                        } else {
                            tracing::debug!("Nostr: event sent to {relay_url}");
                        }
                        let _ = ws.close(None).await;
                    }
                    Err(e) => {
                        tracing::warn!("Nostr: failed to connect to {relay_url} for send: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct NostrChannelFactory;

impl crate::ChannelPluginFactory for NostrChannelFactory {
    fn channel_type(&self) -> &str { "nostr" }
    fn channel_type_name(&self) -> &str { "Nostr" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: false,
            media_attachments: false,
            reactions: true,
            read_receipts: false,
            typing_indicators: false,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(NostrChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_nostr_metadata() {
        let channel = NostrChannel::new();
        assert_eq!(channel.id(), "nostr");
        assert_eq!(channel.name(), "Nostr");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_nostr_features() {
        let channel = NostrChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(!features.group_messages);
        assert!(!features.media_attachments);
        assert!(features.reactions);
        assert!(!features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_nostr_login_bad_type() {
        let mut channel = NostrChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_nostr_login_missing_key() {
        let mut channel = NostrChannel::new();
        let creds = ChannelCredentials {
            credential_type: "private_key".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing"));
    }

    #[tokio::test]
    async fn test_nostr_send_not_connected() {
        let channel = NostrChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("npub1abc123", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }

    #[test]
    fn test_hex_roundtrip() {
        let original = vec![0xde, 0xad, 0xbe, 0xef];
        let encoded = hex_encode(&original);
        assert_eq!(encoded, "deadbeef");
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_derive_pubkey() {
        // Known test vector: a valid 32-byte hex private key
        // Using a well-known test key (all 1s, which is a valid scalar)
        let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
        let result = derive_pubkey(private_key);
        assert!(result.is_ok());
        let pubkey = result.unwrap();
        // x-only pubkey should be 64 hex chars (32 bytes)
        assert_eq!(pubkey.len(), 64);
    }

    #[test]
    fn test_create_signed_event() {
        let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
        let pubkey = derive_pubkey(private_key).unwrap();
        let event = create_signed_event(
            private_key,
            &pubkey,
            1,
            "Hello Nostr!",
            vec![],
        );
        assert!(event.is_ok());
        let event = event.unwrap();
        assert_eq!(event["kind"], 1);
        assert_eq!(event["content"], "Hello Nostr!");
        assert!(event["sig"].as_str().unwrap().len() == 128); // 64 bytes = 128 hex chars
    }

    #[tokio::test]
    async fn test_nostr_login_with_valid_key() {
        let mut channel = NostrChannel::new();
        let mut data = HashMap::new();
        data.insert(
            "private_key".to_string(),
            "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
        );
        data.insert("relays".to_string(), "wss://relay.damus.io".to_string());
        let creds = ChannelCredentials {
            credential_type: "private_key".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_ok());
        assert!(channel.public_key.lock().await.is_some());
        assert_eq!(channel.relays.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn test_nostr_connect_without_login() {
        let mut channel = NostrChannel::new();
        let config = ChannelConfig {
            settings: HashMap::new(),
        };
        let result = channel.connect(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Private key not set"));
    }
}
