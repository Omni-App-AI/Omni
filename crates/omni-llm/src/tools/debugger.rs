//! Debug Adapter Protocol (DAP) client tool.
//!
//! Controls debug sessions via the standardized DAP protocol.
//! Supports launching programs, breakpoints, stepping, and variable inspection.
//!
//! Gated by `debug.session` permission.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

use super::NativeTool;
use crate::error::{LlmError, Result};

/// A DAP debug session.
struct DebugSession {
    child: Child,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    next_seq: AtomicU64,
    initialized: bool,
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl DebugSession {
    async fn spawn(command: &str, args: &[&str]) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| LlmError::ToolCall(format!("Failed to spawn debug adapter '{command}': {e}")))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // Drain stderr
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(target: "dap_stderr", "{}", line);
                }
            });
        }

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let stdin = Arc::new(Mutex::new(Some(stdin)));

        // DAP uses a simple header-body protocol:
        // Content-Length: N\r\n\r\n{json body}
        let pending_clone = Arc::clone(&pending);
        let reader_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                // Read Content-Length header
                let mut header = String::new();
                if reader.read_line(&mut header).await.is_err() || header.is_empty() {
                    break;
                }
                let content_length: usize = header
                    .trim()
                    .strip_prefix("Content-Length: ")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                if content_length == 0 {
                    continue;
                }

                // Read empty line separator
                let mut sep = String::new();
                let _ = reader.read_line(&mut sep).await;

                // Read body
                let mut body = vec![0u8; content_length];
                if tokio::io::AsyncReadExt::read_exact(&mut reader, &mut body)
                    .await
                    .is_err()
                {
                    break;
                }

                if let Ok(msg) = serde_json::from_slice::<Value>(&body) {
                    if let Some(request_seq) = msg.get("request_seq").and_then(|v| v.as_u64()) {
                        let mut map = pending_clone.lock().await;
                        if let Some(tx) = map.remove(&request_seq) {
                            let _ = tx.send(msg);
                        }
                    }
                }
            }
        });

        Ok(Self {
            child,
            stdin,
            pending,
            next_seq: AtomicU64::new(1),
            initialized: false,
            _reader_handle: reader_handle,
        })
    }

    async fn send_request(&self, command: &str, arguments: Option<Value>) -> Result<Value> {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);

        let request = json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": arguments.unwrap_or(Value::Null)
        });

        let body = serde_json::to_string(&request)
            .map_err(|e| LlmError::ToolCall(format!("Failed to serialize DAP request: {e}")))?;

        let message = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        // Set up response channel
        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending.lock().await;
            map.insert(seq, tx);
        }

        // Send
        {
            let mut guard = self.stdin.lock().await;
            if let Some(ref mut stdin) = *guard {
                stdin
                    .write_all(message.as_bytes())
                    .await
                    .map_err(|e| LlmError::ToolCall(format!("Failed to write to debug adapter: {e}")))?;
                stdin.flush().await.ok();
            }
        }

        // Wait for response
        let resp = tokio::time::timeout(std::time::Duration::from_secs(10), rx)
            .await
            .map_err(|_| LlmError::ToolCall(format!("DAP '{command}' timed out")))?
            .map_err(|_| LlmError::ToolCall(format!("DAP response channel closed for '{command}'")))?;

        // Check for error
        if resp.get("success") == Some(&Value::Bool(false)) {
            let msg = resp
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(LlmError::ToolCall(format!("DAP error: {msg}")));
        }

        Ok(resp.get("body").cloned().unwrap_or(Value::Null))
    }

    async fn shutdown(&mut self) {
        let _ = self.send_request("disconnect", Some(json!({"terminateDebuggee": true}))).await;
        let _ = self.child.kill().await;
    }
}

pub struct DebuggerTool {
    session: Arc<Mutex<Option<DebugSession>>>,
}

