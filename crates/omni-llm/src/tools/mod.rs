//! Native tool system for Omni.
//!
//! Built-in tools that run as native Rust code in the core runtime,
//! gated by the permission system. These provide core system automation
//! capabilities (exec, file I/O, web fetch) without requiring WASM extensions.

pub mod app_interact;
pub mod browser;
pub mod clipboard;
pub mod code_search;
pub mod cron;
pub mod debugger;
pub mod exec;
pub mod fs;
pub mod git;
pub mod image;
pub mod lsp;
pub mod memory;
pub mod message;
pub mod notify;
pub mod repl;
pub mod sessions;
pub mod sub_agent;
pub mod testing;
pub mod util;
pub mod web;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use omni_core::database::Database;
use omni_permissions::capability::Capability;
use omni_permissions::policy::{PermissionDecision, PolicyEngine};

use crate::error::{LlmError, Result};
use crate::types::ToolSchema;

/// Trait for native tools that run in the core runtime.
/// All native tools are permission-gated via the PolicyEngine.
#[async_trait]
pub trait NativeTool: Send + Sync {
    /// The tool name as it appears to the LLM.
    fn name(&self) -> &str;

    /// Human-readable description for the LLM.
    fn description(&self) -> &str;

    /// JSON Schema for the tool parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// The permission capability required to use this tool.
    fn required_capability(&self) -> Capability;

    /// Execute the tool with the given parameters.
    /// Permission checking is handled by the registry, not individual tools.
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value>;
}

/// Registry holding all native tools.
pub struct NativeToolRegistry {
    tools: HashMap<String, Box<dyn NativeTool>>,
    policy_engine: Arc<PolicyEngine>,
}

