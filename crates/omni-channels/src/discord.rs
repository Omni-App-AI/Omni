//! Discord Channel Plugin
//!
//! Implements the ChannelPlugin trait for Discord using the serenity crate.
//! Free to use -- only requires a bot token from Discord Developer Portal.
//!
//! ## Setup
//! 1. Create an application at https://discord.com/developers/applications
//! 2. Add a bot to the application
//! 3. Enable "Message Content Intent" under Privileged Gateway Intents
//! 4. Copy the bot token
//! 5. Invite the bot to your server with Message permissions
//!
//! ## Authentication
//! - `credential_type`: "bot_token"
//! - `data.token`: your bot token

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use serenity::all::{
    Context, CreateMessage, EventHandler, GatewayIntents, Message, Ready,
};
use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
};

/// Discord channel plugin using serenity.
pub struct DiscordChannel {
    /// Bot token.
    token: Mutex<Option<String>>,
    /// Connection status.
    status: Arc<AtomicU8>,
    /// Serenity HTTP client for sending messages.
    http: Mutex<Option<Arc<serenity::http::Http>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the gateway task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl DiscordChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            token: Mutex::new(None),
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            http: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }

    fn set_status(&self, status: ConnectionStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }

    fn get_status(&self) -> ConnectionStatus {
        match self.status.load(Ordering::Relaxed) {
            0 => ConnectionStatus::Disconnected,
            1 => ConnectionStatus::Connecting,
            2 => ConnectionStatus::Connected,
            3 => ConnectionStatus::Reconnecting,
            _ => ConnectionStatus::Error,
        }
    }
}

/// Serenity event handler that forwards Discord messages to our channel system.
struct Handler {
    incoming_tx: mpsc::Sender<IncomingMessage>,
    status: Arc<AtomicU8>,
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        // Ignore bot messages
        if msg.author.bot {
            return;
        }

        // In Discord, threads are channels. We detect threads by checking if the
        // message has a message_reference (indicating a reply/thread context).
        // The channel_id itself is the thread channel in Discord's model, so
        // we use it as the thread_id for the outgoing path.
        let thread_id = if msg.guild_id.is_some() {
            msg.message_reference.as_ref().map(|r| {
                // The referenced channel is the parent; our channel_id is the thread
                if r.channel_id != msg.channel_id {
                    msg.channel_id.to_string()
                } else {
                    // Same channel reply -- still pass channel_id as thread context
                    msg.channel_id.to_string()
                }
            })
        } else {
            None
        };

        let incoming = IncomingMessage {
            id: msg.id.to_string(),
            channel_id: "discord".to_string(),
            channel_type: "discord".to_string(),
            instance_id: "default".to_string(),
            sender: msg.author.id.to_string(),
            sender_name: Some(msg.author.name.clone()),
            text: msg.content.clone(),
            is_group: msg.guild_id.is_some(),
            group_id: msg.guild_id.map(|g| g.to_string()),
            thread_id,
            timestamp: chrono::DateTime::from_timestamp(
                msg.timestamp.unix_timestamp(),
                0,
            )
            .unwrap_or_else(chrono::Utc::now),
            media_url: msg.attachments.first().map(|a| a.url.clone()),
            source_trust_level: crate::SourceTrustLevel::Authenticated,
        };

        if let Err(e) = self.incoming_tx.send(incoming).await {
            tracing::warn!("Failed to forward Discord message: {e}");
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        tracing::info!("Discord bot connected as: {}", ready.user.name);
        self.status.store(ConnectionStatus::Connected as u8, Ordering::Relaxed);
    }
}

#[async_trait::async_trait]
impl ChannelPlugin for DiscordChannel {
    fn id(&self) -> &str {
        "discord"
    }

