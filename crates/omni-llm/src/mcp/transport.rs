//! MCP transport layer -- stdio and SSE.
//!
//! Both transports implement the same async interface for sending
//! JSON-RPC requests and receiving responses.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::error::{LlmError, Result};

// ---------------------------------------------------------------------------
// Stdio Transport
// ---------------------------------------------------------------------------

/// Manages a JSON-RPC connection over stdin/stdout of a child process.
pub struct StdioTransport {
    child: Child,
    stdin_writer: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    next_id: AtomicU64,
    /// Background reader task handle
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl StdioTransport {
    /// Spawn the child process and set up JSON-RPC communication.
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        working_dir: Option<&str>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| LlmError::ToolCall(format!("Failed to spawn MCP server '{command}': {e}")))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // Drain stderr in background
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(target: "mcp_stderr", "{}", line);
                }
            });
        }

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let stdin_writer = Arc::new(Mutex::new(Some(stdin)));

        // Background stdout reader -- routes responses to pending requests
        let pending_clone = Arc::clone(&pending);
        let reader_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                match serde_json::from_str::<JsonRpcResponse>(&line) {
                    Ok(resp) => {
                        if let Some(id) = resp.id {
                            let mut map = pending_clone.lock().await;
                            if let Some(tx) = map.remove(&id) {
                                let _ = tx.send(resp);
                            }
                        }
                        // Notifications (no id) are logged but not routed
                    }
                    Err(e) => {
                        tracing::debug!(target: "mcp_transport", "Non-JSON line from MCP server: {e}");
                    }
                }
            }
        });

        Ok(Self {
            child,
            stdin_writer,
            pending,
            next_id: AtomicU64::new(1),
            _reader_handle: reader_handle,
        })
    }

    /// Send a JSON-RPC request and wait for the response.
    pub async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let req = JsonRpcRequest::new(id, method, params);
        let mut payload = serde_json::to_string(&req)
            .map_err(|e| LlmError::ToolCall(format!("Failed to serialize MCP request: {e}")))?;
        payload.push('\n');

        // Set up response channel
        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending.lock().await;
            map.insert(id, tx);
        }

        // Send request
        {
            let mut guard = self.stdin_writer.lock().await;
            if let Some(ref mut stdin) = *guard {
                stdin
                    .write_all(payload.as_bytes())
                    .await
                    .map_err(|e| LlmError::ToolCall(format!("Failed to write to MCP server: {e}")))?;
                stdin
                    .flush()
                    .await
                    .map_err(|e| LlmError::ToolCall(format!("Failed to flush MCP server stdin: {e}")))?;
            } else {
                return Err(LlmError::ToolCall("MCP server stdin closed".to_string()));
            }
        }

        // Wait for response with timeout (30s)
        let resp = tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| LlmError::ToolCall(format!("MCP request '{method}' timed out (30s)")))?
            .map_err(|_| LlmError::ToolCall(format!("MCP response channel closed for '{method}'")))?;

        // Check for error
        if let Some(err) = resp.error {
            return Err(LlmError::ToolCall(format!("{err}")));
        }

        Ok(resp.result.unwrap_or(Value::Null))
    }

    /// Send a notification (no response expected).
    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        // Notifications use id: null (omitted in our struct, so we construct manually)
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(Value::Null)
        });
        let mut line = serde_json::to_string(&payload)
            .map_err(|e| LlmError::ToolCall(format!("Failed to serialize notification: {e}")))?;
        line.push('\n');

        let mut guard = self.stdin_writer.lock().await;
        if let Some(ref mut stdin) = *guard {
            stdin
                .write_all(line.as_bytes())
                .await
                .map_err(|e| LlmError::ToolCall(format!("Failed to write notification: {e}")))?;
            stdin.flush().await.ok();
        }
        Ok(())
    }

    /// Check if the child process is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill the child process.
    pub async fn shutdown(&mut self) {
        // Close stdin first to signal EOF
        {
            let mut guard = self.stdin_writer.lock().await;
            *guard = None;
        }
        // Give a moment for graceful shutdown
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let _ = self.child.kill().await;
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // Best-effort kill on drop
        let _ = self.child.start_kill();
    }
}
