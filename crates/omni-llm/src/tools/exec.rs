//! Shell command execution tool.
//!
//! Allows the AI agent to execute shell commands on the host system.
//! Gated by `process.spawn` permission.

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use tokio::process::Command;

use super::util::floor_char_boundary;
use super::NativeTool;
use crate::error::{LlmError, Result};

/// Native tool for executing shell commands.
pub struct ExecTool {
    /// Maximum output size in bytes (stdout + stderr).
    max_output_bytes: usize,
    /// Default timeout in seconds.
    default_timeout_secs: u64,
}

impl ExecTool {
    pub fn new() -> Self {
        Self {
            max_output_bytes: 50 * 1024,
            default_timeout_secs: 120,
        }
    }
}

#[async_trait]
impl NativeTool for ExecTool {
    fn name(&self) -> &str {
        "exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command on the host system. Returns stdout, stderr, and exit code. \
         Use this for system commands, scripts, and build tools that have no dedicated tool. \
         Prefer dedicated tools when available (e.g., use the git tool instead of `exec git ...`)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "working_directory": {
                    "type": "string",
                    "description": "Working directory for the command (optional, defaults to current directory)"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Timeout in seconds (optional, defaults to 120)"
                }
            },
            "required": ["command"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::ProcessSpawn(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let command_str = params["command"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'command' parameter is required".to_string()))?;

        let working_dir = params["working_directory"].as_str();
        let timeout_secs = params["timeout_secs"]
            .as_u64()
            .unwrap_or(self.default_timeout_secs);

        // Use platform-appropriate shell
        let (shell, flag) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let mut cmd = Command::new(shell);
        cmd.arg(flag).arg(command_str);

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            cmd.output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                // Truncate output (UTF-8 safe -- find nearest char boundary)
                let max = self.max_output_bytes;
                let stdout_result = if stdout.len() > max {
                    let end = floor_char_boundary(&stdout, max);
                    format!("{}...\n[truncated, {} total bytes]", &stdout[..end], stdout.len())
                } else {
                    stdout.to_string()
                };
                let stderr_result = if stderr.len() > max {
                    let end = floor_char_boundary(&stderr, max);
                    format!("{}...\n[truncated, {} total bytes]", &stderr[..end], stderr.len())
                } else {
                    stderr.to_string()
                };

                Ok(serde_json::json!({
                    "exit_code": exit_code,
                    "stdout": stdout_result,
                    "stderr": stderr_result,
                }))
            }
            Ok(Err(e)) => Err(LlmError::ToolCall(format!("Failed to execute command: {e}"))),
            Err(_) => Err(LlmError::Timeout(timeout_secs)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_tool_schema() {
        let tool = ExecTool::new();
        assert_eq!(tool.name(), "exec");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["command"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("command")));
    }

    #[test]
    fn test_exec_tool_capability() {
        let tool = ExecTool::new();
        assert_eq!(tool.required_capability().capability_key(), "process.spawn");
    }

    #[tokio::test]
    async fn test_exec_echo() {
        let tool = ExecTool::new();
        let result = tool
            .execute(serde_json::json!({"command": "echo hello"}))
            .await
            .unwrap();
        assert_eq!(result["exit_code"], 0);
        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains("hello"), "stdout was: {stdout}");
    }

    #[tokio::test]
    async fn test_exec_missing_command() {
        let tool = ExecTool::new();
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_exec_nonexistent_command() {
        let tool = ExecTool::new();
        let result = tool
            .execute(serde_json::json!({"command": "this_command_does_not_exist_12345"}))
            .await;
        // On Windows cmd /C returns exit code 1 for unknown commands, on Unix sh -c returns 127
        match result {
            Ok(val) => assert_ne!(val["exit_code"], 0),
            Err(_) => {} // Also acceptable
        }
    }

    #[tokio::test]
    async fn test_exec_timeout() {
        let mut tool = ExecTool::new();
        tool.default_timeout_secs = 1;
        let cmd = if cfg!(target_os = "windows") {
            "ping -n 10 127.0.0.1"
        } else {
            "sleep 10"
        };
        let result = tool
            .execute(serde_json::json!({"command": cmd}))
            .await;
        assert!(result.is_err());
    }
}
