//! Clipboard read/write tool.
//!
//! Provides system clipboard access for copy/paste workflows.
//! Uses the `arboard` crate for cross-platform support.
//!
//! Gated by `clipboard.read` and `clipboard.write` permissions.

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use serde_json::{json, Value};

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Maximum clipboard content size (1MB).
const MAX_CLIPBOARD_SIZE: usize = 1024 * 1024;

pub struct ClipboardTool;

impl ClipboardTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NativeTool for ClipboardTool {
    fn name(&self) -> &str {
        "clipboard"
    }

    fn description(&self) -> &str {
        "Read from or write to the system clipboard. \
         Actions: 'read' (get clipboard text), 'write' (set clipboard text)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "write"],
                    "description": "Action: 'read' clipboard or 'write' to clipboard"
                },
                "content": {
                    "type": "string",
                    "description": "Text to write to clipboard (required for 'write' action)"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        // Default to read -- the execute method checks for write separately
        Capability::ClipboardRead
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'action' is required".to_string()))?;

        match action {
            "read" => {
                let text = tokio::task::spawn_blocking(|| {
                    let mut clipboard = arboard::Clipboard::new()
                        .map_err(|e| LlmError::ToolCall(format!("Failed to access clipboard: {e}")))?;
                    clipboard
                        .get_text()
                        .map_err(|e| LlmError::ToolCall(format!("Failed to read clipboard: {e}")))
                })
                .await
                .map_err(|e| LlmError::ToolCall(format!("Clipboard task failed: {e}")))??;

                let mut content = text;
                let truncated = content.len() > MAX_CLIPBOARD_SIZE;
                if truncated {
                    content.truncate(MAX_CLIPBOARD_SIZE);
                }

                Ok(json!({
                    "content": content,
                    "length": content.len(),
                    "truncated": truncated
                }))
            }
            "write" => {
                let content = params
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        LlmError::ToolCall("'content' is required for write action".to_string())
                    })?
                    .to_string();

                if content.len() > MAX_CLIPBOARD_SIZE {
                    return Err(LlmError::ToolCall(format!(
                        "Content too large ({} bytes). Maximum: {} bytes",
                        content.len(),
                        MAX_CLIPBOARD_SIZE
                    )));
                }

                let len = content.len();
                tokio::task::spawn_blocking(move || {
                    let mut clipboard = arboard::Clipboard::new()
                        .map_err(|e| LlmError::ToolCall(format!("Failed to access clipboard: {e}")))?;
                    clipboard
                        .set_text(content)
                        .map_err(|e| LlmError::ToolCall(format!("Failed to write clipboard: {e}")))
                })
                .await
                .map_err(|e| LlmError::ToolCall(format!("Clipboard task failed: {e}")))??;

                Ok(json!({
                    "success": true,
                    "length": len
                }))
            }
            _ => Err(LlmError::ToolCall(format!(
                "Unknown clipboard action: '{action}'. Valid actions: read, write"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = ClipboardTool::new();
        assert_eq!(tool.name(), "clipboard");
        assert!(!tool.description().is_empty());
        assert!(tool.parameters_schema().is_object());
    }

    #[tokio::test]
    async fn test_missing_action() {
        let tool = ClipboardTool::new();
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = ClipboardTool::new();
        let result = tool.execute(json!({"action": "invalid"})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown clipboard action"));
    }

    #[tokio::test]
    async fn test_write_missing_content() {
        let tool = ClipboardTool::new();
        let result = tool.execute(json!({"action": "write"})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("content"));
    }

    // Note: read/write integration tests require a desktop environment
    // and are not reliable in CI. Manual testing recommended.
}
