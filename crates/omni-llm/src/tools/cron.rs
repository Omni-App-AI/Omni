//! Cron/scheduling tool for creating and managing scheduled tasks.
//!
//! Gated by `system.scheduling` permission.

use async_trait::async_trait;
use omni_permissions::capability::Capability;

use super::NativeTool;
use crate::error::{LlmError, Result};

/// Native tool for scheduling recurring or one-time tasks.
pub struct CronScheduleTool;

#[async_trait]
impl NativeTool for CronScheduleTool {
    fn name(&self) -> &str {
        "cron_schedule"
    }

    fn description(&self) -> &str {
        "Create, list, or remove scheduled tasks. Tasks run at specified cron intervals \
         and execute the given task string through the agent. Supports standard cron syntax \
         (e.g. '0 9 * * *' for daily at 9am, '*/5 * * * *' for every 5 minutes)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "The action to perform: 'add', 'list', 'remove'",
                    "enum": ["add", "list", "remove"]
                },
                "name": {
                    "type": "string",
                    "description": "Human-readable name for the scheduled task (required for 'add')"
                },
                "schedule": {
                    "type": "string",
                    "description": "Cron expression (required for 'add'). E.g. '0 9 * * *', '*/30 * * * *'"
                },
                "task": {
                    "type": "string",
                    "description": "Task description/prompt for the agent to execute when triggered (required for 'add')"
                },
                "job_id": {
                    "type": "string",
                    "description": "Job ID to remove (required for 'remove')"
                }
            },
            "required": ["action"]
        })
    }

    fn required_capability(&self) -> Capability {
        Capability::SystemScheduling
    }

    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let action = params["action"]
            .as_str()
            .ok_or_else(|| LlmError::ToolCall("'action' parameter is required".to_string()))?;

        match action {
            "add" => {
                let name = params["name"]
                    .as_str()
                    .ok_or_else(|| LlmError::ToolCall("'name' is required for 'add' action".to_string()))?;
                let schedule = params["schedule"]
                    .as_str()
                    .ok_or_else(|| LlmError::ToolCall("'schedule' is required for 'add' action".to_string()))?;
                let task = params["task"]
                    .as_str()
                    .ok_or_else(|| LlmError::ToolCall("'task' is required for 'add' action".to_string()))?;

                // Validate cron expression (field count + range checks)
                let parts: Vec<&str> = schedule.split_whitespace().collect();
                if parts.len() != 5 {
                    return Err(LlmError::ToolCall(format!(
                        "Invalid cron expression '{}': expected 5 fields (minute hour day month weekday)",
                        schedule
                    )));
                }

                // Validate each field's range: minute(0-59), hour(0-23), day(1-31), month(1-12), weekday(0-7)
                let ranges: [(u32, u32); 5] = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
                let field_names = ["minute", "hour", "day", "month", "weekday"];
                for (i, part) in parts.iter().enumerate() {
                    if let Err(msg) = validate_cron_field(part, ranges[i].0, ranges[i].1) {
                        return Err(LlmError::ToolCall(format!(
                            "Invalid cron {}: '{}' -- {}",
                            field_names[i], part, msg
                        )));
                    }
                }

                let job_id = uuid::Uuid::new_v4().to_string()[..8].to_string();

                // Return structured data for the scheduler to process.
                // The actual scheduling is handled by a higher-level scheduler component.
                Ok(serde_json::json!({
                    "action": "add",
                    "job_id": job_id,
                    "name": name,
                    "schedule": schedule,
                    "task": task,
                    "status": "scheduled",
                    "note": "Job registered. The scheduler will execute the task at the specified intervals.",
                }))
            }
            "list" => {
                // Returns placeholder -- actual job listing handled by scheduler component
                Ok(serde_json::json!({
                    "action": "list",
                    "jobs": [],
                    "note": "Job listing delegated to scheduler. Connect a scheduler to see active jobs.",
                }))
            }
            "remove" => {
                let job_id = params["job_id"]
                    .as_str()
                    .ok_or_else(|| LlmError::ToolCall("'job_id' is required for 'remove' action".to_string()))?;

                Ok(serde_json::json!({
                    "action": "remove",
                    "job_id": job_id,
                    "status": "removed",
                    "note": "Job removal queued for scheduler.",
                }))
            }
            _ => Err(LlmError::ToolCall(format!(
                "Unknown action '{}'. Must be 'add', 'list', or 'remove'.",
                action
            ))),
        }
    }
}

