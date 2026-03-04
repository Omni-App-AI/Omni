//! Twitch Channel Plugin
//!
//! Connects to Twitch IRC over WebSocket (wss://irc-ws.chat.twitch.tv:443).
//!
//! ## Authentication
//! - `credential_type`: "oauth"
//! - `data.oauth_token`: Twitch OAuth token (with chat:read + chat:edit scopes)
//! - `data.username`: Twitch username (bot account)
//!
//! ## Connection
//! WebSocket to wss://irc-ws.chat.twitch.tv:443, authenticates with
//! PASS oauth:{token}, NICK {username}, and optionally requests
//! twitch.tv/tags and twitch.tv/commands capabilities.
//!
//! ## Features
//! Group only (Twitch chat channels). Whispers are heavily rate-limited.

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex, oneshot};
use futures::stream::StreamExt;
use futures::SinkExt;

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// Twitch channel plugin using WebSocket IRC.
pub struct TwitchChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// Twitch OAuth token.
    oauth_token: Mutex<Option<String>>,
    /// Twitch username (bot account).
    username: Mutex<Option<String>>,
    /// Channels to join on connect.
    channels_to_join: Mutex<Vec<String>>,
    /// Sender for outgoing IRC messages to the WebSocket writer.
    outgoing_tx: Mutex<Option<mpsc::Sender<String>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the connection task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl TwitchChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            oauth_token: Mutex::new(None),
            username: Mutex::new(None),
            channels_to_join: Mutex::new(Vec::new()),
            outgoing_tx: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }
}

/// Parse a PRIVMSG line from Twitch IRC.
/// Format: `:nickname!user@host PRIVMSG #channel :message text`
fn parse_privmsg(line: &str) -> Option<(&str, &str, &str)> {
    // Find the sender
    if !line.starts_with(':') {
        return None;
    }
    let rest = &line[1..];
    let bang = rest.find('!')?;
    let sender = &rest[..bang];

    // Find PRIVMSG
    let privmsg_idx = rest.find(" PRIVMSG ")?;
    let after_privmsg = &rest[privmsg_idx + 9..]; // skip " PRIVMSG "

    // Find channel and message
    let colon_idx = after_privmsg.find(" :")?;
    let channel = &after_privmsg[..colon_idx];
    let message = &after_privmsg[colon_idx + 2..];

    Some((sender, channel, message))
}

#[async_trait::async_trait]
impl ChannelPlugin for TwitchChannel {
    fn id(&self) -> &str {
        "twitch"
    }

