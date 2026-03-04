//! Notification tool for sending system notifications to the user.
//!
//! Gated by `system.notifications` permission.

use async_trait::async_trait;
use omni_permissions::capability::Capability;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Native tool for sending system notifications.
pub struct NotifyTool;

#[async_trait]
impl NativeTool for NotifyTool {
    fn name(&self) -> &str {
        "notify"
    }

    fn description(&self) -> &str {
        "Send a system notification to the user. Shows as a toast notification in the \
         desktop app or terminal alert in CLI mode. Use for important alerts, task \
         completion notices, or time-sensitive information."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Notification title (short, 1-2 words)"
                },
                "body": {
                    "type": "string",
                    "description": "Notification body text"
                },
                "urgency": {
                    "type": "string",
                    "description": "Urgency level: 'low', 'normal', or 'critical'. Defaults to 'normal'.",
                    "enum": ["low", "normal", "critical"]
                }
            },
            "required": ["title", "body"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::SystemNotifications
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let title = params["title"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'title' parameter is required".to_string()))?;
        let body = params["body"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'body' parameter is required".to_string()))?;
        let urgency = params["urgency"].as_str().unwrap_or("normal");

        // Validate urgency
        match urgency {
            "low" | "normal" | "critical" => {}
            _ => {
                return Err(LlmError::ToolCall(format!(
                    "Invalid urgency '{}'. Must be 'low', 'normal', or 'critical'.",
                    urgency
                )));
            }
        }

        // The notification is returned as structured data.
        // The agent loop emits an OmniEvent::Notification, which the Tauri frontend
        // displays as an OS notification or the CLI prints to stderr.
        Ok(serde_json::json!({
            "action": "notify",
            "title": title,
            "body": body,
            "urgency": urgency,
            "status": "sent",
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_schema() {
        let tool = NotifyTool;
        assert_eq!(tool.name(), "notify");
        assert_eq!(
            tool.required_capability().capability_key(),
            "system.notifications"
        );
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("title")));
        assert!(required.contains(&serde_json::json!("body")));
    }

    #[tokio::test]
    async fn test_notify_execution() {
        let tool = NotifyTool;
        let result = tool
            .execute(serde_json::json!({
                "title": "Task Complete",
                "body": "Build finished successfully.",
                "urgency": "normal",
            }))
            .await
            .unwrap();
        assert_eq!(result["status"], "sent");
        assert_eq!(result["title"], "Task Complete");
        assert_eq!(result["urgency"], "normal");
    }

    #[tokio::test]
    async fn test_notify_default_urgency() {
        let tool = NotifyTool;
        let result = tool
            .execute(serde_json::json!({
                "title": "Info",
                "body": "Something happened.",
            }))
            .await
            .unwrap();
        assert_eq!(result["urgency"], "normal");
    }

    #[tokio::test]
    async fn test_notify_invalid_urgency() {
        let tool = NotifyTool;
        let result = tool
            .execute(serde_json::json!({
                "title": "Test",
                "body": "Test",
                "urgency": "extreme",
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid urgency"));
    }
}
