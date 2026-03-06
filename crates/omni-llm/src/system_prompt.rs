//! System prompt builder for the Omni agent.
//!
//! Constructs a comprehensive system message that tells the LLM:
//! - Who it is (identity)
//! - What tools are available and when to use each one
//! - Workflow patterns for common tasks
//! - Safety and security rules
//! - Dynamic context (connected channels, active extensions, etc.)

use crate::types::ToolSchema;

/// Information about a loaded extension, exposed to the system prompt.
#[derive(Debug, Clone)]
pub struct ExtensionContext {
    /// Extension ID (e.g., "com.example.chatbot").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Tool names this extension provides.
    pub tools: Vec<String>,
    /// Capability keys declared in the manifest (e.g., "channel.send", "ai.inference", "network.http").
    pub permissions: Vec<String>,
    /// Channel instances this extension is bound to (e.g., "discord:production").
    /// Empty means unrestricted (can send to any channel if it has `channel.send` permission).
    pub bound_channels: Vec<String>,
}

/// Context about the runtime environment, injected dynamically.
#[derive(Debug, Clone, Default)]
pub struct RuntimeContext {
    /// Names of connected messaging channels (e.g., "discord:production", "telegram:default").
    pub connected_channels: Vec<String>,
    /// Loaded extensions with their permissions and channel bindings.
    pub loaded_extensions: Vec<ExtensionContext>,
    /// Names of connected MCP servers with their tools.
    pub mcp_servers: Vec<(String, Vec<String>)>,
    /// The current working directory or project path.
    pub working_directory: Option<String>,
    /// The operating system (e.g., "Windows 11", "macOS 14", "Linux").
    pub os: Option<String>,
}

/// Builds the system prompt from components.
pub struct SystemPromptBuilder {
    /// User-provided custom instructions (appended to the default prompt).
    custom_instructions: Option<String>,
    /// Whether to include the default tool guidance.
    include_tool_guidance: bool,
    /// Runtime context for dynamic sections.
    runtime_context: RuntimeContext,
}

