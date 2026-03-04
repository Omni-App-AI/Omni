//! Session/subagent orchestration tools.
//!
//! Allows the agent to spawn subagent sessions, list sessions, and retrieve
//! conversation history. Gated by `process.spawn` and `storage.persistent`.

use std::sync::Arc;

use async_trait::async_trait;
use omni_core::database::Database;
use omni_permissions::capability::Capability;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Native tool for listing active/recent sessions.
pub struct SessionListTool {
    db: Arc<std::sync::Mutex<Database>>,
}

impl SessionListTool {
    pub fn new(db: Arc<std::sync::Mutex<Database>>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NativeTool for SessionListTool {
    fn name(&self) -> &str {
        "session_list"
    }

    fn description(&self) -> &str {
        "List recent conversation sessions. Returns session IDs, creation times, and message counts. \
         Use this to find a session ID for use with session_history."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of sessions to return (default: 20)"
                }
            },
            "required": []
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::StoragePersistent(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let limit = params["limit"].as_u64().unwrap_or(20) as usize;

        let db = self.db.clone();
        let sessions = tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.list_sessions()
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Failed to query sessions: {e}")))?
        .map_err(|e| LlmError::ToolCall(format!("Database error: {e}")))?;

        let session_list: Vec<serde_json::Value> = sessions
            .into_iter()
            .take(limit)
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "created_at": s.created_at,
                    "updated_at": s.updated_at,
                    "metadata": s.metadata,
                })
            })
            .collect();

        let total = session_list.len();
        Ok(serde_json::json!({
            "sessions": session_list,
            "total": total,
        }))
    }
}

/// Native tool for retrieving conversation history from a session.
pub struct SessionHistoryTool {
    db: Arc<std::sync::Mutex<Database>>,
}

impl SessionHistoryTool {
    pub fn new(db: Arc<std::sync::Mutex<Database>>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl NativeTool for SessionHistoryTool {
    fn name(&self) -> &str {
        "session_history"
    }

    fn description(&self) -> &str {
        "Retrieve conversation history from a specific session. Returns messages \
         with roles (user/assistant/system) and content. Use session_list first to \
         find the session_id."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "The session ID to retrieve history for"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum messages to return (default: 50)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Number of messages to skip (default: 0)"
                }
            },
            "required": ["session_id"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::StoragePersistent(None)
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let session_id = params["session_id"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'session_id' parameter is required".to_string()))?;
        let limit = params["limit"].as_u64().unwrap_or(50) as usize;
        let offset = params["offset"].as_u64().unwrap_or(0) as usize;

        let db = self.db.clone();
        let sid = session_id.to_string();
        let messages = tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.get_messages_for_session(&sid)
        })
        .await
        .map_err(|e| LlmError::ToolCall(format!("Failed to query messages: {e}")))?
        .map_err(|e| LlmError::ToolCall(format!("Database error: {e}")))?;

        let msg_list: Vec<serde_json::Value> = messages
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        let total = msg_list.len();
        Ok(serde_json::json!({
            "messages": msg_list,
            "total": total,
            "session_id": session_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Arc<std::sync::Mutex<Database>> {
        Arc::new(std::sync::Mutex::new(
            Database::open(
                &std::env::temp_dir().join("omni_test_sessions.db"),
                "test-key",
            )
            .unwrap(),
        ))
    }

    #[test]
    fn test_session_list_schema() {
        let db = test_db();
        let tool = SessionListTool::new(db);
        assert_eq!(tool.name(), "session_list");
        assert_eq!(tool.required_capability().capability_key(), "storage.persistent");
    }

    #[test]
    fn test_session_history_schema() {
        let db = test_db();
        let tool = SessionHistoryTool::new(db);
        assert_eq!(tool.name(), "session_history");
        assert_eq!(tool.required_capability().capability_key(), "storage.persistent");
    }

    #[tokio::test]
    async fn test_session_list_empty() {
        let db = test_db();
        let tool = SessionListTool::new(db);
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(result["sessions"].is_array());
    }

    #[tokio::test]
    async fn test_session_history_with_session() {
        let db = test_db();

        // Create a session and add a message
        {
            let d = db.lock().unwrap();
            let sid = d.create_session(None).unwrap();
            d.insert_message(&omni_core::database::NewMessage {
                session_id: sid.clone(),
                role: "user".to_string(),
                content: "Hello from session test".to_string(),
                tool_call_id: None,
                tool_calls: None,
                token_count: None,
            })
            .unwrap();

            // Test with this session
            drop(d);

            let tool = SessionHistoryTool::new(db.clone());
            let result = tool
                .execute(serde_json::json!({"session_id": sid}))
                .await
                .unwrap();

            let msgs = result["messages"].as_array().unwrap();
            assert!(!msgs.is_empty());
            assert_eq!(msgs[0]["role"], "user");
        }
    }
}
