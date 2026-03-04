//! LSP client -- manages a single language server connection.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

use super::protocol::LspRequest;
use crate::error::{LlmError, Result};

/// An active language server connection.
pub struct LspClient {
    child: Child,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    next_id: AtomicU64,
    opened_files: Mutex<HashSet<String>>,
    pub language: String,
    pub root_path: String,
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl LspClient {
    pub async fn start(command: &str, args: &[&str], root_path: &str, language: &str) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| LlmError::ToolCall(format!("Failed to spawn language server '{command}': {e}")))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // Drain stderr
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(target: "lsp_stderr", "{}", line);
                }
            });
        }

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let stdin = Arc::new(Mutex::new(Some(stdin)));

        // Background reader for LSP responses (Content-Length header protocol)
        let pending_clone = Arc::clone(&pending);
        let reader_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut header_buf = String::new();

            loop {
                header_buf.clear();
                // Read headers until empty line
                let mut content_length: usize = 0;
                loop {
                    header_buf.clear();
                    match reader.read_line(&mut header_buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(_) => {}
                    }
                    let trimmed = header_buf.trim();
                    if trimmed.is_empty() {
                        break;
                    }
                    if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
                        content_length = len_str.parse().unwrap_or(0);
                    }
                }

                if content_length == 0 {
                    continue;
                }

                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).await.is_err() {
                    return;
                }

                if let Ok(msg) = serde_json::from_slice::<Value>(&body) {
                    if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                        // Response to a request
                        let mut map = pending_clone.lock().await;
                        if let Some(tx) = map.remove(&id) {
                            let _ = tx.send(msg);
                        }
                    }
                    // Notifications (no id) are logged
                }
            }
        });

        let mut client = Self {
            child,
            stdin,
            pending,
            next_id: AtomicU64::new(1),
            opened_files: Mutex::new(HashSet::new()),
            language: language.to_string(),
            root_path: root_path.to_string(),
            _reader_handle: reader_handle,
        };

        client.initialize(root_path).await?;
        Ok(client)
    }

    async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let req = LspRequest::new(id, method, params);
        let body = serde_json::to_string(&req)
            .map_err(|e| LlmError::ToolCall(format!("Serialize error: {e}")))?;
        let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending.lock().await;
            map.insert(id, tx);
        }

        {
            let mut guard = self.stdin.lock().await;
            if let Some(ref mut stdin) = *guard {
                stdin.write_all(message.as_bytes()).await
                    .map_err(|e| LlmError::ToolCall(format!("Write error: {e}")))?;
                stdin.flush().await.ok();
            }
        }

        let resp = tokio::time::timeout(std::time::Duration::from_secs(15), rx)
            .await
            .map_err(|_| LlmError::ToolCall(format!("LSP '{method}' timed out")))?
            .map_err(|_| LlmError::ToolCall(format!("LSP channel closed for '{method}'")))?;

        if let Some(err) = resp.get("error") {
            let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            return Err(LlmError::ToolCall(format!("LSP error: {msg}")));
        }

        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let notif = super::protocol::LspNotification::new(method, params);
        let body = serde_json::to_string(&notif)
            .map_err(|e| LlmError::ToolCall(format!("Serialize error: {e}")))?;
        let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        let mut guard = self.stdin.lock().await;
        if let Some(ref mut stdin) = *guard {
            stdin.write_all(message.as_bytes()).await.ok();
            stdin.flush().await.ok();
        }
        Ok(())
    }

    async fn initialize(&mut self, root_path: &str) -> Result<()> {
        let root_uri = format!("file:///{}", root_path.replace('\\', "/").trim_start_matches('/'));

        let _result = self.send_request("initialize", json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "definition": { "dynamicRegistration": false },
                    "references": { "dynamicRegistration": false },
                    "hover": { "dynamicRegistration": false },
                    "documentSymbol": { "dynamicRegistration": false },
                    "publishDiagnostics": { "relatedInformation": true },
                    "rename": { "dynamicRegistration": false, "prepareSupport": true },
                },
                "workspace": {
                    "symbol": { "dynamicRegistration": false },
                }
            }
        })).await?;

        self.send_notification("initialized", json!({})).await?;

        tracing::info!("LSP server initialized for {} at {}", self.language, root_path);
        Ok(())
    }

    /// Ensure a file is opened in the language server.
    pub async fn ensure_open(&self, file_path: &str) -> Result<()> {
        let mut opened = self.opened_files.lock().await;
        if opened.contains(file_path) {
            return Ok(());
        }

        let uri = path_to_uri(file_path);
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to read {file_path}: {e}")))?;

        let language_id = match self.language.as_str() {
            "rust" => "rust",
            "typescript" => "typescript",
            "javascript" => "javascript",
            "python" => "python",
            "go" => "go",
            _ => "plaintext",
        };

        self.send_notification("textDocument/didOpen", json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": content
            }
        })).await?;

        opened.insert(file_path.to_string());
        Ok(())
    }

    pub async fn goto_definition(&self, file: &str, line: u32, column: u32) -> Result<Value> {
        self.ensure_open(file).await?;
        let uri = path_to_uri(file);
        self.send_request("textDocument/definition", json!({
            "textDocument": { "uri": uri },
            "position": { "line": line.saturating_sub(1), "character": column.saturating_sub(1) }
        })).await
    }

    pub async fn find_references(&self, file: &str, line: u32, column: u32) -> Result<Value> {
        self.ensure_open(file).await?;
        let uri = path_to_uri(file);
        self.send_request("textDocument/references", json!({
            "textDocument": { "uri": uri },
            "position": { "line": line.saturating_sub(1), "character": column.saturating_sub(1) },
            "context": { "includeDeclaration": true }
        })).await
    }

    pub async fn hover(&self, file: &str, line: u32, column: u32) -> Result<Value> {
        self.ensure_open(file).await?;
        let uri = path_to_uri(file);
        self.send_request("textDocument/hover", json!({
            "textDocument": { "uri": uri },
            "position": { "line": line.saturating_sub(1), "character": column.saturating_sub(1) }
        })).await
    }

    pub async fn document_symbols(&self, file: &str) -> Result<Value> {
        self.ensure_open(file).await?;
        let uri = path_to_uri(file);
        self.send_request("textDocument/documentSymbol", json!({
            "textDocument": { "uri": uri }
        })).await
    }

    pub async fn workspace_symbols(&self, query: &str) -> Result<Value> {
        self.send_request("workspace/symbol", json!({
            "query": query
        })).await
    }

    pub async fn rename_preview(&self, file: &str, line: u32, column: u32, new_name: &str) -> Result<Value> {
        self.ensure_open(file).await?;
        let uri = path_to_uri(file);
        self.send_request("textDocument/rename", json!({
            "textDocument": { "uri": uri },
            "position": { "line": line.saturating_sub(1), "character": column.saturating_sub(1) },
            "newName": new_name
        })).await
    }

    pub async fn shutdown(&mut self) {
        let _ = self.send_request("shutdown", Value::Null).await;
        let _ = self.send_notification("exit", Value::Null).await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let _ = self.child.kill().await;
    }
}

fn path_to_uri(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') {
        format!("file://{}", normalized)
    } else {
        format!("file:///{}", normalized)
    }
}