/// Validate a single cron field against its valid range.
/// Supports: `*`, `*/N`, `N`, `N-M`, and comma-separated lists.
fn validate_cron_field(field: &str, min: u32, max: u32) -> std::result::Result<(), String> {
    for part in field.split(',') {
        let part = part.trim();
        if part == "*" {
            continue;
        }
        if let Some(step) = part.strip_prefix("*/") {
            let n: u32 = step.parse().map_err(|_| format!("invalid step '{step}'"))?;
            if n == 0 || n > max {
                return Err(format!("step {n} out of range (1-{max})"));
            }
            continue;
        }
        if let Some((lo, hi)) = part.split_once('-') {
            let lo: u32 = lo.parse().map_err(|_| format!("invalid number '{lo}'"))?;
            let hi: u32 = hi.parse().map_err(|_| format!("invalid number '{hi}'"))?;
            if lo < min || hi > max || lo > hi {
                return Err(format!("range {lo}-{hi} out of bounds ({min}-{max})"));
            }
            continue;
        }
        let n: u32 = part.parse().map_err(|_| format!("invalid number '{part}'"))?;
        if n < min || n > max {
            return Err(format!("value {n} out of range ({min}-{max})"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_schedule_schema() {
        let tool = CronScheduleTool;
        assert_eq!(tool.name(), "cron_schedule");
        assert_eq!(
            tool.required_capability().capability_key(),
            "system.scheduling"
        );
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("action")));
    }

    #[tokio::test]
    async fn test_cron_add() {
        let tool = CronScheduleTool;
        let result = tool
            .execute(serde_json::json!({
                "action": "add",
                "name": "Daily backup",
                "schedule": "0 3 * * *",
                "task": "Run database backup",
            }))
            .await
            .unwrap();
        assert_eq!(result["action"], "add");
        assert_eq!(result["status"], "scheduled");
        assert_eq!(result["name"], "Daily backup");
        assert!(result["job_id"].is_string());
    }

    #[tokio::test]
    async fn test_cron_list() {
        let tool = CronScheduleTool;
        let result = tool
            .execute(serde_json::json!({"action": "list"}))
            .await
            .unwrap();
        assert_eq!(result["action"], "list");
        assert!(result["jobs"].is_array());
    }

    #[tokio::test]
    async fn test_cron_remove() {
        let tool = CronScheduleTool;
        let result = tool
            .execute(serde_json::json!({
                "action": "remove",
                "job_id": "abc12345",
            }))
            .await
            .unwrap();
        assert_eq!(result["status"], "removed");
    }

    #[tokio::test]
    async fn test_cron_invalid_schedule() {
        let tool = CronScheduleTool;
        let result = tool
            .execute(serde_json::json!({
                "action": "add",
                "name": "Bad job",
                "schedule": "not a cron",
                "task": "do something",
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid cron"));
    }

    #[tokio::test]
    async fn test_cron_invalid_field_values() {
        let tool = CronScheduleTool;
        // Minute field out of range (0-59)
        let result = tool
            .execute(serde_json::json!({
                "action": "add",
                "name": "Bad minute",
                "schedule": "99 0 * * *",
                "task": "do something",
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("minute"));

        // Hour field out of range (0-23)
        let result = tool
            .execute(serde_json::json!({
                "action": "add",
                "name": "Bad hour",
                "schedule": "0 25 * * *",
                "task": "do something",
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("hour"));
    }

    #[test]
    fn test_validate_cron_field() {
        assert!(validate_cron_field("*", 0, 59).is_ok());
        assert!(validate_cron_field("*/5", 0, 59).is_ok());
        assert!(validate_cron_field("0", 0, 59).is_ok());
        assert!(validate_cron_field("0-30", 0, 59).is_ok());
        assert!(validate_cron_field("1,15,30", 0, 59).is_ok());
        assert!(validate_cron_field("99", 0, 59).is_err());
        assert!(validate_cron_field("*/0", 0, 59).is_err());
        assert!(validate_cron_field("abc", 0, 59).is_err());
    }

    #[tokio::test]
    async fn test_cron_unknown_action() {
        let tool = CronScheduleTool;
        let result = tool
            .execute(serde_json::json!({"action": "pause"}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown action"));
    }
}
