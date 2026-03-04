//! Twitter/X Channel Plugin
//!
//! Implements the ChannelPlugin trait for Twitter/X using the X API v2.
//! Uses OAuth 1.0a for user-context authentication (posting tweets, DMs).
//!
//! ## Setup
//! 1. Create a project at https://developer.x.com/
//! 2. Create an app with Read and Write permissions
//! 3. Generate consumer keys and access tokens
//! 4. Provide all four tokens via login()
//!
//! ## Authentication
//! - `credential_type`: "oauth1"
//! - `data.api_key`: Consumer API key
//! - `data.api_secret`: Consumer API secret
//! - `data.access_token`: Access token
//! - `data.access_token_secret`: Access token secret
//!
//! ## Messaging
//! - `send_message(recipient, msg)`:
//!   - If `recipient` starts with "tweet:", posts a tweet
//!   - Otherwise, sends a DM to the user ID
//! - Incoming: polls for mentions and DMs at a configurable interval

use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use tokio::sync::{mpsc, Mutex, oneshot};

use crate::common::{get_status, set_status};
use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
};

type HmacSha1 = Hmac<Sha1>;

const X_API_BASE: &str = "https://api.x.com/2";
const DEFAULT_POLL_INTERVAL_SECS: u64 = 30;
const TWEET_MAX_LEN: usize = 280;
const DM_MAX_LEN: usize = 10_000;

/// Initial backoff duration for rate limiting (seconds).
const RATE_LIMIT_INITIAL_BACKOFF_SECS: u64 = 5;
/// Maximum backoff duration for rate limiting (seconds).
const RATE_LIMIT_MAX_BACKOFF_SECS: u64 = 300;

/// Twitter/X channel plugin.
pub struct TwitterChannel {
    /// OAuth 1.0a credentials.
    credentials: Mutex<Option<TwitterCredentials>>,
    /// Connection status.
    status: Arc<AtomicU8>,
    /// HTTP client.
    client: reqwest::Client,
    /// Authenticated user ID (set after login).
    user_id: Mutex<Option<String>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Shutdown signal for the polling task.
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

#[derive(Clone)]
struct TwitterCredentials {
    api_key: String,
    api_secret: String,
    access_token: String,
    access_token_secret: String,
}

impl TwitterChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);
        Self {
            credentials: Mutex::new(None),
            status: Arc::new(AtomicU8::new(0)),
            client: reqwest::Client::new(),
            user_id: Mutex::new(None),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            shutdown_tx: Mutex::new(None),
        }
    }

    /// Generate OAuth 1.0a Authorization header for a request.
    ///
    /// `url` must be the base URL without query parameters.
    /// `params` must include all query parameters so they are signed per RFC 5849.
    fn oauth_header(
        creds: &TwitterCredentials,
        method: &str,
        url: &str,
        params: &[(&str, &str)],
    ) -> String {
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let nonce = uuid::Uuid::new_v4().to_string().replace('-', "");

        let mut oauth_params = vec![
            ("oauth_consumer_key", creds.api_key.as_str()),
            ("oauth_nonce", &nonce),
            ("oauth_signature_method", "HMAC-SHA1"),
            ("oauth_timestamp", &timestamp),
            ("oauth_token", creds.access_token.as_str()),
            ("oauth_version", "1.0"),
        ];

        // Combine OAuth params with request params for signature base (RFC 5849 §3.4.1.3.1)
        let mut all_params: Vec<(&str, &str)> = oauth_params.clone();
        all_params.extend_from_slice(params);
        all_params.sort_by_key(|(k, _)| *k);

        let param_string: String = all_params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let signature_base = format!(
            "{}&{}&{}",
            method.to_uppercase(),
            percent_encode(url),
            percent_encode(&param_string)
        );

        let signing_key = format!(
            "{}&{}",
            percent_encode(&creds.api_secret),
            percent_encode(&creds.access_token_secret)
        );

        let mut mac = HmacSha1::new_from_slice(signing_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(signature_base.as_bytes());
        let signature = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        oauth_params.push(("oauth_signature", &signature));

        let header = oauth_params
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join(", ");

        format!("OAuth {}", header)
    }

    /// Build a URL with query parameters and return (full_url, params_for_signing).
    fn build_query_url(base_url: &str, params: &[(&str, &str)]) -> String {
        if params.is_empty() {
            return base_url.to_string();
        }
        let query: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        format!("{}?{}", base_url, query)
    }

    /// Calculate backoff from X-Rate-Limit-Reset header or use exponential backoff.
    fn rate_limit_backoff(
        resp_headers: &reqwest::header::HeaderMap,
        consecutive_429s: u32,
    ) -> Duration {
        // Try X-Rate-Limit-Reset header (unix timestamp when rate limit resets)
        if let Some(reset_val) = resp_headers.get("x-ratelimit-reset") {
            if let Ok(reset_str) = reset_val.to_str() {
                if let Ok(reset_ts) = reset_str.parse::<i64>() {
                    let now = chrono::Utc::now().timestamp();
                    let wait_secs = (reset_ts - now).max(1) as u64;
                    // Cap at max backoff
                    return Duration::from_secs(wait_secs.min(RATE_LIMIT_MAX_BACKOFF_SECS));
                }
            }
        }

        // Exponential backoff: 5s, 10s, 20s, 40s, ..., capped at 300s
        let backoff = RATE_LIMIT_INITIAL_BACKOFF_SECS * 2u64.pow(consecutive_429s);
        Duration::from_secs(backoff.min(RATE_LIMIT_MAX_BACKOFF_SECS))
    }
}

