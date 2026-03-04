//! Signal Channel Plugin
//!
//! Integrates with signal-cli REST API for Signal messaging.
//!
//! ## Prerequisites
//! Run the signal-cli-rest-api Docker container:
//! ```text
//! docker run -p 8080:8080 bbernhard/signal-cli-rest-api
//! ```
//!
//! ## Authentication
//! - `credential_type`: "api_key"
//!   - `data.api_url`: signal-cli REST API base URL (e.g., "http://localhost:8080")
//!   - `data.phone_number` (optional): registered phone number (e.g., "+15551234567")
//!     If omitted, login returns PendingApproval with QR code for device linking.
//!
//! ## Login Flow
//! - With phone_number: validates via GET {api_url}/v1/about, returns Success
//! - Without phone_number: POST {api_url}/v1/qrcodelink?device_name=omni,
//!   returns PendingApproval with QR code PNG (base64)
//!
//! ## Sending Messages
//! POST {api_url}/v2/send with JSON body
//!
//! ## Receiving Messages
//! Polling GET {api_url}/v1/receive/{phone_number} every 2s
//!
//! ## Features
//! - Direct messages
//! - Group messages
//! - Media attachments
//! - Reactions
//! - Read receipts
//! - Typing indicators

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Signal channel plugin using signal-cli REST API.
pub struct SignalChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// signal-cli REST API base URL.
    api_url: Mutex<Option<String>>,
    /// Registered phone number.
    phone_number: Mutex<Option<String>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the polling task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl SignalChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            client: reqwest::Client::new(),
            api_url: Mutex::new(None),
            phone_number: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for SignalChannel {
    fn id(&self) -> &str {
        "signal"
    }

    fn name(&self) -> &str {
        "Signal"
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
        let api_url = self.api_url.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("API URL not set. Call login() first.".into()))?;
        let phone = self.phone_number.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Phone number not set. Complete login first.".into()))?;

        set_status(&self.status, ConnectionStatus::Connecting);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.incoming_tx.clone();
        let status = self.status.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let receive_url = format!(
                "{}/v1/receive/{}",
                api_url.trim_end_matches('/'),
                phone
            );

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Signal: polling task shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                        let resp = client.get(&receive_url)
                            .timeout(std::time::Duration::from_secs(30))
                            .send()
                            .await;

                        let resp = match resp {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::debug!("Signal poll error: {e}");
                                continue;
                            }
                        };

                        if !resp.status().is_success() {
                            tracing::debug!("Signal poll returned HTTP {}", resp.status());
                            continue;
                        }

                        let messages: Vec<serde_json::Value> = match resp.json().await {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        for msg in &messages {
                            let envelope = match msg.get("envelope") {
                                Some(e) => e,
                                None => continue,
                            };

                            // Get data message
                            let data_msg = match envelope.get("dataMessage") {
                                Some(d) => d,
                                None => continue,
                            };

                            let text = data_msg.get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            if text.is_empty() {
                                continue;
                            }

                            let sender = envelope.get("source")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let sender_name = envelope.get("sourceName")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let timestamp_ms = envelope.get("timestamp")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            let timestamp = chrono::DateTime::from_timestamp_millis(timestamp_ms)
                                .unwrap_or_else(chrono::Utc::now);

                            let group_id = data_msg.get("groupInfo")
                                .and_then(|v| v.get("groupId"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());

                            let is_group = group_id.is_some();

                            let incoming = IncomingMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                channel_id: "signal".to_string(),
                                channel_type: "signal".to_string(),
                                instance_id: "default".to_string(),
                                sender,
                                sender_name,
                                text,
                                is_group,
                                group_id,
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

        let api_url = credentials
            .data
            .get("api_url")
            .ok_or_else(|| {
                ChannelError::AuthFailed("Missing 'api_url' in credentials data".into())
            })?
            .clone();

        if api_url.is_empty() {
            return Err(ChannelError::AuthFailed("API URL cannot be empty".into()));
        }

        let phone_number = credentials.data.get("phone_number").cloned();

        if let Some(ref phone) = phone_number {
            if !phone.is_empty() {
                // Validate the API is reachable
                let about_url = format!("{}/v1/about", api_url.trim_end_matches('/'));
                match self.client.get(&about_url).send().await {
                    Ok(r) if r.status().is_success() => {
                        tracing::info!("Signal CLI REST API validated at {api_url}");
                    }
                    Ok(r) => {
                        tracing::warn!("Signal CLI REST API returned HTTP {}", r.status());
                    }
                    Err(e) => {
                        tracing::warn!("Signal CLI REST API not reachable: {e}");
                    }
                }

                *self.api_url.lock().await = Some(api_url);
                *self.phone_number.lock().await = Some(phone.clone());
                return Ok(LoginStatus::Success);
            }
        }

        // No phone number -- request QR code for device linking
        let qr_url = format!(
            "{}/v1/qrcodelink?device_name=omni",
            api_url.trim_end_matches('/')
        );

        let resp = self.client.get(&qr_url)
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Failed to get QR code: {e}")))?;

        if !resp.status().is_success() {
            return Err(ChannelError::AuthFailed(format!(
                "QR code request returned HTTP {}",
                resp.status()
            )));
        }

        let qr_bytes = resp.bytes().await
            .map_err(|e| ChannelError::AuthFailed(format!("Failed to read QR code bytes: {e}")))?;

        let qr_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &qr_bytes,
        );

        *self.api_url.lock().await = Some(api_url);

        Ok(LoginStatus::PendingApproval {
            qr_code_data: Some(qr_base64),
        })
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.api_url.lock().await = None;
        *self.phone_number.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Signal not connected".into()));
        }

        let api_url = self.api_url.lock().await.clone()
            .ok_or_else(|| ChannelError::NotConnected("API URL not set".into()))?;
        let phone = self.phone_number.lock().await.clone()
            .ok_or_else(|| ChannelError::NotConnected("Phone number not set".into()))?;

        let send_url = format!("{}/v2/send", api_url.trim_end_matches('/'));

        // Signal has no strict message limit, chunk at 8000 as safe default
        let chunks = chunk_message(&message.text, 8000);

        for chunk in chunks {
            let body = serde_json::json!({
                "message": chunk,
                "number": phone,
                "recipients": [recipient],
            });

            let resp = self.client.post(&send_url)
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Signal send failed: {e}")))?;

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed(format!(
                    "Signal API error: {body}"
                )));
            }
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct SignalChannelFactory;

impl crate::ChannelPluginFactory for SignalChannelFactory {
    fn channel_type(&self) -> &str { "signal" }
    fn channel_type_name(&self) -> &str { "Signal" }
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
        Box::new(SignalChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_signal_metadata() {
        let channel = SignalChannel::new();
        assert_eq!(channel.id(), "signal");
        assert_eq!(channel.name(), "Signal");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_signal_features() {
        let channel = SignalChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(features.read_receipts);
        assert!(features.typing_indicators);
    }

    #[tokio::test]
    async fn test_signal_login_bad_type() {
        let mut channel = SignalChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_signal_login_missing_api_url() {
        let mut channel = SignalChannel::new();
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("api_url"));
    }

    #[tokio::test]
    async fn test_signal_send_not_connected() {
        let channel = SignalChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("+15551234567", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }

    #[tokio::test]
    async fn test_signal_connect_without_login() {
        let mut channel = SignalChannel::new();
        let config = ChannelConfig {
            settings: HashMap::new(),
        };
        let result = channel.connect(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API URL not set"));
    }
}
