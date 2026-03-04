//! WhatsApp Web Channel Plugin (Baileys Sidecar)
//!
//! Implements the ChannelPlugin trait for WhatsApp using the Baileys library
//! running as a Node.js sidecar process. Free -- no API costs, uses QR code
//! pairing like WhatsApp Web.
//!
//! ## Requirements
//! - Node.js installed on the system
//! - `sidecar/whatsapp/` directory with `bridge.js` and `node_modules/`
//!
//! ## Setup
//! 1. Run `npm install` in the `sidecar/whatsapp/` directory
//! 2. Connect via Omni -- a QR code will be displayed
//! 3. Scan the QR code with your WhatsApp app
//! 4. Session credentials are saved locally for auto-reconnection
//!
//! ## Authentication
//! - `credential_type`: "qr_code" (no data needed -- QR is generated on connect)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex, oneshot};

use omni_core::events::{EventBus, OmniEvent};

use crate::{
    ChannelConfig, ChannelCredentials, ChannelError, ChannelFeatures, ChannelPlugin,
    ConnectionStatus, IncomingMessage, LoginStatus, OutgoingMessage, Result,
};

/// WhatsApp Web channel plugin using Baileys sidecar.
pub struct WhatsAppWebChannel {
    /// Connection status.
    status: Arc<AtomicU8>,
    /// Child process handle.
    child: Mutex<Option<Child>>,
    /// Stdin writer for sending commands.
    stdin_writer: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    /// Sender for incoming messages.
    incoming_tx: mpsc::Sender<IncomingMessage>,
    /// Receiver handed to ChannelManager once.
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    /// Auth directory for session persistence.
    auth_dir: Mutex<PathBuf>,
    /// Sidecar script path.
    sidecar_dir: Mutex<PathBuf>,
    /// Next request ID for JSON-RPC.
    next_request_id: AtomicU64,
    /// Pending request response channels.
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
    /// Last QR code data received.
    last_qr: Arc<Mutex<Option<String>>>,
    /// Event bus for pushing QR code events to the frontend.
    event_bus: Option<EventBus>,
    /// Channel ID for event emission (set via set_event_bus).
    channel_id_for_events: String,
}

impl WhatsAppWebChannel {
    pub fn new() -> Self {
        let (incoming_tx, incoming_rx) = mpsc::channel(128);

        // Resolve sidecar path -- probe multiple candidates for dev & production
        let default_sidecar = Self::find_sidecar_dir();

        // Default auth dir
        let default_auth = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".omni")
            .join("channels")
            .join("whatsapp");

        Self {
            status: Arc::new(AtomicU8::new(ConnectionStatus::Disconnected as u8)),
            child: Mutex::new(None),
            stdin_writer: Arc::new(Mutex::new(None)),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            auth_dir: Mutex::new(default_auth),
            sidecar_dir: Mutex::new(default_sidecar),
            next_request_id: AtomicU64::new(1),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            last_qr: Arc::new(Mutex::new(None)),
            event_bus: None,
            channel_id_for_events: String::new(),
        }
    }

    /// Find the sidecar directory by probing multiple candidate paths.
    /// Works in both dev (`tauri dev` from ui/) and production (exe-relative).
    fn find_sidecar_dir() -> PathBuf {
        let mut candidates: Vec<PathBuf> = Vec::new();

        // 1. Relative to executable (production layout)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                candidates.push(parent.join("sidecar").join("whatsapp"));
                // Walk up from exe -- covers ui/src-tauri/target/debug/ → project root
                let mut dir = parent;
                for _ in 0..6 {
                    let candidate = dir.join("sidecar").join("whatsapp");
                    candidates.push(candidate);
                    match dir.parent() {
                        Some(p) => dir = p,
                        None => break,
                    }
                }
            }
        }

        // 2. Relative to CWD (for dev workflows)
        candidates.push(PathBuf::from("sidecar/whatsapp"));
        candidates.push(PathBuf::from("../../sidecar/whatsapp"));

        // Return first candidate that has bridge.js
        for c in &candidates {
            if c.join("bridge.js").exists() {
                return c.clone();
            }
        }

        // Fallback -- will fail at connect() with a clear error message
        PathBuf::from("sidecar/whatsapp")
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

    /// Send a JSON-RPC command to the sidecar and wait for response.
    async fn send_command(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);

        let msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });

        let (tx, rx) = oneshot::channel();
        self.pending_requests.lock().await.insert(id, tx);

        // Write command to stdin
        let mut stdin = self.stdin_writer.lock().await;
        let writer = stdin
            .as_mut()
            .ok_or_else(|| ChannelError::NotConnected("Sidecar not running".into()))?;

        let line = format!("{}\n", serde_json::to_string(&msg).unwrap());
        writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| ChannelError::Other(format!("Failed to write to sidecar: {e}")))?;
        writer.flush().await.ok();

        // Wait for response (with timeout)
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => {
                if let Some(error) = response.get("error").and_then(|e| e.as_str()) {
                    Err(ChannelError::Other(error.to_string()))
                } else {
                    Ok(response)
                }
            }
            Ok(Err(_)) => Err(ChannelError::Other("Response channel closed".into())),
            Err(_) => {
                self.pending_requests.lock().await.remove(&id);
                Err(ChannelError::Other("Command timed out (30s)".into()))
            }
        }
    }

    /// Find the Node.js executable.
    fn find_node() -> Result<String> {
        // Check common locations
        for cmd in &["node", "node.exe"] {
            if which_exists(cmd) {
                return Ok(cmd.to_string());
            }
        }
        Err(ChannelError::Config(
            "Node.js not found. Install Node.js to use WhatsApp Web channel.".into(),
        ))
    }
}

