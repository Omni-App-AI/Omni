//! Tlon/Urbit Channel Plugin
//!
//! Integrates with Urbit ships via the Eyre HTTP API.
//!
//! ## Authentication
//! - `credential_type`: "access_code"
//! - `data.ship_url`: URL of the Urbit ship (e.g., "http://localhost:8080")
//! - `data.ship_name`: ship name / @p (e.g., "~zod")
//! - `data.access_code`: the ship's +code for login
//!
//! ## Login
//! `POST {ship_url}/~/login` with body `password={access_code}`
//! Returns a session cookie (`urbauth-{ship}=...`).
//!
//! ## Sending Messages
//! `PUT {ship_url}/~/channel/{channel_id}` with a poke action JSON body.
//!
//! ## Receiving Messages
//! SSE stream from `GET {ship_url}/~/channel/{channel_id}`
//!
//! ## Features
//! - Direct messages
//! - Group messages

use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Tlon/Urbit channel plugin using the Eyre HTTP API.
pub struct UrbitChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API requests (cookie store disabled -- we manage cookies manually).
    client: reqwest::Client,
    /// Ship URL (e.g., "http://localhost:8080").
    ship_url: Mutex<Option<String>>,
    /// Ship name / @p (e.g., "~zod").
    ship_name: Mutex<Option<String>>,
    /// Ship +code for authentication.
    access_code: Mutex<Option<String>>,
    /// Session cookie from login (urbauth-{ship}=...).
    cookie: Arc<Mutex<Option<String>>>,
    /// Eyre channel ID for pokes and SSE subscription.
    channel_id: Arc<Mutex<Option<String>>>,
    /// Monotonically increasing event-id for Eyre channel actions.
    event_id_counter: Arc<AtomicU64>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the SSE listener task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl UrbitChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            ship_url: Mutex::new(None),
            ship_name: Mutex::new(None),
            access_code: Mutex::new(None),
            cookie: Arc::new(Mutex::new(None)),
            channel_id: Arc::new(Mutex::new(None)),
            event_id_counter: Arc::new(AtomicU64::new(1)),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }

    fn next_event_id(&self) -> u64 {
        self.event_id_counter.fetch_add(1, Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for UrbitChannel {
    fn id(&self) -> &str {
        "urbit"
    }

    fn name(&self) -> &str {
        "Tlon/Urbit"
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

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        let ship_url = self.ship_url.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Ship URL not set. Call login() first.".into()))?;
        let ship_name = self.ship_name.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Ship name not set. Call login() first.".into()))?;
        let access_code = self.access_code.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Access code not set. Call login() first.".into()))?;

        set_status(&self.status, ConnectionStatus::Connecting);

        // Step 1: Authenticate via POST /~/login to get session cookie
        let login_url = format!("{}/~/login", ship_url.trim_end_matches('/'));
        let resp = self.client
            .post(&login_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!("password={}", access_code))
            .send()
            .await
            .map_err(|e| {
                set_status(&self.status, ConnectionStatus::Error);
                ChannelError::AuthFailed(format!("Urbit login request failed: {e}"))
            })?;

        if !resp.status().is_success() {
            set_status(&self.status, ConnectionStatus::Error);
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed(format!("Urbit login failed (HTTP): {body}")));
        }

        // Extract session cookie from set-cookie header
        let cookie_value = resp.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|v| v.contains("urbauth-"))
            .map(|v| {
                // Extract just the cookie key=value part (before any ';')
                v.split(';').next().unwrap_or(v).to_string()
            })
            .ok_or_else(|| {
                set_status(&self.status, ConnectionStatus::Error);
                ChannelError::AuthFailed("No urbauth cookie in login response".into())
            })?;

        *self.cookie.lock().await = Some(cookie_value.clone());
        tracing::info!("Urbit: authenticated with ship {}", ship_name);

        // Step 2: Generate a channel ID for this session
        let ch_id = format!("omni-{}", uuid::Uuid::new_v4());
        *self.channel_id.lock().await = Some(ch_id.clone());

        // Step 3: Open the Eyre channel with a subscribe action
        let channel_url = format!(
            "{}/~/channel/{}",
            ship_url.trim_end_matches('/'),
            ch_id
        );
        let ship_bare = ship_name.trim_start_matches('~');
        let sub_id = self.next_event_id();
        let subscribe_body = serde_json::json!([{
            "id": sub_id,
            "action": "subscribe",
            "ship": ship_bare,
            "app": "chat-store",
            "path": "/mailbox"
        }]);

        let sub_resp = self.client
            .put(&channel_url)
            .header("Cookie", &cookie_value)
            .header("Content-Type", "application/json")
            .json(&subscribe_body)
            .send()
            .await;

        if let Err(e) = &sub_resp {
            tracing::warn!("Urbit: failed to subscribe to chat-store (may not be installed): {e}");
        }

        // Step 4: Spawn SSE listener task for incoming messages
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.incoming_tx.clone();
        let status = self.status.clone();
        let cookie_arc = self.cookie.clone();
        let channel_id_arc = self.channel_id.clone();
        let event_counter = self.event_id_counter.clone();
        let client = self.client.clone();
        let sse_url = channel_url.clone();

        tokio::spawn(async move {
            use reqwest_eventsource::{EventSource, Event};
            use futures::StreamExt;

            let cookie_val = match cookie_arc.lock().await.clone() {
                Some(c) => c,
                None => return,
            };

            let request_builder = client.get(&sse_url)
                .header("Cookie", &cookie_val)
                .header("Accept", "text/event-stream");

            let mut es = EventSource::new(request_builder).expect("EventSource creation");

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Urbit SSE: shutting down");
                        break;
                    }
                    event = es.next() => {
                        match event {
                            Some(Ok(Event::Message(msg))) => {
                                // Parse SSE data as JSON
                                let json: serde_json::Value = match serde_json::from_str(&msg.data) {
                                    Ok(v) => v,
                                    Err(_) => continue,
                                };

                                // ACK the event
                                if let Some(event_id) = json.get("id").and_then(|v| v.as_u64()) {
                                    let ack_id = event_counter.fetch_add(1, Ordering::Relaxed);
                                    let ch_id = channel_id_arc.lock().await.clone().unwrap_or_default();
                                    let ack_url = format!(
                                        "{}/~/channel/{}",
                                        sse_url.rsplit_once("/~/channel/").map(|(base, _)| base).unwrap_or(""),
                                        ch_id
                                    );
                                    let ack_body = serde_json::json!([{
                                        "id": ack_id,
                                        "action": "ack",
                                        "event-id": event_id
                                    }]);
                                    let _ = client.put(&ack_url)
                                        .header("Cookie", &cookie_val)
                                        .json(&ack_body)
                                        .send()
                                        .await;
                                }

                                // Extract message content from the event
                                // Urbit chat events come in various formats depending on the app
                                if let Some(json_data) = json.get("json") {
                                    let text = json_data.get("message")
                                        .or_else(|| json_data.get("text"))
                                        .or_else(|| json_data.get("letter").and_then(|l| l.get("text")))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    if text.is_empty() {
                                        continue;
                                    }

                                    let sender = json_data.get("author")
                                        .or_else(|| json_data.get("ship"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string();

                                    let msg_id = json.get("id")
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                                    let incoming = IncomingMessage {
                                        id: msg_id,
                                        channel_id: "urbit".to_string(),
                                        channel_type: "urbit".to_string(),
                                        instance_id: "default".to_string(),
                                        sender,
                                        sender_name: None,
                                        text,
                                        is_group: false,
                                        group_id: None,
                                        thread_id: None,
                                        timestamp: chrono::Utc::now(),
                                        media_url: None,
                                        source_trust_level: crate::SourceTrustLevel::Authenticated,
                                    };

                                    let _ = tx.send(incoming).await;
                                }
                            }
                            Some(Ok(Event::Open)) => {
                                tracing::debug!("Urbit SSE: connection opened");
                            }
                            Some(Err(e)) => {
                                tracing::warn!("Urbit SSE error: {e}");
                                // Don't break on transient errors
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            }
                            None => {
                                tracing::info!("Urbit SSE: stream ended");
                                break;
                            }
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

        // Delete the Eyre channel on disconnect
        if let (Some(url), Some(ch_id), Some(cookie)) = (
            self.ship_url.lock().await.clone(),
            self.channel_id.lock().await.clone(),
            self.cookie.lock().await.clone(),
        ) {
            let channel_url = format!("{}/~/channel/{}", url.trim_end_matches('/'), ch_id);
            let _ = self.client.delete(&channel_url)
                .header("Cookie", &cookie)
                .send()
                .await;
        }

        *self.cookie.lock().await = None;
        *self.channel_id.lock().await = None;
        set_status(&self.status, ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "access_code" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'access_code'.",
                credentials.credential_type
            )));
        }

        let ship_url = credentials
            .data
            .get("ship_url")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'ship_url' in credentials data".into())
            })?
            .clone();

        let ship_name = credentials
            .data
            .get("ship_name")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'ship_name' in credentials data".into())
            })?
            .clone();

        let access_code = credentials
            .data
            .get("access_code")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'access_code' in credentials data".into())
            })?
            .clone();

        if ship_url.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Ship URL cannot be empty".into(),
            ));
        }

        if ship_name.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Ship name cannot be empty".into(),
            ));
        }

        if access_code.is_empty() {
            return Err(ChannelError::AuthFailed(
                "Access code cannot be empty".into(),
            ));
        }

        *self.ship_url.lock().await = Some(ship_url);
        *self.ship_name.lock().await = Some(ship_name);
        *self.access_code.lock().await = Some(access_code);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.ship_url.lock().await = None;
        *self.ship_name.lock().await = None;
        *self.access_code.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected(
                "Urbit not connected".into(),
            ));
        }

        let cookie = self.cookie.lock().await.clone()
            .ok_or_else(|| {
                ChannelError::NotConnected("No session cookie -- ship not authenticated".into())
            })?;

        let ship_url = self.ship_url.lock().await.clone()
            .ok_or_else(|| ChannelError::NotConnected("Ship URL not set".into()))?;

        let ship_name = self.ship_name.lock().await.clone()
            .ok_or_else(|| ChannelError::NotConnected("Ship name not set".into()))?;

        let ch_id = self.channel_id.lock().await.clone()
            .ok_or_else(|| ChannelError::NotConnected("Channel ID not set".into()))?;

        let channel_url = format!(
            "{}/~/channel/{}",
            ship_url.trim_end_matches('/'),
            ch_id
        );

        let ship_bare = ship_name.trim_start_matches('~').to_string();

        // Urbit messages don't have a strict limit but chunk at 8000 to be safe
        let chunks = chunk_message(&message.text, 8000);

        for chunk in chunks {
            let event_id = self.next_event_id();
            let poke_body = serde_json::json!([{
                "id": event_id,
                "action": "poke",
                "ship": ship_bare,
                "app": "chat-hook",
                "mark": "chat-action",
                "json": {
                    "message": {
                        "path": recipient,
                        "envelope": {
                            "uid": uuid::Uuid::new_v4().to_string(),
                            "number": event_id,
                            "author": format!("~{}", ship_bare),
                            "when": chrono::Utc::now().timestamp_millis(),
                            "letter": { "text": chunk }
                        }
                    }
                }
            }]);

            let resp = self.client
                .put(&channel_url)
                .header("Cookie", &cookie)
                .header("Content-Type", "application/json")
                .json(&poke_body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Urbit send failed: {e}")))?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed(format!(
                    "Urbit API error: {body}"
                )));
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct UrbitChannelFactory;