/// Percent-encode a string per RFC 5849 (OAuth 1.0a).
fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[async_trait::async_trait]
impl ChannelPlugin for TwitterChannel {
    fn id(&self) -> &str {
        "twitter"
    }

    fn name(&self) -> &str {
        "Twitter/X"
    }

    fn features(&self) -> ChannelFeatures {
        ChannelFeatures {
            direct_messages: true,
            group_messages: false,
            media_attachments: true,
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
        let creds = self
            .credentials
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::Config("Not authenticated. Call login() first.".into()))?;

        let user_id = self
            .user_id
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::Config("User ID not set. Call login() first.".into()))?;

        set_status(&self.status, ConnectionStatus::Connecting);

        let poll_interval = config
            .settings
            .get("poll_interval_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_POLL_INTERVAL_SECS);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.incoming_tx.clone();
        let status = self.status.clone();
        let client = self.client.clone();

        set_status(&self.status, ConnectionStatus::Connected);

        // Spawn polling task for mentions and DMs
        tokio::spawn(async move {
            let mut last_mention_id: Option<String> = None;
            let mut last_dm_event_id: Option<String> = None;
            let mut consecutive_429s: u32 = 0;

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("Twitter polling shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(poll_interval)) => {
                        // ── Poll for mentions ────────────────────────────
                        let mentions_url = format!("{}/users/{}/mentions", X_API_BASE, user_id);
                        let mention_params: Vec<(&str, &str)> = if let Some(ref since_id) = last_mention_id {
                            vec![
                                ("since_id", since_id.as_str()),
                                ("tweet.fields", "created_at,author_id"),
                            ]
                        } else {
                            vec![
                                ("max_results", "5"),
                                ("tweet.fields", "created_at,author_id"),
                            ]
                        };

                        let query_url = Self::build_query_url(&mentions_url, &mention_params);
                        let auth = Self::oauth_header(&creds, "GET", &mentions_url, &mention_params);

                        match client
                            .get(&query_url)
                            .header("Authorization", &auth)
                            .send()
                            .await
                        {
                            Ok(resp) if resp.status().is_success() => {
                                consecutive_429s = 0;
                                if let Ok(body) = resp.json::<serde_json::Value>().await {
                                    if let Some(data) = body["data"].as_array() {
                                        for tweet in data.iter().rev() {
                                            let tweet_id = tweet["id"].as_str().unwrap_or("").to_string();
                                            let author_id = tweet["author_id"].as_str().unwrap_or("unknown").to_string();
                                            let text = tweet["text"].as_str().unwrap_or("").to_string();
                                            let created_at = tweet["created_at"]
                                                .as_str()
                                                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                                .unwrap_or_else(chrono::Utc::now);

                                            let incoming = IncomingMessage {
                                                id: tweet_id.clone(),
                                                channel_id: "twitter".to_string(),
                                                channel_type: "twitter".to_string(),
                                                instance_id: "default".to_string(),
                                                sender: author_id,
                                                sender_name: None,
                                                text,
                                                is_group: false,
                                                group_id: None,
                                                thread_id: None,
                                                timestamp: created_at,
                                                media_url: None,
                                                source_trust_level: crate::SourceTrustLevel::Authenticated,
                                            };

                                            if tx.send(incoming).await.is_err() {
                                                tracing::warn!("Twitter incoming channel closed");
                                                set_status(&status, ConnectionStatus::Error);
                                                return;
                                            }

                                            last_mention_id = Some(tweet_id);
                                        }
                                    }
                                }
                            }
                            Ok(resp) => {
                                let status_code = resp.status();
                                if status_code.as_u16() == 429 {
                                    consecutive_429s += 1;
                                    let backoff = Self::rate_limit_backoff(resp.headers(), consecutive_429s);
                                    tracing::warn!(
                                        "Twitter mentions rate limited (attempt {}), backing off {}s",
                                        consecutive_429s,
                                        backoff.as_secs()
                                    );
                                    tokio::time::sleep(backoff).await;
                                } else {
                                    consecutive_429s = 0;
                                    tracing::debug!("Twitter mentions poll: {}", status_code);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Twitter mentions poll error: {e}");
                            }
                        }

                        // ── Poll for DMs ─────────────────────────────────
                        let dm_url = format!("{}/dm_events", X_API_BASE);
                        let mut dm_params: Vec<(&str, &str)> = vec![
                            ("dm_event.fields", "id,text,sender_id,created_at"),
                            ("event_types", "MessageCreate"),
                            ("max_results", "5"),
                        ];
                        if let Some(ref since_id) = last_dm_event_id {
                            dm_params.push(("since_id", since_id.as_str()));
                        }

                        let dm_query_url = Self::build_query_url(&dm_url, &dm_params);
                        let dm_auth = Self::oauth_header(&creds, "GET", &dm_url, &dm_params);

                        match client
                            .get(&dm_query_url)
                            .header("Authorization", &dm_auth)
                            .send()
                            .await
                        {
                            Ok(resp) if resp.status().is_success() => {
                                if let Ok(body) = resp.json::<serde_json::Value>().await {
                                    if let Some(data) = body["data"].as_array() {
                                        for dm_event in data.iter().rev() {
                                            let event_id = dm_event["id"].as_str().unwrap_or("").to_string();
                                            let sender_id = dm_event["sender_id"].as_str().unwrap_or("unknown").to_string();
                                            let text = dm_event["text"].as_str().unwrap_or("").to_string();
                                            let created_at = dm_event["created_at"]
                                                .as_str()
                                                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                                .unwrap_or_else(chrono::Utc::now);

                                            // Skip DMs sent by ourselves
                                            if sender_id == user_id {
                                                last_dm_event_id = Some(event_id);
                                                continue;
                                            }

                                            let incoming = IncomingMessage {
                                                id: format!("dm:{}", event_id),
                                                channel_id: "twitter".to_string(),
                                                channel_type: "twitter".to_string(),
                                                instance_id: "default".to_string(),
                                                sender: sender_id,
                                                sender_name: None,
                                                text,
                                                is_group: false,
                                                group_id: None,
                                                thread_id: None,
                                                timestamp: created_at,
                                                media_url: None,
                                                source_trust_level: crate::SourceTrustLevel::Authenticated,
                                            };

                                            if tx.send(incoming).await.is_err() {
                                                tracing::warn!("Twitter incoming channel closed");
                                                set_status(&status, ConnectionStatus::Error);
                                                return;
                                            }

                                            last_dm_event_id = Some(event_id);
                                        }
                                    }
                                }
                            }
                            Ok(resp) => {
                                let status_code = resp.status();
                                if status_code.as_u16() == 429 {
                                    // Share backoff counter with mentions
                                    consecutive_429s += 1;
                                    let backoff = Self::rate_limit_backoff(resp.headers(), consecutive_429s);
                                    tracing::warn!(
                                        "Twitter DM rate limited (attempt {}), backing off {}s",
                                        consecutive_429s,
                                        backoff.as_secs()
                                    );
                                    tokio::time::sleep(backoff).await;
                                } else {
                                    tracing::debug!("Twitter DM poll: {}", status_code);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Twitter DM poll error: {e}");
                            }
                        }
                    }
                }
            }
        });

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
        if credentials.credential_type != "oauth1" {
            return Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'oauth1'.",
                credentials.credential_type
            )));
        }

        let api_key = credentials
            .data
            .get("api_key")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'api_key'".into()))?
            .clone();
        let api_secret = credentials
            .data
            .get("api_secret")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'api_secret'".into()))?
            .clone();
        let access_token = credentials
            .data
            .get("access_token")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'access_token'".into()))?
            .clone();
        let access_token_secret = credentials
            .data
            .get("access_token_secret")
            .ok_or_else(|| ChannelError::AuthFailed("Missing 'access_token_secret'".into()))?
            .clone();

        let creds = TwitterCredentials {
            api_key,
            api_secret,
            access_token,
            access_token_secret,
        };

        // Verify credentials by fetching authenticated user info
        let url = format!("{}/users/me", X_API_BASE);
        let auth = Self::oauth_header(&creds, "GET", &url, &[]);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("HTTP error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::AuthFailed(format!(
                "Twitter auth failed ({}): {}",
                status, body
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::AuthFailed(format!("Invalid response: {e}")))?;

        let user_id = body["data"]["id"]
            .as_str()
            .ok_or_else(|| ChannelError::AuthFailed("Missing user ID in response".into()))?
            .to_string();

        let username = body["data"]["username"].as_str().unwrap_or("unknown");
        tracing::info!("Twitter authenticated as: @{} ({})", username, user_id);

        *self.user_id.lock().await = Some(user_id);
        *self.credentials.lock().await = Some(creds);

        Ok(LoginStatus::Success)
    }

    async fn logout(&mut self) -> Result<()> {
        self.disconnect().await?;
        *self.credentials.lock().await = None;
        *self.user_id.lock().await = None;
        Ok(())
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        let creds = self
            .credentials
            .lock()
            .await
            .clone()
            .ok_or_else(|| ChannelError::NotConnected("Not authenticated".into()))?;

        if recipient.starts_with("tweet:") || recipient == "tweet" {
            // Post a tweet
            self.post_tweet(&creds, &message.text).await
        } else {
            // Send a DM
            self.send_dm(&creds, recipient, &message.text).await
        }
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        // Use blocking lock to avoid spurious None from try_lock contention.
        // This is safe because the lock is never held across an await point
        // and is only taken briefly to move the Option out.
        self.incoming_rx.blocking_lock().take()
    }
}