/// Check if a command exists on PATH.
fn which_exists(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

#[async_trait::async_trait]
impl ChannelPlugin for WhatsAppWebChannel {
    fn id(&self) -> &str {
        "whatsapp-web"
    }

    fn name(&self) -> &str {
        "WhatsApp Web"
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
        self.get_status()
    }

    async fn connect(&mut self, config: ChannelConfig) -> Result<()> {
        // Allow overriding sidecar and auth dir via config
        if let Some(sidecar) = config.settings.get("sidecar_dir").and_then(|v| v.as_str()) {
            *self.sidecar_dir.lock().await = PathBuf::from(sidecar);
        }
        if let Some(auth) = config.settings.get("auth_dir").and_then(|v| v.as_str()) {
            *self.auth_dir.lock().await = PathBuf::from(auth);
        }

        let node = Self::find_node()?;
        let sidecar_dir = self.sidecar_dir.lock().await.clone();
        let auth_dir = self.auth_dir.lock().await.clone();

        let bridge_path = sidecar_dir.join("bridge.js");
        if !bridge_path.exists() {
            return Err(ChannelError::Config(format!(
                "Sidecar script not found at: {}",
                bridge_path.display()
            )));
        }

        // Check if node_modules exists; if not, run npm install
        let node_modules = sidecar_dir.join("node_modules");
        if !node_modules.exists() {
            tracing::info!("Installing WhatsApp sidecar dependencies...");
            let npm_status = std::process::Command::new("npm")
                .args(["install", "--production"])
                .current_dir(&sidecar_dir)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()
                .map_err(|e| {
                    ChannelError::Config(format!(
                        "Failed to run npm install: {e}. Install Node.js and npm."
                    ))
                })?;

            if !npm_status.success() {
                return Err(ChannelError::Config(
                    "npm install failed in sidecar/whatsapp/".into(),
                ));
            }
        }

        self.set_status(ConnectionStatus::Connecting);

        // Spawn the sidecar process
        let mut child = Command::new(&node)
            .arg(&bridge_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .current_dir(&sidecar_dir)
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ChannelError::Other(format!("Failed to spawn sidecar: {e}")))?;

        // Take ownership of stdin/stdout
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ChannelError::Other("Failed to capture sidecar stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ChannelError::Other("Failed to capture sidecar stdout".into()))?;

        *self.stdin_writer.lock().await = Some(stdin);
        *self.child.lock().await = Some(child);

        // Spawn stdout reader task
        let incoming_tx = self.incoming_tx.clone();
        let pending = self.pending_requests.clone();
        let status = self.status.clone();
        let last_qr = self.last_qr.clone();
        let event_bus = self.event_bus.clone();
        let channel_id_for_events = self.channel_id_for_events.clone();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // Check if this is a response to a request (has "id" field)
                if let Some(id) = parsed.get("id").and_then(|v| v.as_u64()) {
                    let mut pending = pending.lock().await;
                    if let Some(tx) = pending.remove(&id) {
                        let _ = tx.send(parsed);
                    }
                    continue;
                }

                // Otherwise it's an event
                let event = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("");
                let data = parsed.get("data").cloned().unwrap_or(serde_json::Value::Null);

                match event {
                    "ready" => {
                        tracing::info!("WhatsApp sidecar ready");
                    }
                    "qr" => {
                        if let Some(qr_data) = data.get("qrImage").and_then(|v| v.as_str()) {
                            tracing::info!("WhatsApp QR code received -- scan with your phone");
                            *last_qr.lock().await = Some(qr_data.to_string());
                            if let Some(ref bus) = event_bus {
                                bus.emit(OmniEvent::ChannelQrCode {
                                    channel_id: channel_id_for_events.clone(),
                                    qr_data: qr_data.to_string(),
                                });
                            }
                        }
                    }
                    "connected" => {
                        let phone = data.get("phone").and_then(|v| v.as_str()).unwrap_or("");
                        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        tracing::info!("WhatsApp connected as: {} ({})", name, phone);
                        status.store(ConnectionStatus::Connected as u8, Ordering::Relaxed);
                        if let Some(ref bus) = event_bus {
                            bus.emit(OmniEvent::ChannelConnected {
                                channel_id: channel_id_for_events.clone(),
                            });
                        }
                    }
                    "reconnecting" => {
                        // Sidecar is auto-recovering from stale auth -- clearing
                        // auth dir and reconnecting to generate a fresh QR code.
                        tracing::info!("WhatsApp auto-recovering from stale auth -- waiting for fresh QR");
                        status.store(ConnectionStatus::Connecting as u8, Ordering::Relaxed);
                    }
                    "disconnected" => {
                        let logged_out = data.get("loggedOut").and_then(|v| v.as_bool()).unwrap_or(false);
                        if logged_out {
                            tracing::warn!("WhatsApp logged out -- re-scan QR required");
                        } else {
                            tracing::warn!("WhatsApp disconnected");
                        }
                        status.store(ConnectionStatus::Disconnected as u8, Ordering::Relaxed);
                        if let Some(ref bus) = event_bus {
                            bus.emit(OmniEvent::ChannelDisconnected {
                                channel_id: channel_id_for_events.clone(),
                            });
                        }
                    }
                    "message" => {
                        let from = data.get("from").and_then(|v| v.as_str()).unwrap_or("");
                        let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let msg_id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let is_group = data.get("isGroup").and_then(|v| v.as_bool()).unwrap_or(false);
                        let group_id = data.get("groupId").and_then(|v| v.as_str()).map(String::from);
                        let push_name = data.get("pushName").and_then(|v| v.as_str()).map(String::from);
                        let timestamp = data
                            .get("timestamp")
                            .and_then(|v| v.as_i64())
                            .and_then(|ts| chrono::DateTime::from_timestamp(ts / 1000, 0))
                            .unwrap_or_else(chrono::Utc::now);

                        let incoming = IncomingMessage {
                            id: msg_id.to_string(),
                            channel_id: "whatsapp-web".to_string(),
                            channel_type: "whatsapp-web".to_string(),
                            instance_id: "default".to_string(),
                            sender: from.to_string(),
                            sender_name: push_name,
                            text: text.to_string(),
                            is_group,
                            group_id,
                            thread_id: None,
                            timestamp,
                            media_url: data.get("mediaUrl").and_then(|v| v.as_str()).map(String::from),
                            source_trust_level: crate::SourceTrustLevel::Authenticated,
                        };

                        if let Err(e) = incoming_tx.send(incoming).await {
                            tracing::warn!("Failed to forward WhatsApp message: {e}");
                        }
                    }
                    "error" => {
                        let error = data.get("message").and_then(|v| v.as_str()).unwrap_or("unknown");
                        // Non-fatal errors (Baileys internal timeouts, sync issues) should
                        // NOT change connection status. Only "disconnected" events change status.
                        tracing::warn!("WhatsApp sidecar error (non-fatal): {error}");
                        if let Some(ref bus) = event_bus {
                            bus.emit(OmniEvent::ChannelError {
                                channel_id: channel_id_for_events.clone(),
                                error: error.to_string(),
                            });
                        }
                    }
                    _ => {
                        tracing::debug!("Unknown WhatsApp event: {event}");
                    }
                }
            }

            // Stdout closed -- process exited
            tracing::warn!("WhatsApp sidecar process exited");
            status.store(ConnectionStatus::Disconnected as u8, Ordering::Relaxed);
            if let Some(ref bus) = event_bus {
                bus.emit(OmniEvent::ChannelDisconnected {
                    channel_id: channel_id_for_events.clone(),
                });
            }
        });

        // Send connect command to sidecar
        let auth_str = auth_dir.display().to_string();
        self.send_command("connect", serde_json::json!({ "authDir": auth_str }))
            .await?;

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // Send logout to sidecar -- unlinks device from WhatsApp + clears auth files
        let _ = self.send_command("logout", serde_json::Value::Null).await;

        // Kill the child process
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
        }

        *self.stdin_writer.lock().await = None;

        // Clear local auth directory so next connect requires fresh QR scan
        let auth_dir = self.auth_dir.lock().await.clone();
        if auth_dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&auth_dir) {
                tracing::warn!("Failed to clear WhatsApp auth dir: {e}");
            }
        }

        // Clear cached QR
        *self.last_qr.lock().await = None;

        self.set_status(ConnectionStatus::Disconnected);
        Ok(())
    }

    async fn login(&mut self, credentials: ChannelCredentials) -> Result<LoginStatus> {
        match credentials.credential_type.as_str() {
            "qr_code" => {
                // QR code login -- the QR is generated during connect()
                // Return PendingApproval to indicate the user needs to scan
                if let Some(auth) = credentials.data.get("auth_dir") {
                    *self.auth_dir.lock().await = PathBuf::from(auth);
                }
                if let Some(sidecar) = credentials.data.get("sidecar_dir") {
                    *self.sidecar_dir.lock().await = PathBuf::from(sidecar);
                }

                Ok(LoginStatus::PendingApproval {
                    qr_code_data: self.last_qr.lock().await.clone(),
                })
            }
            _ => Err(ChannelError::AuthFailed(format!(
                "Unsupported credential type '{}'. Use 'qr_code'.",
                credentials.credential_type
            ))),
        }
    }

    async fn logout(&mut self) -> Result<()> {
        // disconnect() already sends "logout" to sidecar and clears auth dir
        self.disconnect().await
    }

    async fn send_message(&self, recipient: &str, message: OutgoingMessage) -> Result<()> {
        let result = self
            .send_command(
                "send",
                serde_json::json!({
                    "to": recipient,
                    "text": message.text,
                }),
            )
            .await?;

        if result.get("error").is_some() {
            return Err(ChannelError::SendFailed(
                result["error"].as_str().unwrap_or("unknown").to_string(),
            ));
        }

        Ok(())
    }

    fn incoming_messages(&self) -> Option<mpsc::Receiver<IncomingMessage>> {
        self.incoming_rx.try_lock().ok()?.take()
    }

    fn set_event_bus(&mut self, event_bus: EventBus, channel_id: String) {
        self.event_bus = Some(event_bus);
        self.channel_id_for_events = channel_id;
    }
}