impl crate::ChannelPluginFactory for UrbitChannelFactory {
    fn channel_type(&self) -> &str { "urbit" }
    fn channel_type_name(&self) -> &str { "Urbit" }
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
        Box::new(UrbitChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_urbit_metadata() {
        let channel = UrbitChannel::new();
        assert_eq!(channel.id(), "urbit");
        assert_eq!(channel.name(), "Tlon/Urbit");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_urbit_features() {
        let channel = UrbitChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(!features.media_attachments);
        assert!(!features.reactions);
        assert!(!features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_urbit_login_bad_type() {
        let mut channel = UrbitChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_urbit_login_missing_fields() {
        let mut channel = UrbitChannel::new();

        // Missing ship_url
        let creds = ChannelCredentials {
            credential_type: "access_code".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ship_url"));

        // Missing ship_name
        let mut data = HashMap::new();
        data.insert("ship_url".to_string(), "http://localhost:8080".to_string());
        let creds = ChannelCredentials {
            credential_type: "access_code".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ship_name"));

        // Missing access_code
        let mut data = HashMap::new();
        data.insert("ship_url".to_string(), "http://localhost:8080".to_string());
        data.insert("ship_name".to_string(), "~zod".to_string());
        let creds = ChannelCredentials {
            credential_type: "access_code".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("access_code"));
    }

    #[tokio::test]
    async fn test_urbit_send_not_connected() {
        let channel = UrbitChannel::new();
        let msg = OutgoingMessage {
            text: "Hello from Urbit".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("~sampel-palnet", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
