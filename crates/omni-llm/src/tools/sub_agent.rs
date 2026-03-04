//! Sub-agent spawning tool for multi-agent orchestration.
//!
//! Allows the main agent to delegate tasks to parallel sub-agents,
//! each running their own AgentLoop with a restricted tool set.
//!
//! Returns a delegated action for the AgentLoop to handle.
//! Gated by `agent.spawn` permission.

use async_trait::async_trait;
use omni_permissions::capability::Capability;
use serde_json::{json, Value};

use super::NativeTool;
use crate::error::{LlmError, Result};

pub struct AgentSpawnTool;

impl AgentSpawnTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NativeTool for AgentSpawnTool {
    fn name(&self) -> &str {
        "agent_spawn"
    }

    fn description(&self) -> &str {
        "Spawn a sub-agent to handle a task in parallel. The sub-agent gets its own conversation \
         context and tool access (except agent_spawn, to prevent recursion). Use this for \
         independent tasks that can run concurrently, like writing tests while refactoring code. \
         Set 'wait' to true to block until the sub-agent completes and get its result, or false \
         to get a task ID for checking later."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The task description for the sub-agent to execute"
                },
                "context_files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "File paths to read and include as context for the sub-agent"
                },
                "model": {
                    "type": "string",
                    "description": "LLM model to use (defaults to parent agent's model)"
                },
                "max_iterations": {
                    "type": "integer",
                    "description": "Maximum agent loop iterations (default: 15)"
                },
                "wait": {
                    "type": "boolean",
                    "description": "Wait for the sub-agent to complete (default: true). If false, returns a task_id."
                }
            },
            "required": ["task"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::AgentSpawn(None)
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let task = params
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| LlmError::ToolCall("'task' is required".to_string()))?
            .to_string();

        let context_files = params
            .get("context_files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let model = params
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let max_iterations = params
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(15) as u32;

        let wait = params
            .get("wait")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Return a delegated action for the AgentLoop to handle
        Ok(json!({
            "action": "spawn_agent",
            "task": task,
            "context_files": context_files,
            "model": model,
            "max_iterations": max_iterations,
            "wait": wait
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = AgentSpawnTool::new();
        assert_eq!(tool.name(), "agent_spawn");
        assert!(!tool.description().is_empty());
        assert!(tool.parameters_schema().is_object());
        assert!(matches!(tool.required_capability(), Capability::AgentSpawn(_)));
    }

    #[tokio::test]
    async fn test_returns_delegated_action() {
        let tool = AgentSpawnTool::new();
        let result = tool
            .execute(json!({
                "task": "Write tests for auth module",
                "context_files": ["src/auth.rs"],
                "max_iterations": 10,
                "wait": true
            }))
            .await
            .unwrap();

        assert_eq!(result["action"], "spawn_agent");
        assert_eq!(result["task"], "Write tests for auth module");
        assert_eq!(result["max_iterations"], 10);
        assert_eq!(result["wait"], true);
    }

    #[tokio::test]
    async fn test_defaults() {
        let tool = AgentSpawnTool::new();
        let result = tool
            .execute(json!({ "task": "Do something" }))
            .await
            .unwrap();

        assert_eq!(result["max_iterations"], 15);
        assert_eq!(result["wait"], true);
        assert!(result["model"].is_null());
    }

    #[tokio::test]
    async fn test_missing_task() {
        let tool = AgentSpawnTool::new();
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }
}