impl TwitterChannel {
    /// Post a tweet using the X API v2.
    async fn post_tweet(&self, creds: &TwitterCredentials, text: &str) -> Result<()> {
        let url = format!("{}/tweets", X_API_BASE);

        // Chunk if needed (280 chars per tweet)
        let chunks = crate::common::chunk_message(text, TWEET_MAX_LEN);

        for chunk in chunks {
            let body = serde_json::json!({ "text": chunk });
            let auth = Self::oauth_header(creds, "POST", &url, &[]);

            let resp = self
                .client
                .post(&url)
                .header("Authorization", &auth)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("HTTP error: {e}")))?;

            if resp.status().as_u16() == 429 {
                let backoff = Self::rate_limit_backoff(resp.headers(), 1);
                tracing::warn!("Twitter tweet rate limited, backing off {}s", backoff.as_secs());
                tokio::time::sleep(backoff).await;
                return Err(ChannelError::SendFailed("Rate limited. Try again later.".into()));
            }

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed(format!(
                    "Tweet failed ({}): {}",
                    status, body
                )));
            }
        }

        Ok(())
    }

    /// Send a DM using the X API v2.
    async fn send_dm(
        &self,
        creds: &TwitterCredentials,
        recipient_id: &str,
        text: &str,
    ) -> Result<()> {
        let url = format!("{}/dm_conversations/with/{}/messages", X_API_BASE, recipient_id);

        let chunks = crate::common::chunk_message(text, DM_MAX_LEN);

        for chunk in chunks {
            let body = serde_json::json!({
                "text": chunk
            });
            let auth = Self::oauth_header(creds, "POST", &url, &[]);

            let resp = self
                .client
                .post(&url)
                .header("Authorization", &auth)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ChannelError::SendFailed(format!("HTTP error: {e}")))?;

            if resp.status().as_u16() == 429 {
                let backoff = Self::rate_limit_backoff(resp.headers(), 1);
                tracing::warn!("Twitter DM rate limited, backing off {}s", backoff.as_secs());
                tokio::time::sleep(backoff).await;
                return Err(ChannelError::SendFailed("Rate limited. Try again later.".into()));
            }

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(ChannelError::SendFailed(format!(
                    "DM failed ({}): {}",
                    status, body
                )));
            }
        }

        Ok(())
    }
}