impl NativeToolRegistry {
    /// Create a new registry with all built-in tools registered.
    pub fn new(policy_engine: Arc<PolicyEngine>) -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
            policy_engine,
        };
        registry.register_builtins(None);
        registry
    }

    /// Create a new registry with database access for session tools.
    pub fn new_with_db(policy_engine: Arc<PolicyEngine>, db: Arc<Mutex<Database>>) -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
            policy_engine,
        };
        registry.register_builtins(Some(db));
        registry
    }

    fn register_builtins(&mut self, db: Option<Arc<Mutex<Database>>>) {
        // Core system tools
        self.register(Box::new(exec::ExecTool::new()));
        self.register(Box::new(fs::ReadFileTool));
        self.register(Box::new(fs::WriteFileTool));
        self.register(Box::new(fs::EditFileTool));
        self.register(Box::new(fs::ListFilesTool));
        self.register(Box::new(fs::ApplyPatchTool));
        self.register(Box::new(fs::GrepSearchTool));

        // Web tools
        self.register(Box::new(web::WebFetchTool::new()));
        self.register(Box::new(web::WebSearchTool::new()));

        // Memory tools
        self.register(Box::new(memory::MemorySaveTool { base_dir_override: None }));
        self.register(Box::new(memory::MemorySearchTool { base_dir_override: None }));
        self.register(Box::new(memory::MemoryGetTool { base_dir_override: None }));

        // Image tool
        self.register(Box::new(image::ImageAnalyzeTool));

        // Messaging tools
        self.register(Box::new(message::SendMessageTool));
        self.register(Box::new(message::ListChannelsTool));

        // Notification tool
        self.register(Box::new(notify::NotifyTool));

        // Scheduling tool
        self.register(Box::new(cron::CronScheduleTool));

        // Browser scraping tool
        self.register(Box::new(browser::WebScrapeTool::new()));

        // Desktop app automation tool
        self.register(Box::new(app_interact::AppInteractTool::new()));

        // Git tool
        self.register(Box::new(git::GitTool::new()));

        // Test runner tool
        self.register(Box::new(testing::TestRunnerTool::new()));

        // Clipboard tool
        self.register(Box::new(clipboard::ClipboardTool::new()));

        // Code search tool
        self.register(Box::new(code_search::CodeSearchTool::new()));

        // LSP tool
        self.register(Box::new(lsp::LspTool::new()));

        // Sub-agent spawning tool
        self.register(Box::new(sub_agent::AgentSpawnTool::new()));

        // REPL tool
        self.register(Box::new(repl::ReplTool::new()));

        // Debugger tool
        self.register(Box::new(debugger::DebuggerTool::new()));

        // Session tools (require database access)
        if let Some(db) = db {
            self.register(Box::new(sessions::SessionListTool::new(db.clone())));
            self.register(Box::new(sessions::SessionHistoryTool::new(db)));
        }
    }

    fn register(&mut self, tool: Box<dyn NativeTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get tool schemas for all registered native tools, formatted for the LLM.
    pub fn get_all_schemas(&self) -> Vec<ToolSchema> {
        self.tools
            .values()
            .map(|tool| ToolSchema {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters_schema(),
                required_permission: Some(tool.required_capability().capability_key().to_string()),
            })
            .collect()
    }

    /// Pre-approve all native tool capabilities for a given caller.
    ///
    /// This populates the PolicyEngine's session cache so that native tools
    /// are not blocked by the default deny policy during chat agent execution.
    pub async fn pre_approve_all(&self, caller_id: &str) {
        for tool in self.tools.values() {
            let cap = tool.required_capability();
            self.policy_engine.grant_session_cache(caller_id, &cap).await;
        }
    }

    /// Check if a tool name belongs to a native tool.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Execute a native tool by name, with permission checking.
    /// Returns the tool result or an error if permission is denied.
    pub async fn execute(
        &self,
        tool_name: &str,
        params: serde_json::Value,
        caller_id: &str,
    ) -> Result<serde_json::Value> {
        let tool = self
            .tools
            .get(tool_name)
            .ok_or_else(|| LlmError::ToolNotFound(tool_name.to_string()))?;

        // Check permission
        let capability = tool.required_capability();
        let decision = self.policy_engine.check_sync(caller_id, &capability);

        match decision {
            PermissionDecision::Allow => {}
            PermissionDecision::Deny { reason } => {
                return Err(LlmError::PermissionDenied(reason));
            }
            PermissionDecision::Prompt { .. } => {
                return Err(LlmError::PermissionDenied(format!(
                    "Tool '{}' requires user approval for '{}'",
                    tool_name, capability
                )));
            }
        }

        tool.execute(params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omni_permissions::policy::DefaultPolicy;

    fn test_registry(default_policy: DefaultPolicy) -> NativeToolRegistry {
        let db = Arc::new(Mutex::new(
            Database::open(
                &std::env::temp_dir().join("test_native_tools.db"),
                "test-key",
            )
            .unwrap(),
        ));
        let policy = Arc::new(PolicyEngine::new(db.clone(), default_policy));
        NativeToolRegistry::new_with_db(policy, db)
    }

    fn test_registry_no_db(default_policy: DefaultPolicy) -> NativeToolRegistry {
        let db = Arc::new(Mutex::new(
            Database::open(
                &std::env::temp_dir().join("test_native_tools_nodb.db"),
                "test-key",
            )
            .unwrap(),
        ));
        let policy = Arc::new(PolicyEngine::new(db, default_policy));
        NativeToolRegistry::new(policy)
    }

    #[test]
    fn test_all_builtin_tools_registered() {
        let registry = test_registry(DefaultPolicy::Deny);

        // Core system tools (7)
        assert!(registry.has_tool("exec"));
        assert!(registry.has_tool("read_file"));
        assert!(registry.has_tool("write_file"));
        assert!(registry.has_tool("edit_file"));
        assert!(registry.has_tool("list_files"));
        assert!(registry.has_tool("apply_patch"));
        assert!(registry.has_tool("grep_search"));

        // Web tools (2)
        assert!(registry.has_tool("web_fetch"));
        assert!(registry.has_tool("web_search"));

        // Memory tools (3)
        assert!(registry.has_tool("memory_save"));
        assert!(registry.has_tool("memory_search"));
        assert!(registry.has_tool("memory_get"));

        // Image tool (1)
        assert!(registry.has_tool("image_analyze"));

        // Messaging tools (2)
        assert!(registry.has_tool("send_message"));
        assert!(registry.has_tool("list_channels"));

        // Notification tool (1)
        assert!(registry.has_tool("notify"));

        // Scheduling tool (1)
        assert!(registry.has_tool("cron_schedule"));

        // Browser scraping tool (1)
        assert!(registry.has_tool("web_scrape"));

        // App automation tool (1)
        assert!(registry.has_tool("app_interact"));

        // Session tools (2, requires db)
        assert!(registry.has_tool("session_list"));
        assert!(registry.has_tool("session_history"));

        // Git tool (1)
        assert!(registry.has_tool("git"));

        // Test runner tool (1)
        assert!(registry.has_tool("test_runner"));

        // Clipboard tool (1)
        assert!(registry.has_tool("clipboard"));

        // Code search tool (1)
        assert!(registry.has_tool("code_search"));

        // LSP tool (1)
        assert!(registry.has_tool("lsp"));

        // Sub-agent tool (1)
        assert!(registry.has_tool("agent_spawn"));

        // REPL tool (1)
        assert!(registry.has_tool("repl"));

        // Debugger tool (1)
        assert!(registry.has_tool("debugger"));

        // Nonexistent
        assert!(!registry.has_tool("nonexistent"));
    }

    #[test]
    fn test_get_all_schemas() {
        let registry = test_registry(DefaultPolicy::Deny);
        let schemas = registry.get_all_schemas();
        // 29 total tools when db is provided
        assert_eq!(schemas.len(), 29);
        for schema in &schemas {
            assert!(!schema.name.is_empty());
            assert!(!schema.description.is_empty());
            assert!(schema.parameters.is_object());
        }
    }

    #[test]
    fn test_get_all_schemas_no_db() {
        let registry = test_registry_no_db(DefaultPolicy::Deny);
        let schemas = registry.get_all_schemas();
        // 27 tools without db (no session_list/session_history)
        assert_eq!(schemas.len(), 27);
    }

    #[tokio::test]
    async fn test_permission_denied() {
        let registry = test_registry(DefaultPolicy::Deny);
        let result = registry
            .execute("exec", serde_json::json!({"command": "echo hello"}), "test-caller")
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Permission denied") || err.contains("permission"),
            "Expected permission error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = test_registry(DefaultPolicy::Deny);
        let result = registry
            .execute("nonexistent_tool", serde_json::json!({}), "test-caller")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
