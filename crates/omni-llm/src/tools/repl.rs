//! Persistent REPL tool for interactive code execution.
//!
//! Spawns language interpreters (Python, Node.js, etc.) as long-lived
//! processes with persistent state between executions.
//!
//! Gated by `process.spawn` permission.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Maximum output from a single execution (50KB).
const MAX_OUTPUT: usize = 50 * 1024;
/// Maximum concurrent REPL sessions.
const MAX_SESSIONS: usize = 3;
/// Execution timeout (30 seconds).
const EXEC_TIMEOUT_SECS: u64 = 30;

struct ReplSession {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_lines: Arc<Mutex<Vec<String>>>,
    language: String,
}

pub struct ReplTool {
    sessions: Arc<Mutex<HashMap<String, ReplSession>>>,
    next_id: std::sync::atomic::AtomicU32,
}

impl ReplTool {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_id: std::sync::atomic::AtomicU32::new(1),
        }
    }

    fn get_interpreter(language: &str) -> Result<(String, Vec<String>)> {
        match language.to_lowercase().as_str() {
            "python" | "python3" | "py" => {
                let cmd = if cfg!(windows) { "python" } else { "python3" };
                Ok((cmd.to_string(), vec!["-i".to_string(), "-u".to_string()]))
            }
            "javascript" | "js" | "node" => {
                Ok(("node".to_string(), vec!["-i".to_string()]))
            }
            _ => Err(LlmError::ToolCall(format!(
                "Unsupported REPL language: '{}'. Supported: python, javascript",
                language
            ))),
        }
    }

    async fn action_start(&self, params: &Value) -> Result<Value> {
        let language = params
            .get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'language' is required".to_string()))?;

        let mut sessions = self.sessions.lock().await;
        if sessions.len() >= MAX_SESSIONS {
            return Err(LlmError::ToolCall(format!(
                "Maximum {} concurrent REPL sessions reached. Stop an existing session first.",
                MAX_SESSIONS
            )));
        }

        let (cmd, args) = Self::get_interpreter(language)?;
        let mut child = Command::new(&cmd)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| LlmError::ToolCall(format!("Failed to start {}: {e}", cmd)))?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        // Collect stdout lines in background
        let stdout_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let lines_clone = Arc::clone(&stdout_lines);
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut buf = lines_clone.lock().await;
                buf.push(line);
            }
        });

        // Drain stderr
        if let Some(stderr) = child.stderr.take() {
            let lines_clone2 = Arc::clone(&stdout_lines);
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    // Merge stderr into stdout buffer
                    let mut buf = lines_clone2.lock().await;
                    buf.push(format!("[stderr] {}", line));
                }
            });
        }

        let session_id = format!(
            "repl-{}-{}",
            language,
            self.next_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );

        let session = ReplSession {
            child,
            stdin,
            stdout_lines,
            language: language.to_string(),
        };

        let id = session_id.clone();
        sessions.insert(session_id, session);

        // Brief delay for the interpreter to start
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(json!({
            "session_id": id,
            "language": language,
            "status": "started"
        }))
    }

    async fn action_execute(&self, params: &Value) -> Result<Value> {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'session_id' is required".to_string()))?;
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'code' is required".to_string()))?;

        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(session_id).ok_or_else(|| {
            LlmError::ToolCall(format!("REPL session '{}' not found", session_id))
        })?;

        // Clear previous output
        {
            let mut lines = session.stdout_lines.lock().await;
            lines.clear();
        }

        // Send code with a unique marker so we know when output is done
        let marker = format!("__OMNI_REPL_DONE_{}__", uuid::Uuid::new_v4().as_simple());
        let code_with_marker = match session.language.as_str() {
            "python" | "python3" | "py" => {
                format!("{}\nprint('{}')\n", code, marker)
            }
            _ => {
                format!("{}\nconsole.log('{}')\n", code, marker)
            }
        };

        session
            .stdin
            .write_all(code_with_marker.as_bytes())
            .await
            .map_err(|e| LlmError::ToolCall(format!("Failed to send code to REPL: {e}")))?;
        session.stdin.flush().await.ok();

        // Wait for marker in output (with timeout)
        let lines_ref = Arc::clone(&session.stdout_lines);
        let marker_clone = marker.clone();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(EXEC_TIMEOUT_SECS),
            async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let lines = lines_ref.lock().await;
                    if lines.iter().any(|l| l.contains(&marker_clone)) {
                        return lines.clone();
                    }
                }
            },
        )
        .await;

        match result {
            Ok(lines) => {
                let output: Vec<&str> = lines
                    .iter()
                    .filter(|l| !l.contains(&marker))
                    .map(|s| s.as_str())
                    .collect();
                let mut output_text = output.join("\n");
                let truncated = output_text.len() > MAX_OUTPUT;
                if truncated {
                    output_text.truncate(MAX_OUTPUT);
                    output_text.push_str("\n... (output truncated)");
                }

                Ok(json!({
                    "session_id": session_id,
                    "output": output_text,
                    "truncated": truncated,
                    "success": true
                }))
            }
            Err(_) => Ok(json!({
                "session_id": session_id,
                "output": "Execution timed out",
                "success": false,
                "error": format!("Execution timed out after {}s", EXEC_TIMEOUT_SECS)
            })),
        }
    }

    async fn action_stop(&self, params: &Value) -> Result<Value> {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'session_id' is required".to_string()))?;

        let mut sessions = self.sessions.lock().await;
        if let Some(mut session) = sessions.remove(session_id) {
            let _ = session.child.kill().await;
            Ok(json!({
                "session_id": session_id,
                "status": "stopped"
            }))
        } else {
            Err(LlmError::ToolCall(format!(
                "REPL session '{}' not found",
                session_id
            )))
        }
    }

    async fn action_list(&self) -> Result<Value> {
        let sessions = self.sessions.lock().await;
        let list: Vec<Value> = sessions
            .iter()
            .map(|(id, s)| {
                json!({
                    "session_id": id,
                    "language": s.language,
                })
            })
            .collect();

        Ok(json!({
            "sessions": list,
            "count": list.len()
        }))
    }
}