impl SystemPromptBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            custom_instructions: None,
            include_tool_guidance: true,
            runtime_context: RuntimeContext::default(),
        }
    }

    /// Set custom instructions from the user's config.
    pub fn with_custom_instructions(mut self, instructions: String) -> Self {
        self.custom_instructions = Some(instructions);
        self
    }

    /// Set runtime context for dynamic prompt sections.
    pub fn with_runtime_context(mut self, context: RuntimeContext) -> Self {
        self.runtime_context = context;
        self
    }

    /// Disable the default tool guidance (for users who want full control).
    #[allow(dead_code)]
    pub fn without_tool_guidance(mut self) -> Self {
        self.include_tool_guidance = false;
        self
    }

    /// Build the complete system prompt.
    /// `available_tools` is the list of tool schemas the agent will have access to.
    pub fn build(&self, available_tools: &[ToolSchema]) -> String {
        let mut sections = Vec::new();

        // 1. Identity
        sections.push(self.build_identity());

        // 2. Tool guidance (the main section)
        if self.include_tool_guidance {
            sections.push(self.build_tool_guidance(available_tools));
        }

        // 3. Workflow patterns
        if self.include_tool_guidance {
            sections.push(self.build_workflow_patterns());
        }

        // 4. Safety rules
        sections.push(self.build_safety_rules());

        // 5. Dynamic runtime context
        let runtime = self.build_runtime_context();
        if !runtime.is_empty() {
            sections.push(runtime);
        }

        // 6. Custom instructions (always last, so they can override)
        if let Some(ref custom) = self.custom_instructions {
            sections.push(format!("# Custom Instructions\n\n{}", custom));
        }

        sections.join("\n\n")
    }

    fn build_identity(&self) -> String {
        "# Identity

You are Omni, a personal AI assistant with deep system integration. \
Operate through tool calls — never ask the user to do things manually. \
Be direct: explain briefly, then act."
            .to_string()
    }

    fn build_tool_guidance(&self, tools: &[ToolSchema]) -> String {
        // Compact tool guidance — tool descriptions are already in the schema,
        // so we only need brief selection hints here.
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        let mut guide = String::from("# Tool Selection Guide\n\n\
            Each tool's full description is in its schema. Key selection rules:\n");

        // File ops
        if tool_names.iter().any(|n| matches!(*n, "read_file" | "edit_file" | "write_file")) {
            guide.push_str(
                "- **Files**: `read_file` before editing. `edit_file` for targeted changes. \
                 `write_file` only for new files. `apply_patch` for multi-change diffs. \
                 `grep_search` to find code first.\n",
            );
        }

        // Code intelligence
        if tool_names.iter().any(|n| matches!(*n, "code_search" | "lsp")) {
            guide.push_str(
                "- **Code intel**: `grep_search` for text search, `code_search` for symbols, \
                 `lsp` for types/definitions/references.\n",
            );
        }

        // Git
        if tool_names.contains(&"git") {
            guide.push_str("- **Git**: Use `git` tool (structured JSON) over `exec git ...`.\n");
        }

        // Testing
        if tool_names.iter().any(|n| matches!(*n, "test_runner" | "debugger" | "repl")) {
            guide.push_str(
                "- **Testing**: `test_runner` for tests, `debugger` for DAP debugging, `repl` for code experiments.\n",
            );
        }

        // Web
        if tool_names.iter().any(|n| matches!(*n, "web_search" | "web_fetch" | "web_scrape")) {
            guide.push_str(
                "- **Web**: `web_search` to find info, `web_fetch` for specific URLs, \
                 `web_scrape` for JS-rendered pages or crawling.\n",
            );
        }

        // Communication
        if tool_names.iter().any(|n| matches!(*n, "send_message" | "list_channels")) {
            guide.push_str(
                "- **Messaging**: Call `list_channels` first. Channels use `type:instance` keys. \
                 Confirm with user before `send_message`.\n",
            );
        }

        // System
        if tool_names.contains(&"app_interact") {
            guide.push_str(
                "- **Desktop automation**: `app_interact` for UI automation. Include `process_name` \
                 for browsers. Use `screenshot` for visual context.\n",
            );
        }

        if tool_names.contains(&"exec") {
            guide.push_str("- **Shell**: `exec` for commands without a dedicated tool.\n");
        }

        // MCP / Extension / Flowchart (dynamic tools)
        let has_mcp = tools.iter().any(|t| t.name.starts_with("mcp_"));
        let has_flow = tools.iter().any(|t| t.name.starts_with("flow."));
        let has_ext = tools.iter().any(|t| t.name.contains('.') && !t.name.starts_with("mcp_") && !t.name.starts_with("flow."));
        if has_mcp || has_flow || has_ext {
            guide.push_str("- **External**: ");
            if has_mcp { guide.push_str("`mcp_*` = MCP server tools. "); }
            if has_flow { guide.push_str("`flow.*` = flowchart workflows. "); }
            if has_ext { guide.push_str("Dotted names = WASM extensions. "); }
            guide.push('\n');
        }

        guide
    }

    fn build_workflow_patterns(&self) -> String {
        "# Workflows
- **Bug fix**: grep_search → read_file → edit_file → test_runner
- **Feature**: grep_search → read_file → edit_file/write_file → test_runner → git commit
- **Research**: web_search → web_fetch → implement → test_runner
- **Message**: list_channels → confirm with user → send_message"
            .to_string()
    }

    fn build_safety_rules(&self) -> String {
        "# Safety
- Read files before editing. Confirm destructive actions with user.
- Never commit secrets. Prefer dedicated tools over raw `exec`.
- Confirm before sending messages. Respect permission denials."
            .to_string()
    }

    fn build_runtime_context(&self) -> String {
        let mut sections = Vec::new();

        if self.runtime_context.connected_channels.is_empty()
            && self.runtime_context.loaded_extensions.is_empty()
            && self.runtime_context.mcp_servers.is_empty()
            && self.runtime_context.working_directory.is_none()
            && self.runtime_context.os.is_none()
        {
            return String::new();
        }

        sections.push("# Environment".to_string());

        if let Some(ref os) = self.runtime_context.os {
            sections.push(format!("- **Operating system**: {}", os));
        }

        if let Some(ref wd) = self.runtime_context.working_directory {
            sections.push(format!("- **Working directory**: {}", wd));
        }

        if !self.runtime_context.connected_channels.is_empty() {
            let channels = self
                .runtime_context
                .connected_channels
                .iter()
                .map(|c| format!("`{}`", c))
                .collect::<Vec<_>>()
                .join(", ");
            sections.push(format!(
                "- **Connected channels**: {} (use `list_channels` for full status and features)",
                channels
            ));
        }

        if !self.runtime_context.loaded_extensions.is_empty() {
            sections.push("- **Loaded extensions**:".to_string());
            for ext in &self.runtime_context.loaded_extensions {
                let mut parts = vec![format!("  - `{}` ({})", ext.id, ext.name)];

                if !ext.tools.is_empty() {
                    parts.push(format!("    - Tools: {}", ext.tools.join(", ")));
                }

                if !ext.permissions.is_empty() {
                    parts.push(format!("    - Permissions: {}", ext.permissions.join(", ")));
                }

                if !ext.bound_channels.is_empty() {
                    let channels = ext
                        .bound_channels
                        .iter()
                        .map(|c| format!("`{}`", c))
                        .collect::<Vec<_>>()
                        .join(", ");
                    parts.push(format!("    - Bound channels: {}", channels));
                } else if ext.permissions.iter().any(|p| p == "channel.send") {
                    parts.push(
                        "    - Bound channels: unrestricted (can send to any connected channel)"
                            .to_string(),
                    );
                }

                sections.push(parts.join("\n"));
            }
        }

        if !self.runtime_context.mcp_servers.is_empty() {
            let servers = self
                .runtime_context
                .mcp_servers
                .iter()
                .map(|(name, tools)| {
                    if tools.is_empty() {
                        format!("`{}`", name)
                    } else {
                        format!("`{}` ({} tools)", name, tools.len())
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            sections.push(format!("- **MCP servers**: {}", servers));
        }

        sections.join("\n")
    }
}

impl Default for SystemPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the default system prompt with no custom instructions or runtime context.
/// Useful for tests and simple setups.
pub fn default_system_prompt(tools: &[ToolSchema]) -> String {
    SystemPromptBuilder::new().build(tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tools() -> Vec<ToolSchema> {
        vec![
            ToolSchema {
                name: "read_file".to_string(),
                description: "Read file contents.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "write_file".to_string(),
                description: "Write file contents.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "edit_file".to_string(),
                description: "Edit file contents.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "exec".to_string(),
                description: "Execute shell command.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "git".to_string(),
                description: "Git operations.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "grep_search".to_string(),
                description: "Search files.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "test_runner".to_string(),
                description: "Run tests.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "web_search".to_string(),
                description: "Search the web.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "web_fetch".to_string(),
                description: "Fetch URL.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "send_message".to_string(),
                description: "Send a message.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "list_channels".to_string(),
                description: "List channels.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
        ]
    }

    #[test]
    fn test_default_prompt_contains_identity() {
        let prompt = default_system_prompt(&sample_tools());
        assert!(prompt.contains("You are Omni"));
        assert!(prompt.contains("personal AI assistant"));
    }

    #[test]
    fn test_prompt_contains_tool_guidance() {
        let prompt = default_system_prompt(&sample_tools());
        assert!(prompt.contains("# Tool Selection Guide"));
        assert!(prompt.contains("Files"));
        assert!(prompt.contains("read_file"));
        assert!(prompt.contains("edit_file"));
    }

    #[test]
    fn test_prompt_contains_workflow_patterns() {
        let prompt = default_system_prompt(&sample_tools());
        assert!(prompt.contains("# Workflows"));
        assert!(prompt.contains("Bug fix"));
    }

    #[test]
    fn test_prompt_contains_safety_rules() {
        let prompt = default_system_prompt(&sample_tools());
        assert!(prompt.contains("# Safety"));
        assert!(prompt.contains("Read files before editing"));
        assert!(prompt.contains("Never commit secrets"));
    }

    #[test]
    fn test_prompt_skips_missing_tool_categories() {
        // Only include exec -- no file tools, no git, etc.
        let tools = vec![ToolSchema {
            name: "exec".to_string(),
            description: "Execute command.".to_string(),
            parameters: serde_json::json!({"type": "object"}),
            required_permission: None,
        }];
        let prompt = default_system_prompt(&tools);
        // Should NOT contain file operations section
        assert!(!prompt.contains("**Files**"));
        // Should contain shell guidance (exec is there)
        assert!(prompt.contains("**Shell**"));
    }

    #[test]
    fn test_mcp_tools_section() {
        let tools = vec![
            ToolSchema {
                name: "mcp_github_search_repos".to_string(),
                description: "[MCP:github] Search repositories.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "exec".to_string(),
                description: "Execute.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
        ];
        let prompt = default_system_prompt(&tools);
        assert!(prompt.contains("mcp_*"));
        assert!(prompt.contains("MCP server tools"));
    }

    #[test]
    fn test_extension_tools_section() {
        let tools = vec![
            ToolSchema {
                name: "my-ext.summarize".to_string(),
                description: "Summarize text.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
            ToolSchema {
                name: "exec".to_string(),
                description: "Execute.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            },
        ];
        let prompt = default_system_prompt(&tools);
        assert!(prompt.contains("WASM extensions"));
    }

    #[test]
    fn test_runtime_context() {
        let tools = sample_tools();
        let ctx = RuntimeContext {
            connected_channels: vec!["discord:production".to_string()],
            loaded_extensions: vec![ExtensionContext {
                id: "summarizer".to_string(),
                name: "Summarizer".to_string(),
                tools: vec!["summarize".to_string()],
                permissions: vec!["ai.inference".to_string()],
                bound_channels: vec![],
            }],
            mcp_servers: vec![("github".to_string(), vec!["search".to_string()])],
            working_directory: Some("/home/user/project".to_string()),
            os: Some("Linux".to_string()),
        };
        let prompt = SystemPromptBuilder::new()
            .with_runtime_context(ctx)
            .build(&tools);
        assert!(prompt.contains("# Environment"));
        assert!(prompt.contains("discord:production"));
        assert!(prompt.contains("summarizer"));
        assert!(prompt.contains("Summarizer"));
        assert!(prompt.contains("ai.inference"));
        assert!(prompt.contains("github"));
        assert!(prompt.contains("/home/user/project"));
        assert!(prompt.contains("Linux"));
    }

    #[test]
    fn test_custom_instructions() {
        let tools = sample_tools();
        let prompt = SystemPromptBuilder::new()
            .with_custom_instructions("Always respond in Spanish.".to_string())
            .build(&tools);
        assert!(prompt.contains("# Custom Instructions"));
        assert!(prompt.contains("Always respond in Spanish."));
    }

    #[test]
    fn test_without_tool_guidance() {
        let tools = sample_tools();
        let prompt = SystemPromptBuilder::new()
            .without_tool_guidance()
            .build(&tools);
        assert!(!prompt.contains("# Tool Selection Guide"));
        assert!(!prompt.contains("# Workflows"));
        // Identity and safety should still be present
        assert!(prompt.contains("# Identity"));
        assert!(prompt.contains("# Safety"));
    }

    #[test]
    fn test_empty_runtime_context_no_section() {
        let tools = sample_tools();
        let prompt = SystemPromptBuilder::new().build(&tools);
        assert!(!prompt.contains("# Environment"));
    }

    #[test]
    fn test_all_tool_categories_covered() {
        // Create all 29 tools and verify key categories appear
        let all_tool_names = vec![
            "exec",
            "read_file",
            "write_file",
            "edit_file",
            "list_files",
            "apply_patch",
            "grep_search",
            "web_fetch",
            "web_search",
            "web_scrape",
            "memory_save",
            "memory_search",
            "memory_get",
            "image_analyze",
            "send_message",
            "list_channels",
            "notify",
            "cron_schedule",
            "app_interact",
            "git",
            "test_runner",
            "clipboard",
            "code_search",
            "lsp",
            "agent_spawn",
            "repl",
            "debugger",
            "session_list",
            "session_history",
        ];
        let tools: Vec<ToolSchema> = all_tool_names
            .iter()
            .map(|name| ToolSchema {
                name: name.to_string(),
                description: format!("{} tool.", name),
                parameters: serde_json::json!({"type": "object"}),
                required_permission: None,
            })
            .collect();

        let prompt = default_system_prompt(&tools);

        // Verify key category hints appear in the compact guide
        assert!(prompt.contains("**Files**"), "Missing Files guidance");
        assert!(prompt.contains("**Code intel**"), "Missing Code intel guidance");
        assert!(prompt.contains("**Git**"), "Missing Git guidance");
        assert!(prompt.contains("**Testing**"), "Missing Testing guidance");
        assert!(prompt.contains("**Web**"), "Missing Web guidance");
        assert!(prompt.contains("**Messaging**"), "Missing Messaging guidance");
        assert!(prompt.contains("**Desktop automation**"), "Missing Desktop automation guidance");
        assert!(prompt.contains("**Shell**"), "Missing Shell guidance");

        // Verify key tool names are mentioned
        for name in &["read_file", "edit_file", "grep_search", "git", "web_search",
                       "web_fetch", "list_channels", "send_message", "app_interact", "exec"] {
            assert!(
                prompt.contains(name),
                "Tool '{}' not mentioned in guidance",
                name
            );
        }
    }

    #[test]
    fn test_extension_context_with_permissions_and_bindings() {
        let tools = sample_tools();
        let ctx = RuntimeContext {
            connected_channels: vec![
                "discord:production".to_string(),
                "telegram:default".to_string(),
            ],
            loaded_extensions: vec![
                ExtensionContext {
                    id: "com.example.chatbot".to_string(),
                    name: "Chatbot".to_string(),
                    tools: vec!["respond".to_string(), "summarize".to_string()],
                    permissions: vec![
                        "channel.send".to_string(),
                        "ai.inference".to_string(),
                    ],
                    bound_channels: vec!["discord:production".to_string()],
                },
                ExtensionContext {
                    id: "com.example.analytics".to_string(),
                    name: "Analytics".to_string(),
                    tools: vec!["analyze".to_string()],
                    permissions: vec!["network.http".to_string()],
                    bound_channels: vec![],
                },
            ],
            mcp_servers: vec![],
            working_directory: None,
            os: None,
        };
        let prompt = SystemPromptBuilder::new()
            .with_runtime_context(ctx)
            .build(&tools);

        // Chatbot extension info
        assert!(prompt.contains("com.example.chatbot"));
        assert!(prompt.contains("Chatbot"));
        assert!(prompt.contains("channel.send"));
        assert!(prompt.contains("ai.inference"));
        assert!(prompt.contains("Bound channels"));
        assert!(prompt.contains("discord:production"));

        // Analytics extension info
        assert!(prompt.contains("com.example.analytics"));
        assert!(prompt.contains("Analytics"));
        assert!(prompt.contains("network.http"));

        // Analytics has no bindings and no channel.send, so no binding line
    }

    #[test]
    fn test_extension_with_channel_send_but_no_bindings() {
        let tools = sample_tools();
        let ctx = RuntimeContext {
            connected_channels: vec![],
            loaded_extensions: vec![ExtensionContext {
                id: "com.example.sender".to_string(),
                name: "Sender".to_string(),
                tools: vec![],
                permissions: vec!["channel.send".to_string()],
                bound_channels: vec![], // No bindings = unrestricted
            }],
            mcp_servers: vec![],
            working_directory: None,
            os: None,
        };
        let prompt = SystemPromptBuilder::new()
            .with_runtime_context(ctx)
            .build(&tools);

        assert!(prompt.contains("unrestricted"));
        assert!(prompt.contains("can send to any connected channel"));
    }
}