    fn name(&self) -> &str {
        "Discord"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: false,
            typing_indicators: true,
            threads: true,
        }
    }

    fn status(&self) -> ConnectionStatus {
        self.get_status()
    }

    async fn connect(&mut self, _config: ChannelConfig) -> Result<()> {
        let token = self
            .token
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::Config("Bot token not set. Call login() first.".into()))?;

        self.set_status(ConnectionStatus::Connecting);

        // Create the HTTP client for sending messages
        let http = Arc::new(serenity::http::Http::new(&token));
        *self.http.lock().await = Some(http);

        // Create serenity client with required intents
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let status_clone = self.status.clone();

        let handler = Handler {
            incoming_tx: self.incoming_tx.clone(),
            status: status_clone,
        };

        let mut client = serenity::Client::builder(&token, intents)
            .event_handler(handler)
            .await
            .map_err(|e| ChannelError::Other(format!("Failed to create Discord client: {e}")))?;

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        // Spawn the gateway connection in a background task
        let status_for_task = self.status.clone();
        tokio::spawn(async move {
            tokio::select! {
                result = client.start() => {
                    if let Err(e) = result {
                        tracing::error!("Discord gateway error: {e}");
                        status_for_task.store(ConnectionStatus::Error as u8, Ordering::Relaxed);
                    }
                }
                _ = &mut shutdown_rx => {
                    tracing::info!("Discord gateway shutting down");
                    client.shard_manager.shutdown_all().await;
                }
            }
        });

        // The handler's ready() callback will set Connected status
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        *self.http.lock().await = None;
        self.set_status(ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        if credentials.credential_type != "bot_token" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'bot_token'.",
                credentials.credential_type
            )));
        }

        let token = credentials
            .data
            .get("token")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'token' in credentials data".into()))?
            .clone();

        if token.is_empty() {
            return Err(ChannelError::AuthFailed("Token cannot be empty".into()));
        }

        // Validate token by making an API call
        let http = serenity::http::Http::new(&token);
        match http.get_current_user().await {
            Ok(user) => {
                tracing::info!("Discord authenticated as: {}", user.name);
                *self.token.lock().await = Some(token);
                Ok(LoginStatus::Success)
            }
            Err(e) => Err(ChannelError::AuthFailed(format!(
                "Invalid bot token: {e}"
            ))),
        }
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.token.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        let http = self
            .http
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Discord not connected".into()))?;

        // If thread_id is provided, send to that channel (Discord threads are channels)
        let target = message.thread_id.as_deref().unwrap_or(recipient);
        let channel_id: u64 = target
            .parse()
            .map_err(|_| ChannelError::SendFailed(format!("Invalid channel/thread ID: {target}")))?;
        let channel = serenity::model::id::ChannelId::new(channel_id);

        // Discord has a 2000 character limit per message
        let chunks = chunk_message(&message.text, 2000);

        for chunk in chunks {
            let builder = CreateMessage::new().content(chunk);
            channel
                .send_message(&http, builder)
                .await
                .map_err(|e| ChannelError::SendFailed(format!("Discord send failed: {e}")))?;
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        // This is sync but we need to get the lock. Use try_lock.
        self.incoming_rx.try_lock().ok()?.take()
    }
}

/// Split a message into chunks that fit within Discord's character limit.
fn chunk_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Try to break at a newline
        let split_at = remaining[..max_len]
            .rfind('\n')
            .unwrap_or(max_len);

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk.to_string());
        remaining = rest.trim_start_matches('\n');
    }

    chunks
}


pub struct DiscordChannelFactory;

impl crate::ChannelPluginFactory for DiscordChannelFactory {
    fn channel_type(&self) -> &str { "discord" }
    fn channel_type_name(&self) -> &str { "Discord" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: false,
            typing_indicators: true,
            threads: true,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(DiscordChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_channel_metadata() {
        let channel = DiscordChannel::new();
        assert_eq!(channel.id(), "discord");
        assert_eq!(channel.name(), "Discord");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_discord_features() {
        let channel = DiscordChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(!features.read_receipts);
        assert!(features.typing_indicators);
        assert!(features.threads);
    }

    #[test]
    fn test_chunk_message() {
        // Short message -- no chunking
        let chunks = chunk_message("Hello", 2000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello");

        // Long message with newlines
        let long_msg = "Line 1\nLine 2\nLine 3";
        let chunks = chunk_message(long_msg, 10);
        assert!(chunks.len() > 1);

        // Very long message without newlines
        let long = "x".repeat(5000);
        let chunks = chunk_message(&long, 2000);
        assert!(chunks.len() >= 3);
    }

    #[tokio::test]
    async fn test_discord_login_bad_type() {
        let mut channel = DiscordChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: std::collections::HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_discord_login_missing_token() {
        let mut channel = DiscordChannel::new();
        let creds = ChannelCredentials {
            credential_type: "bot_token".to_string(),
            data: std::collections::HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing"));
    }

    #[tokio::test]
    async fn test_discord_send_not_connected() {
        let channel = DiscordChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("123456", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not connected"));
    }
}