#[async_trait]
impl NativeTool for ReplTool {
    fn name(&self) -> &str {
        "repl"
    }

    fn description(&self) -> &str {
        "Interactive REPL with persistent state. Actions: 'start' (create session), \
         'execute' (run code in session), 'stop' (end session), 'list' (show active sessions). \
         Variables and state persist between executions within a session. \
         Supported languages: python, javascript."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "execute", "stop", "list"],
                    "description": "REPL action to perform"
                },
                "language": {
                    "type": "string",
                    "enum": ["python", "javascript"],
                    "description": "Programming language (for 'start' action)"
                },
                "session_id": {
                    "type": "string",
                    "description": "Session ID (for 'execute' and 'stop' actions)"
                },
                "code": {
                    "type": "string",
                    "description": "Code to execute (for 'execute' action)"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::ProcessSpawn(None)
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'action' is required".to_string()))?;

        match action {
            "start" => self.action_start(&params).await,
            "execute" => self.action_execute(&params).await,
            "stop" => self.action_stop(&params).await,
            "list" => self.action_list().await,
            _ => Err(LlmError::ToolCall(format!(
                "Unknown repl action: '{action}'. Valid: start, execute, stop, list"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = ReplTool::new();
        assert_eq!(tool.name(), "repl");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_get_interpreter_python() {
        let (cmd, _) = ReplTool::get_interpreter("python").unwrap();
        assert!(cmd == "python" || cmd == "python3");
    }

    #[test]
    fn test_get_interpreter_js() {
        let (cmd, _) = ReplTool::get_interpreter("javascript").unwrap();
        assert_eq!(cmd, "node");
    }

    #[test]
    fn test_get_interpreter_unknown() {
        assert!(ReplTool::get_interpreter("ruby").is_err());
    }

    #[tokio::test]
    async fn test_list_empty() {
        let tool = ReplTool::new();
        let result = tool.execute(json!({ "action": "list" })).await.unwrap();
        assert_eq!(result["count"], 0);
        assert!(result["sessions"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_stop_nonexistent() {
        let tool = ReplTool::new();
        let result = tool
            .execute(json!({ "action": "stop", "session_id": "nonexistent" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_nonexistent() {
        let tool = ReplTool::new();
        let result = tool
            .execute(json!({
                "action": "execute",
                "session_id": "nonexistent",
                "code": "1+1"
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = ReplTool::new();
        let result = tool.execute(json!({ "action": "invalid" })).await;
        assert!(result.is_err());
    }
}