impl DebuggerTool {
    pub fn new() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
        }
    }

    fn detect_adapter(program: &str) -> Result<(String, Vec<String>)> {
        let ext = std::path::Path::new(program)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "py" => Ok(("python".to_string(), vec![
                "-m".to_string(),
                "debugpy.adapter".to_string(),
            ])),
            "js" | "ts" | "mjs" => Ok(("node".to_string(), vec![
                "--inspect-brk".to_string(),
            ])),
            _ => {
                // Try codelldb for compiled languages
                if which_exe("codelldb").is_some() {
                    Ok(("codelldb".to_string(), vec!["--port".to_string(), "0".to_string()]))
                } else if which_exe("lldb-vscode").is_some() {
                    Ok(("lldb-vscode".to_string(), Vec::new()))
                } else {
                    Err(LlmError::ToolCall(format!(
                        "No debug adapter found for '{}'. Install codelldb, debugpy, or use Node --inspect.",
                        program
                    )))
                }
            }
        }
    }

    async fn action_launch(&self, params: &Value) -> Result<Value> {
        let program = params
            .get("program")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'program' is required".to_string()))?;

        let mut guard = self.session.lock().await;
        if guard.is_some() {
            return Err(LlmError::ToolCall(
                "Debug session already active. Use 'disconnect' first.".to_string(),
            ));
        }

        let (adapter_cmd, adapter_args) = Self::detect_adapter(program)?;
        let args: Vec<&str> = adapter_args.iter().map(|s| s.as_str()).collect();

        let mut session = DebugSession::spawn(&adapter_cmd, &args).await?;

        // Initialize
        let init_result = session
            .send_request("initialize", Some(json!({
                "clientID": "omni",
                "clientName": "Omni Debugger",
                "adapterID": adapter_cmd,
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "supportsVariableType": true,
            })))
            .await?;

        session.initialized = true;

        // Launch
        let prog_args = params
            .get("args")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        let launch_result = session
            .send_request("launch", Some(json!({
                "program": program,
                "args": prog_args,
                "stopOnEntry": params.get("stop_on_entry").and_then(|v| v.as_bool()).unwrap_or(false),
            })))
            .await?;

        *guard = Some(session);

        Ok(json!({
            "status": "launched",
            "program": program,
            "adapter": adapter_cmd,
            "capabilities": init_result,
            "launch": launch_result,
        }))
    }

    async fn action_breakpoint(&self, params: &Value) -> Result<Value> {
        let file = params
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'file' is required".to_string()))?;
        let line = params
            .get("line")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'line' is required".to_string()))? as u32;
        let condition = params.get("condition").and_then(|v| v.as_str());

        let guard = self.session.lock().await;
        let session = guard
            .as_ref()
            .ok_or_else(|| LlmError::ToolCall("No active debug session".to_string()))?;

        let mut bp = json!({ "line": line });
        if let Some(cond) = condition {
            bp["condition"] = json!(cond);
        }

        let result = session
            .send_request("setBreakpoints", Some(json!({
                "source": { "path": file },
                "breakpoints": [bp]
            })))
            .await?;

        Ok(json!({
            "file": file,
            "line": line,
            "breakpoints": result.get("breakpoints")
        }))
    }

    async fn session_command(&self, command: &str) -> Result<Value> {
        let guard = self.session.lock().await;
        let session = guard
            .as_ref()
            .ok_or_else(|| LlmError::ToolCall("No active debug session".to_string()))?;

        let result = session
            .send_request(command, Some(json!({"threadId": 1})))
            .await?;

        Ok(json!({ "command": command, "result": result }))
    }

    async fn action_variables(&self, _params: &Value) -> Result<Value> {
        let guard = self.session.lock().await;
        let session = guard
            .as_ref()
            .ok_or_else(|| LlmError::ToolCall("No active debug session".to_string()))?;

        // Get stack trace to find frame
        let st = session
            .send_request("stackTrace", Some(json!({"threadId": 1})))
            .await?;

        let frame_id = st
            .get("stackFrames")
            .and_then(|f| f.as_array())
            .and_then(|a| a.first())
            .and_then(|f| f.get("id"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Get scopes
        let scopes = session
            .send_request("scopes", Some(json!({"frameId": frame_id})))
            .await?;

        let scope_ref = scopes
            .get("scopes")
            .and_then(|s| s.as_array())
            .and_then(|a| a.first())
            .and_then(|s| s.get("variablesReference"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Get variables
        let vars = session
            .send_request("variables", Some(json!({"variablesReference": scope_ref})))
            .await?;

        Ok(vars)
    }

    async fn action_evaluate(&self, params: &Value) -> Result<Value> {
        let expression = params
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'expression' is required".to_string()))?;

        let guard = self.session.lock().await;
        let session = guard
            .as_ref()
            .ok_or_else(|| LlmError::ToolCall("No active debug session".to_string()))?;

        let result = session
            .send_request("evaluate", Some(json!({
                "expression": expression,
                "context": "repl"
            })))
            .await?;

        Ok(result)
    }

    async fn action_stacktrace(&self) -> Result<Value> {
        let guard = self.session.lock().await;
        let session = guard
            .as_ref()
            .ok_or_else(|| LlmError::ToolCall("No active debug session".to_string()))?;

        let result = session
            .send_request("stackTrace", Some(json!({"threadId": 1})))
            .await?;

        Ok(result)
    }

    async fn action_attach(&self, params: &Value) -> Result<Value> {
        let process_id = params
            .get("process_id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| LlmError::ToolCall("'process_id' is required for attach".to_string()))?;

        let mut guard = self.session.lock().await;
        if guard.is_some() {
            return Err(LlmError::ToolCall(
                "Debug session already active. Use 'disconnect' first.".to_string(),
            ));
        }

        // For attach, detect adapter from program hint or default to codelldb
        let (adapter_cmd, adapter_args) = if let Some(program) = params.get("program").and_then(|v| v.as_str()) {
            Self::detect_adapter(program)?
        } else {
            if which_exe("codelldb").is_some() {
                ("codelldb".to_string(), vec!["--port".to_string(), "0".to_string()])
            } else if which_exe("lldb-vscode").is_some() {
                ("lldb-vscode".to_string(), Vec::new())
            } else {
                return Err(LlmError::ToolCall(
                    "No debug adapter found for attach. Install codelldb or specify 'program' hint.".to_string(),
                ));
            }
        };

        let args: Vec<&str> = adapter_args.iter().map(|s| s.as_str()).collect();
        let mut session = DebugSession::spawn(&adapter_cmd, &args).await?;

        // Initialize
        let init_result = session
            .send_request("initialize", Some(json!({
                "clientID": "omni",
                "clientName": "Omni Debugger",
                "adapterID": &adapter_cmd,
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "supportsVariableType": true,
            })))
            .await?;

        session.initialized = true;

        // Attach to running process
        let attach_result = session
            .send_request("attach", Some(json!({
                "pid": process_id,
            })))
            .await?;

        *guard = Some(session);

        Ok(json!({
            "status": "attached",
            "process_id": process_id,
            "adapter": adapter_cmd,
            "capabilities": init_result,
            "attach": attach_result,
        }))
    }

    async fn action_disconnect(&self) -> Result<Value> {
        let mut guard = self.session.lock().await;
        if let Some(mut session) = guard.take() {
            session.shutdown().await;
            Ok(json!({ "status": "disconnected" }))
        } else {
            Err(LlmError::ToolCall("No active debug session".to_string()))
        }
    }
}

fn which_exe(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(name);
            if candidate.exists() {
                Some(candidate)
            } else if cfg!(windows) {
                let with_exe = dir.join(format!("{}.exe", name));
                if with_exe.exists() {
                    Some(with_exe)
                } else {
                    None
                }
            } else {
                None
            }
        })
    })
}

