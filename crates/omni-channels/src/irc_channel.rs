//! IRC Channel Plugin
//!
//! Plain TCP IRC client implementation using tokio.
//!
//! ## Authentication
//! - `credential_type`: "password"
//! - `data.nickname`: IRC nickname
//! - `data.password`: IRC password (NickServ or server password)
//! - `data.server`: IRC server hostname
//! - `data.port`: IRC server port (default "6667")
//!
//! ## Connection
//! TCP connection to {server}:{port}, authenticates with PASS/NICK/USER,
//! then JOINs configured channels.
//!
//! ## Features
//! DM + group (channels). Plain TCP only (port 6667); TLS would need additional config.

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
    common::{chunk_message, set_status, get_status},
};

/// IRC channel plugin using plain TCP.
pub struct IrcChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// IRC server hostname.
    server: Mutex<Option<String>>,
    /// IRC server port.
    port: Mutex<u16>,
    /// IRC nickname.
    nickname: Mutex<Option<String>>,
    /// IRC password (NickServ or server password).
    password: Mutex<Option<String>>,
    /// Channels to join on connect.
    channels_to_join: Mutex<Vec<String>>,
    /// Sender for outgoing IRC messages to the TCP writer.
    outgoing_tx: Mutex<Option<mpsc::Sender<String>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the connection task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl IrcChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            server: Mutex::new(None),
            port: Mutex::new(6667),
            nickname: Mutex::new(None),
            password: Mutex::new(None),
            channels_to_join: Mutex::new(Vec::new()),
            outgoing_tx: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }
}

/// Parse a PRIVMSG line from IRC.
/// Format: `:nickname!user@host PRIVMSG #channel :message text`
fn parse_privmsg(line: &str) -> Option<(&str, &str, &str)> {
    if !line.starts_with(':') {
        return None;
    }
    let rest = &line[1..];
    let bang = rest.find('!')?;
    let sender = &rest[..bang];

    let privmsg_idx = rest.find(" PRIVMSG ")?;
    let after_privmsg = &rest[privmsg_idx + 9..];

    let colon_idx = after_privmsg.find(" :")?;
    let target = &after_privmsg[..colon_idx];
    let message = &after_privmsg[colon_idx + 2..];

    Some((sender, target, message))
}

#[async_trait::async_trait]
impl ChannelPlugin for IrcChannel {
    fn id(&self) -> &str {
        "irc"
    }