pub struct TwitterChannelFactory;

impl crate::ChannelPluginFactory for TwitterChannelFactory {
    fn channel_type(&self) -> &str {
        "twitter"
    }
    fn channel_type_name(&self) -> &str {
        "Twitter/X"
    }
    fn features(&self) -> crate::ChannelFeatures {
        crate::ChannelFeatures {
            direct_messages: true,
            group_messages: false,
            media_attachments: true,
            reactions: false,
            read_receipts: false,
            typing_indicators: false,
            threads: false,
        }
    }
    fn create_instance(&self, _instance_id: &str) -> Box<dyn crate::ChannelPlugin> {
        Box::new(TwitterChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChannelPluginFactory;
    use std::collections::HashMap;

    #[test]
    fn test_twitter_channel_metadata() {
        let channel = TwitterChannel::new();
        assert_eq!(channel.id(), "twitter");
        assert_eq!(channel.name(), "Twitter/X");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_twitter_features() {
        let channel = TwitterChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(!features.group_messages);
        assert!(features.media_attachments);
        assert!(!features.reactions);
    }

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello"), "hello");
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(percent_encode("test~value"), "test~value");
    }

    #[test]
    fn test_factory() {
        let factory = TwitterChannelFactory;
        assert_eq!(factory.channel_type(), "twitter");
        assert_eq!(factory.channel_type_name(), "Twitter/X");
        let instance = factory.create_instance("brand-a");
        assert_eq!(instance.id(), "twitter");
        assert_eq!(instance.status(), ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_login_bad_type() {
        let mut channel = TwitterChannel::new();
        let creds = ChannelCredentials {
            credential_type: "bearer".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("oauth1"));
    }

    #[tokio::test]
    async fn test_login_missing_fields() {
        let mut channel = TwitterChannel::new();
        let creds = ChannelCredentials {
            credential_type: "oauth1".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("api_key"));
    }

    #[tokio::test]
    async fn test_send_not_connected() {
        let channel = TwitterChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("tweet", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not authenticated"));
    }

    #[test]
    fn test_oauth_header_includes_params_in_signature() {
        let creds = TwitterCredentials {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            access_token: "test_token".to_string(),
            access_token_secret: "test_token_secret".to_string(),
        };

        // With no params
        let header1 = TwitterChannel::oauth_header(&creds, "GET", "https://api.x.com/2/users/me", &[]);
        assert!(header1.starts_with("OAuth "));
        assert!(header1.contains("oauth_consumer_key"));
        assert!(header1.contains("oauth_signature"));

        // With query params -- signature should differ
        let header2 = TwitterChannel::oauth_header(
            &creds,
            "GET",
            "https://api.x.com/2/users/123/mentions",
            &[("since_id", "999"), ("tweet.fields", "created_at,author_id")],
        );
        assert!(header2.starts_with("OAuth "));
        // Signatures must be different since params differ
        let sig1 = header1.split("oauth_signature=").nth(1).unwrap_or("");
        let sig2 = header2.split("oauth_signature=").nth(1).unwrap_or("");
        assert_ne!(sig1, sig2, "Signatures should differ when query params are included");
    }

    #[test]
    fn test_build_query_url() {
        let url = TwitterChannel::build_query_url(
            "https://api.x.com/2/users/123/mentions",
            &[("since_id", "999"), ("tweet.fields", "created_at")],
        );
        assert_eq!(url, "https://api.x.com/2/users/123/mentions?since_id=999&tweet.fields=created_at");

        let url_empty = TwitterChannel::build_query_url("https://api.x.com/2/tweets", &[]);
        assert_eq!(url_empty, "https://api.x.com/2/tweets");
    }

    #[test]
    fn test_rate_limit_backoff_exponential() {
        let headers = reqwest::header::HeaderMap::new();

        // Exponential: 5, 10, 20, 40, 80, 160, 300 (capped)
        assert_eq!(TwitterChannel::rate_limit_backoff(&headers, 0).as_secs(), 5);
        assert_eq!(TwitterChannel::rate_limit_backoff(&headers, 1).as_secs(), 10);
        assert_eq!(TwitterChannel::rate_limit_backoff(&headers, 2).as_secs(), 20);
        assert_eq!(TwitterChannel::rate_limit_backoff(&headers, 3).as_secs(), 40);
        assert_eq!(TwitterChannel::rate_limit_backoff(&headers, 6).as_secs(), 300); // capped
        assert_eq!(TwitterChannel::rate_limit_backoff(&headers, 10).as_secs(), 300); // still capped
    }

    #[test]
    fn test_rate_limit_backoff_from_header() {
        let mut headers = reqwest::header::HeaderMap::new();
        let future_ts = chrono::Utc::now().timestamp() + 45; // 45 seconds from now
        headers.insert("x-ratelimit-reset", future_ts.to_string().parse().unwrap());

        let backoff = TwitterChannel::rate_limit_backoff(&headers, 0);
        // Should be approximately 45 seconds (±1 for timing)
        assert!(backoff.as_secs() >= 44 && backoff.as_secs() <= 46);
    }
}