#[async_trait]
impl NativeTool for DebuggerTool {
    fn name(&self) -> &str {
        "debugger"
    }

    fn description(&self) -> &str {
        "Control debug sessions via DAP. Actions: 'launch' (start debugging a program), \
         'attach' (attach to a running process by PID), 'breakpoint' (set a breakpoint), \
         'continue' (resume execution), 'step_over'/'step_into'/'step_out' (stepping), \
         'variables' (inspect current scope), 'evaluate' (evaluate an expression), \
         'stacktrace' (show call stack), 'disconnect' (end debug session). \
         Auto-detects debug adapters (debugpy, codelldb, node --inspect)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["launch", "attach", "breakpoint", "continue", "step_over",
                             "step_into", "step_out", "variables", "evaluate",
                             "stacktrace", "disconnect"],
                    "description": "Debug action to perform"
                },
                "program": {
                    "type": "string",
                    "description": "Path to program to debug (for 'launch')"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Program arguments (for 'launch')"
                },
                "stop_on_entry": {
                    "type": "boolean",
                    "description": "Stop at program entry (for 'launch')"
                },
                "file": {
                    "type": "string",
                    "description": "Source file path (for 'breakpoint')"
                },
                "line": {
                    "type": "integer",
                    "description": "Line number (for 'breakpoint')"
                },
                "condition": {
                    "type": "string",
                    "description": "Conditional breakpoint expression (for 'breakpoint')"
                },
                "expression": {
                    "type": "string",
                    "description": "Expression to evaluate (for 'evaluate')"
                },
                "process_id": {
                    "type": "integer",
                    "description": "PID of the process to attach to (for 'attach')"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::Debugging
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'action' is required".to_string()))?;

        match action {
            "launch" => self.action_launch(&params).await,
            "attach" => self.action_attach(&params).await,
            "breakpoint" => self.action_breakpoint(&params).await,
            "continue" => self.session_command("continue").await,
            "step_over" => self.session_command("next").await,
            "step_into" => self.session_command("stepIn").await,
            "step_out" => self.session_command("stepOut").await,
            "variables" => self.action_variables(&params).await,
            "evaluate" => self.action_evaluate(&params).await,
            "stacktrace" => self.action_stacktrace().await,
            "disconnect" => self.action_disconnect().await,
            _ => Err(LlmError::ToolCall(format!(
                "Unknown debugger action: '{action}'. Valid: launch, attach, breakpoint, continue, \
                 step_over, step_into, step_out, variables, evaluate, stacktrace, disconnect"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = DebuggerTool::new();
        assert_eq!(tool.name(), "debugger");
        assert!(!tool.description().is_empty());
        assert!(tool.parameters_schema().is_object());
        assert!(matches!(tool.required_capability(), Capability::Debugging));
    }

    #[test]
    fn test_detect_adapter_python() {
        let (cmd, _) = DebuggerTool::detect_adapter("test.py").unwrap();
        assert_eq!(cmd, "python");
    }

    #[test]
    fn test_detect_adapter_js() {
        let (cmd, _) = DebuggerTool::detect_adapter("app.js").unwrap();
        assert_eq!(cmd, "node");
    }

    #[tokio::test]
    async fn test_no_session_errors() {
        let tool = DebuggerTool::new();
        let result = tool.execute(json!({ "action": "continue" })).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No active debug session"));
    }

    #[tokio::test]
    async fn test_disconnect_no_session() {
        let tool = DebuggerTool::new();
        let result = tool.execute(json!({ "action": "disconnect" })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = DebuggerTool::new();
        let result = tool.execute(json!({ "action": "invalid" })).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown debugger action"));
    }
}