    fn name(&self) -> &str {
        "Twitch"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: false,
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

    async fn connect(&mut self, config: ChannelConfig) -> Result<()> {
        let token = self.oauth_token.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("OAuth token not set. Call login() first.".into()))?;
        let username = self.username.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Username not set. Call login() first.".into()))?;

        // Extract optional channel list from config
        if let Some(channels) = config.settings.get("channels") {
            if let Some(arr) = channels.as_array() {
                let mut to_join = self.channels_to_join.lock().await;
                for ch in arr {
                    if let Some(s) = ch.as_str() {
                        to_join.push(s.to_string());
                    }
                }
            }
        }

        let channels_to_join = self.channels_to_join.lock().await.clone();

        set_status(&self.status, ConnectionStatus::Connecting);

        // Connect to Twitch IRC WebSocket
        let (ws_stream, _) = tokio_tungstenite::connect_async("wss://irc-ws.chat.twitch.tv:443")
            .await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Twitch WS connect failed: {e}")))?;

        let (mut ws_writer, mut ws_reader) = ws_stream.split();

        // Authenticate
        use tokio_tungstenite::tungstenite::Message;
        ws_writer.send(Message::Text(format!("PASS oauth:{token}").into())).await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to send PASS: {e}")))?;
        ws_writer.send(Message::Text(format!("NICK {username}").into())).await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to send NICK: {e}")))?;

        // Request capabilities
        ws_writer.send(Message::Text(
            "CAP REQ :twitch.tv/tags twitch.tv/commands".into()
        )).await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to send CAP REQ: {e}")))?;

        // Join channels
        for ch in &channels_to_join {
            let ch_name = if ch.starts_with('#') { ch.clone() } else { format!("#{ch}") };
            ws_writer.send(Message::Text(format!("JOIN {ch_name}").into())).await
                .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to JOIN {ch_name}: {e}")))?;
        }

        // Set up outgoing message channel
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(128);
        *self.outgoing_tx.lock().await = Some(outgoing_tx);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.incoming_tx.clone();
        let status = self.status.clone();
        let bot_username = username.to_lowercase();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Twitch: WebSocket task shutting down");
                        break;
                    }
                    // Send outgoing messages
                    Some(msg) = outgoing_rx.recv() => {
                        if ws_writer.send(Message::Text(msg.into())).await.is_err() {
                            tracing::warn!("Twitch: failed to send outgoing message");
                            break;
                        }
                    }
                    // Receive incoming messages
                    Some(frame) = ws_reader.next() => {
                        let frame = match frame {
                            Ok(f) => f,
                            Err(e) => {
                                tracing::warn!("Twitch: WS read error: {e}");
                                break;
                            }
                        };

                        let text = match frame {
                            Message::Text(t) => t.to_string(),
                            Message::Ping(data) => {
                                let _ = ws_writer.send(Message::Pong(data)).await;
                                continue;
                            }
                            Message::Close(_) => break,
                            _ => continue,
                        };

                        // IRC can have multiple lines in one frame
                        for line in text.lines() {
                            let line = line.trim_end_matches('\r');

                            // Handle PING
                            if line.starts_with("PING") {
                                let pong = line.replacen("PING", "PONG", 1);
                                let _ = ws_writer.send(Message::Text(pong.into())).await;
                                continue;
                            }

                            // Parse PRIVMSG
                            // Lines with tags: @tag1=val;tag2=val :nick!user@host PRIVMSG #ch :msg
                            let irc_line = if line.starts_with('@') {
                                // Skip tags
                                match line.find(' ') {
                                    Some(idx) => &line[idx + 1..],
                                    None => continue,
                                }
                            } else {
                                line
                            };

                            if let Some((sender, channel, message)) = parse_privmsg(irc_line) {
                                // Skip our own messages
                                if sender.to_lowercase() == bot_username {
                                    continue;
                                }

                                let incoming = IncomingMessage {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    channel_id: "twitch".to_string(),
                                    channel_type: "twitch".to_string(),
                                    instance_id: "default".to_string(),
                                    sender: sender.to_string(),
                                    sender_name: Some(sender.to_string()),
                                    text: message.to_string(),
                                    is_group: true,
                                    group_id: Some(channel.to_string()),
                                    thread_id: None,
                                    timestamp: chrono::Utc::now(),
                                    media_url: None,
                                    source_trust_level: crate::SourceTrustLevel::Authenticated,
                                };

                                let _ = tx.send(incoming).await;
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
        *self.outgoing_tx.lock().await = None;
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

        let token = credentials
            .data
            .get("oauth_token")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'oauth_token' in credentials data".into()))?
            .clone();

        let username = credentials
            .data
            .get("username")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'username' in credentials data".into()))?
            .clone();

        if token.is_empty() || username.is_empty() {
            return Err(ChannelError::AuthFailed("oauth_token and username cannot be empty".into()));
        }

        *self.oauth_token.lock().await = Some(token);
        *self.username.lock().await = Some(username);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.oauth_token.lock().await = None;
        *self.username.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("Twitch not connected".into()));
        }

        let outgoing = self.outgoing_tx.lock().await;
        let sender = outgoing.as_ref()
            .ok_or_else(|| ChannelError::NotConnected("WebSocket not available".into()))?;

        let channel = if recipient.starts_with('#') {
            recipient.to_string()
        } else {
            format!("#{recipient}")
        };

        // Twitch chat has a 500-character limit per message
        let chunks = chunk_message(&message.text, 500);

        for chunk in chunks {
            let irc_msg = format!("PRIVMSG {} :{}", channel, chunk);
            sender.send(irc_msg).await
                .map_err(|e| ChannelError::SendFailed(format!("Failed to queue message: {e}")))?;
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct TwitchChannelFactory;

impl crate::ChannelPluginFactory for TwitchChannelFactory {
    fn channel_type(&self) -> &str { "twitch" }
    fn channel_type_name(&self) -> &str { "Twitch" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: false,
            group_messages: true,
            media_attachments: false,
            reactions: false,
            read_receipts: false,
            typing_indicators: false,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(TwitchChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_twitch_metadata() {
        let channel = TwitchChannel::new();
        assert_eq!(channel.id(), "twitch");
        assert_eq!(channel.name(), "Twitch");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_twitch_features() {
        let channel = TwitchChannel::new();
        let features = channel.features();
        assert!(!features.direct_messages);
        assert!(features.group_messages);
        assert!(!features.media_attachments);
        assert!(!features.reactions);
        assert!(!features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_twitch_login_bad_type() {
        let mut channel = TwitchChannel::new();
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_twitch_send_not_connected() {
        let channel = TwitchChannel::new();
        let msg = OutgoingMessage {
            text: "Hello Twitch".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("streamerchannel", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }

    #[test]
    fn test_parse_privmsg() {
        let line = ":tmi.twitch.tv 001 botname :Welcome, GLHF!";
        assert!(parse_privmsg(line).is_none());

        let line = ":username!username@username.tmi.twitch.tv PRIVMSG #channel :Hello World";
        let (sender, channel, message) = parse_privmsg(line).unwrap();
        assert_eq!(sender, "username");
        assert_eq!(channel, "#channel");
        assert_eq!(message, "Hello World");
    }

    #[tokio::test]
    async fn test_twitch_connect_without_login() {
        let mut channel = TwitchChannel::new();
        let config = ChannelConfig {
            settings: HashMap::new(),
        };
        let result = channel.connect(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("OAuth token not set"));
    }
}
