//! Telegram Channel Plugin
//!
//! Implements the ChannelPlugin trait for Telegram using the teloxide crate.
//! Free to use -- only requires a bot token from @BotFather.
//!
//! ## Setup
//! 1. Open Telegram and message @BotFather
//! 2. Send /newbot and follow the prompts
//! 3. Copy the bot token provided
//!
//! ## Authentication
//! - `credential_type`: "bot_token"
//! - `data.token`: your bot token from @BotFather (e.g. "123456:ABCdefGHI...")

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::ChatId;
use tokio::sync::{mpsc, Mutex, oneshot};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
};

/// Telegram channel plugin using teloxide.
pub struct TelegramChannel {
    /// Bot token.
    token: Mutex<Option<String>>,
    /// Connection status.
    status: Arc<AtomicU8>,
    /// Teloxide Bot instance for sending messages.
    bot: Mutex<Option<Bot>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the polling task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl TelegramChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            token: Mutex::new(None),
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            bot: Mutex::new(None),
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

#[async_trait::async_trait]
impl ChannelPlugin for TelegramChannel {
    fn id(&self) -> &str {
        "telegram"
    }

    fn name(&self) -> &str {
        "Telegram"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: true,
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

        let bot = Bot::new(&token);
        *self.bot.lock().await = Some(bot.clone());

        let incoming_tx = self.incoming_tx.clone();
        let status = self.status.clone();

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        // Spawn polling task
        tokio::spawn(async move {
            status.store(ConnectionStatus::Connected as u8, Ordering::Relaxed);
            tracing::info!("Telegram bot polling started");

            // Build the dispatcher with a message handler
            let handler = Update::filter_message().endpoint(
                move |msg: Message, _bot: Bot| {
                    let tx = incoming_tx.clone();
                    async move {
                        let text = msg.text().unwrap_or("").to_string();

                        // Extract media file ID if present (photo, document, audio, video)
                        let media_url = msg.photo()
                            .and_then(|photos| photos.last())
                            .map(|p| format!("tg://file/{}", p.file.id))
                            .or_else(|| msg.document().map(|d| format!("tg://file/{}", d.file.id)))
                            .or_else(|| msg.audio().map(|a| format!("tg://file/{}", a.file.id)))
                            .or_else(|| msg.video().map(|v| format!("tg://file/{}", v.file.id)));

                        // Skip only if both text and media are absent
                        if text.is_empty() && media_url.is_none() {
                            return respond(());
                        }

                        let sender = msg
                            .from
                            .as_ref()
                            .map(|u| u.id.0.to_string())
                            .unwrap_or_default();
                        let sender_name = msg
                            .from
                            .as_ref()
                            .map(|u| {
                                u.last_name
                                    .as_ref()
                                    .map(|ln| format!("{} {}", u.first_name, ln))
                                    .unwrap_or_else(|| u.first_name.clone())
                            });

                        let is_group = msg.chat.is_group() || msg.chat.is_supergroup();

                        let incoming = IncomingMessage {
                            id: msg.id.0.to_string(),
                            channel_id: "telegram".to_string(),
                            channel_type: "telegram".to_string(),
                            instance_id: "default".to_string(),
                            sender,
                            sender_name,
                            text,
                            is_group,
                            group_id: if is_group {
                                Some(msg.chat.id.0.to_string())
                            } else {
                                None
                            },
                            thread_id: msg.thread_id.map(|tid| tid.to_string()),
                            timestamp: chrono::DateTime::from_timestamp(
                                msg.date.timestamp(),
                                0,
                            )
                            .unwrap_or_else(chrono::Utc::now),
                            media_url,
                            source_trust_level: crate::SourceTrustLevel::Authenticated,
                        };

                        if let Err(e) = tx.send(incoming).await {
                            tracing::warn!("Failed to forward Telegram message: {e}");
                        }

                        respond(())
                    }
                },
            );

            let mut dispatcher = Dispatcher::builder(bot, handler)
                .enable_ctrlc_handler()
                .build();

            let shutdown_token = dispatcher.shutdown_token();

            tokio::select! {
                _ = dispatcher.dispatch() => {
                    tracing::info!("Telegram dispatcher stopped");
                }
                _ = &mut shutdown_rx => {
                    tracing::info!("Telegram polling shutting down");
                    let _ = shutdown_token.shutdown();
                }
            }

            status.store(ConnectionStatus::Disconnected as u8, Ordering::Relaxed);
        });

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(());
        }
        *self.bot.lock().await = None;
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

        // Validate token by calling getMe
        let bot = Bot::new(&token);
        match bot.get_me().await {
            Ok(me) => {
                tracing::info!("Telegram authenticated as: @{}", me.username());
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
        let bot = self
            .bot
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Telegram not connected".into()))?;

        // Parse chat ID
        let chat_id: i64 = recipient
            .parse()
            .map_err(|_| ChannelError::SendFailed(format!("Invalid chat ID: {recipient}")))?;

        // Telegram has a 4096 character limit per message
        let chunks = chunk_message(&message.text, 4096);

        // Parse thread_id for topic/forum support
        let thread_id = message
            .thread_id
            .as_deref()
            .and_then(|tid| tid.parse::<i32>().ok())
            .map(|id| teloxide::types::ThreadId(teloxide::types::MessageId(id)));

        for chunk in chunks {
            let mut req = bot.send_message(ChatId(chat_id), chunk);
            if let Some(tid) = thread_id {
                req = req.message_thread_id(tid);
            }
            req.await
                .map_err(|e| ChannelError::SendFailed(format!("Telegram send failed: {e}")))?;
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }
}

/// Split a message into chunks that fit within Telegram's character limit.
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

        let split_at = remaining[..max_len]
            .rfind('\n')
            .unwrap_or(max_len);

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk.to_string());
        remaining = rest.trim_start_matches('\n');
    }

    chunks
}


pub struct TelegramChannelFactory;

impl crate::ChannelPluginFactory for TelegramChannelFactory {
    fn channel_type(&self) -> &str { "telegram" }
    fn channel_type_name(&self) -> &str { "Telegram" }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: true,
            media_attachments: true,
            reactions: true,
            read_receipts: true,
            typing_indicators: true,
            threads: true,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(TelegramChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_channel_metadata() {
        let channel = TelegramChannel::new();
        assert_eq!(channel.id(), "telegram");
        assert_eq!(channel.name(), "Telegram");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_telegram_features() {
        let channel = TelegramChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(features.read_receipts);
        assert!(features.typing_indicators);
        assert!(features.threads);
    }

    #[test]
    fn test_telegram_chunk_message() {
        let chunks = chunk_message("Hello", 4096);
        assert_eq!(chunks.len(), 1);

        let long = "x".repeat(10000);
        let chunks = chunk_message(&long, 4096);
        assert!(chunks.len() >= 3);
    }

    #[tokio::test]
    async fn test_telegram_login_bad_type() {
        let mut channel = TelegramChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth".to_string(),
            data: std::collections::HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_telegram_login_missing_token() {
        let mut channel = TelegramChannel::new();
        let creds = ChannelCredentials {
            credential_type: "bot_token".to_string(),
            data: std::collections::HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing"));
    }

    #[tokio::test]
    async fn test_telegram_send_not_connected() {
        let channel = TelegramChannel::new();
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