    fn name(&self) -> &str {
        "IRC"
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

    async fn connect(&mut self, config: ChannelConfig) -> Result<()> {
        let server = self.server.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Server not set. Call login() first.".into()))?;
        let port = *self.port.lock().await;
        let nickname = self.nickname.lock().await.clone()
            .ok_or_else(|| ChannelError::Config("Nickname not set. Call login() first.".into()))?;
        let password = self.password.lock().await.clone();

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

        // Connect via TCP
        let addr = format!("{server}:{port}");
        let stream = tokio::net::TcpStream::connect(&addr).await
            .map_err(|e| ChannelError::ConnectionFailed(format!("IRC TCP connect to {addr} failed: {e}")))?;

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // Authenticate
        if let Some(ref pw) = password {
            if !pw.is_empty() {
                writer.write_all(format!("PASS {pw}\r\n").as_bytes()).await
                    .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to send PASS: {e}")))?;
            }
        }

        writer.write_all(format!("NICK {nickname}\r\n").as_bytes()).await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to send NICK: {e}")))?;

        writer.write_all(format!("USER {nickname} 0 * :Omni Bot\r\n").as_bytes()).await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to send USER: {e}")))?;

        // Join channels
        for ch in &channels_to_join {
            let ch_name = if ch.starts_with('#') { ch.clone() } else { format!("#{ch}") };
            writer.write_all(format!("JOIN {ch_name}\r\n").as_bytes()).await
                .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to JOIN {ch_name}: {e}")))?;
        }

        // Set up outgoing message channel
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(128);
        *self.outgoing_tx.lock().await = Some(outgoing_tx);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.incoming_tx.clone();
        let status = self.status.clone();
        let bot_nick = nickname.to_lowercase();

        tokio::spawn(async move {
            let mut line_buf = String::new();

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("IRC: connection task shutting down");
                        let _ = writer.write_all(b"QUIT :Goodbye\r\n").await;
                        break;
                    }
                    // Send outgoing messages
                    Some(msg) = outgoing_rx.recv() => {
                        if writer.write_all(msg.as_bytes()).await.is_err() {
                            tracing::warn!("IRC: failed to write to socket");
                            break;
                        }
                    }
                    // Read incoming lines
                    result = buf_reader.read_line(&mut line_buf) => {
                        match result {
                            Ok(0) => {
                                tracing::info!("IRC: connection closed");
                                break;
                            }
                            Ok(_) => {
                                let line = line_buf.trim_end_matches('\n').trim_end_matches('\r');

                                // Handle PING
                                if line.starts_with("PING") {
                                    let pong = line.replacen("PING", "PONG", 1);
                                    let _ = writer.write_all(format!("{pong}\r\n").as_bytes()).await;
                                    line_buf.clear();
                                    continue;
                                }

                                // Parse PRIVMSG
                                if let Some((sender, target, message)) = parse_privmsg(line) {
                                    // Skip our own messages
                                    if sender.to_lowercase() == bot_nick {
                                        line_buf.clear();
                                        continue;
                                    }

                                    let is_group = target.starts_with('#');

                                    let incoming = IncomingMessage {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        channel_id: "irc".to_string(),
                                        channel_type: "irc".to_string(),
                                        instance_id: "default".to_string(),
                                        sender: sender.to_string(),
                                        sender_name: Some(sender.to_string()),
                                        text: message.to_string(),
                                        is_group,
                                        group_id: if is_group { Some(target.to_string()) } else { None },
                                        thread_id: None,
                                        timestamp: chrono::Utc::now(),
                                        media_url: None,
                                        source_trust_level: crate::SourceTrustLevel::Authenticated,
                                    };

                                    let _ = tx.send(incoming).await;
                                }

                                line_buf.clear();
                            }
                            Err(e) => {
                                tracing::warn!("IRC: read error: {e}");
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
        *self.outgoing_tx.lock().await = None;
        set_status(&self.status, ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "password" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'password'.",
                credentials.credential_type
            )));
        }

        let nickname = credentials
            .data
            .get("nickname")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'nickname' in credentials data".into()))?
            .clone();

        let password = credentials
            .data
            .get("password")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'password' in credentials data".into()))?
            .clone();

        let server = credentials
            .data
            .get("server")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'server' in credentials data".into()))?
            .clone();

        if nickname.is_empty() || server.is_empty() {
            return Err(ChannelError::AuthFailed("nickname and server cannot be empty".into()));
        }

        // Parse optional port (defaults to 6667)
        if let Some(port_str) = credentials.data.get("port") {
            let port: u16 = port_str
                .parse()
                .map_err(|_| ChannelError::AuthFailed(format!("Invalid port: {port_str}")))?;
            *self.port.lock().await = port;
        }

        *self.nickname.lock().await = Some(nickname);
        *self.password.lock().await = Some(password);
        *self.server.lock().await = Some(server);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.nickname.lock().await = None;
        *self.password.lock().await = None;
        *self.server.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        if get_status(&self.status) != ConnectionStatus::Connected {
            return Err(ChannelError::NotConnected("IRC not connected".into()));
        }

        let outgoing = self.outgoing_tx.lock().await;
        let sender = outgoing.as_ref()
            .ok_or_else(|| ChannelError::NotConnected("TCP connection not available".into()))?;

        // IRC PRIVMSG has a 512-byte line limit (including protocol overhead).
        // Safe message content limit is ~450 characters.
        let chunks = chunk_message(&message.text, 450);

        for chunk in chunks {
            let irc_msg = format!("PRIVMSG {} :{}\r\n", recipient, chunk);
            sender.send(irc_msg).await
                .map_err(|e| ChannelError::SendFailed(format!("Failed to queue IRC message: {e}")))?;
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}


pub struct IrcChannelFactory;

impl crate::ChannelPluginFactory for IrcChannelFactory {
    fn channel_type(&self) -> &str { "irc" }
    fn channel_type_name(&self) -> &str { "IRC" }
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
        Box::new(IrcChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_irc_metadata() {
        let channel = IrcChannel::new();
        assert_eq!(channel.id(), "irc");
        assert_eq!(channel.name(), "IRC");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_irc_features() {
        let channel = IrcChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(!features.media_attachments);
        assert!(!features.reactions);
        assert!(!features.read_receipts);
        assert!(!features.typing_indicators);
    }

    #[tokio::test]
    async fn test_irc_login_bad_type() {
        let mut channel = IrcChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_irc_login_missing_fields() {
        let mut channel = IrcChannel::new();
        // Missing server and password
        let mut data = HashMap::new();
        data.insert("nickname".to_string(), "omni_bot".to_string());
        let creds = ChannelCredentials {
            credential_type: "password".to_string(),
            data,
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("password"));
    }

    #[tokio::test]
    async fn test_irc_send_not_connected() {
        let channel = IrcChannel::new();
        let msg = OutgoingMessage {
            text: "Hello IRC".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("#general", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }

    #[test]
    fn test_parse_privmsg() {
        let line = ":nick!user@host PRIVMSG #channel :Hello World";
        let (sender, target, message) = parse_privmsg(line).unwrap();
        assert_eq!(sender, "nick");
        assert_eq!(target, "#channel");
        assert_eq!(message, "Hello World");

        // Direct message
        let line = ":nick!user@host PRIVMSG botname :private message";
        let (sender, target, message) = parse_privmsg(line).unwrap();
        assert_eq!(sender, "nick");
        assert_eq!(target, "botname");
        assert_eq!(message, "private message");

        // Not a PRIVMSG
        let line = ":server 001 botname :Welcome";
        assert!(parse_privmsg(line).is_none());
    }

    #[tokio::test]
    async fn test_irc_connect_without_login() {
        let mut channel = IrcChannel::new();
        let config = ChannelConfig {
            settings: HashMap::new(),
        };
        let result = channel.connect(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Server not set"));
    }
}