pub struct WhatsAppWebChannelFactory;

impl crate::ChannelPluginFactory for WhatsAppWebChannelFactory {
    fn channel_type(&self) -> &str { "whatsapp-web" }
    fn channel_type_name(&self) -> &str { "WhatsApp Web" }
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
        Box::new(WhatsAppWebChannel::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whatsapp_web_metadata() {
        let channel = WhatsAppWebChannel::new();
        assert_eq!(channel.id(), "whatsapp-web");
        assert_eq!(channel.name(), "WhatsApp Web");
        assert_eq!(channel.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_whatsapp_web_features() {
        let channel = WhatsAppWebChannel::new();
        let features = channel.features();
        assert!(features.direct_messages);
        assert!(features.group_messages);
        assert!(features.media_attachments);
        assert!(features.reactions);
        assert!(features.read_receipts);
        assert!(features.typing_indicators);
    }

    #[tokio::test]
    async fn test_whatsapp_web_login_bad_type() {
        let mut channel = WhatsAppWebChannel::new();
        let creds = ChannelCredentials {
            credential_type: "api_key".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_whatsapp_web_login_qr_code() {
        let mut channel = WhatsAppWebChannel::new();
        let creds = ChannelCredentials {
            credential_type: "qr_code".to_string(),
            data: HashMap::new(),
        };
        let result = channel.login(creds).await;
        assert!(result.is_ok());
        match result.unwrap() {
            LoginStatus::PendingApproval { .. } => {} // Expected
            other => panic!("Expected PendingApproval, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_whatsapp_web_send_not_connected() {
        let channel = WhatsAppWebChannel::new();
        let msg = OutgoingMessage {
            text: "Hello".to_string(),
            media_url: None,
            reply_to: None,
            thread_id: None,
        };
        let result = channel.send_message("+1234567890", msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not running"));
    }

    #[test]
    fn test_which_exists() {
        // "node" may or may not exist, but the function shouldn't panic
        let _ = which_exists("node");
        // A command that definitely doesn't exist
        assert!(!which_exists("definitelynotacommand12345"));
    }
}
