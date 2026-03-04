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

You are Omni, a personal AI assistant with deep system integration. You can read and write files, \
run commands, search code, control desktop applications, send messages across platforms, browse the \
web, run tests, debug programs, and much more. You operate through structured tool calls -- always \
use the appropriate tool rather than asking the user to perform actions manually.

Be direct and helpful. When a task requires multiple steps, execute them yourself rather than \
listing instructions. Explain what you're doing briefly, then do it."
            .to_string()
    }

    fn build_tool_guidance(&self, tools: &[ToolSchema]) -> String {
        // Collect which tool categories are actually available
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        let mut guide = String::from("# Tool Usage Guide\n\n");
        guide.push_str(
            "Below is guidance on when and how to use each tool. Tools are grouped by category \
             with selection guidance for choosing between related tools.\n",
        );

        // ── File Operations ──
        if tool_names.iter().any(|n| {
            matches!(
                *n,
                "read_file"
                    | "write_file"
                    | "edit_file"
                    | "list_files"
                    | "apply_patch"
                    | "grep_search"
            )
        }) {
            guide.push_str("\n## File Operations\n\n");
            guide.push_str(
                "**Selection guide**: Use `read_file` to understand code before modifying it. \
                 Use `edit_file` for single, targeted changes (one replacement per call). \
                 Use `apply_patch` when making multiple changes to the same file in one operation. \
                 Use `write_file` only for new files or complete rewrites. \
                 Use `grep_search` to locate code before reading/editing. \
                 Use `list_files` to explore directory structure.\n\n",
            );

            if tool_names.contains(&"read_file") {
                guide.push_str(
                    "- **read_file**: Read file contents. Always read a file before editing it. \
                     Use `offset` and `limit` for large files. Returns base64 for binary files.\n",
                );
            }
            if tool_names.contains(&"write_file") {
                guide.push_str(
                    "- **write_file**: Create a new file or completely overwrite an existing one. \
                     Prefer `edit_file` for partial modifications -- `write_file` replaces the entire file.\n",
                );
            }
            if tool_names.contains(&"edit_file") {
                guide.push_str(
                    "- **edit_file**: Replace an exact string in a file with new content. \
                     The `old_string` must appear exactly once in the file (include enough surrounding \
                     context to make it unique). For multiple changes, make separate `edit_file` calls or \
                     use `apply_patch` instead.\n",
                );
            }
            if tool_names.contains(&"list_files") {
                guide.push_str(
                    "- **list_files**: List directory contents with names, sizes, and types. \
                     Supports recursive listing (max depth 3). Use this to understand project structure.\n",
                );
            }
            if tool_names.contains(&"apply_patch") {
                guide.push_str(
                    "- **apply_patch**: Apply a unified diff to a file. Best for making \
                     multiple changes to the same file in a single operation. The patch must use \
                     standard unified diff format with correct line numbers.\n",
                );
            }
            if tool_names.contains(&"grep_search") {
                guide.push_str(
                    "- **grep_search**: Search file contents with regex patterns. Returns matching lines \
                     with file paths and line numbers. Use this to locate code, find references, or search \
                     across an entire project. Use `glob` to filter by file type (e.g., `*.rs`, `*.py`).\n",
                );
            }
        }

        // ── Code Intelligence ──
        if tool_names
            .iter()
            .any(|n| matches!(*n, "code_search" | "lsp"))
        {
            guide.push_str("\n## Code Intelligence\n\n");
            guide.push_str(
                "**Selection guide**: Use `grep_search` for quick text/regex searches. \
                 Use `code_search` for symbol-aware searches (function definitions, class hierarchies, \
                 import graphs) -- it understands code structure. Use `lsp` for real-time type information, \
                 go-to-definition, find-references, and compiler diagnostics. The LSP is the most powerful \
                 but requires starting a language server first.\n\n",
            );

            if tool_names.contains(&"code_search") {
                guide.push_str(
                    "- **code_search**: Semantic code search powered by syntax analysis. \
                     `index` a project first, then `search` for symbols by name/type, \
                     `symbols` to list all symbols in a file, or `dependencies` to see import graphs. \
                     Works offline without a running language server.\n",
                );
            }
            if tool_names.contains(&"lsp") {
                guide.push_str(
                    "- **lsp**: Language Server Protocol integration for real-time code intelligence. \
                     `start` a server for the project's language, then use `goto_definition`, \
                     `find_references`, `hover` (type info), `diagnostics` (errors/warnings), \
                     `symbols` (search/list), and `rename_preview`. Requires a running language server \
                     (auto-detected: rust-analyzer, tsserver, pyright, gopls). Lines and columns are 1-based.\n",
                );
            }
        }

        // ── Version Control ──
        if tool_names.contains(&"git") {
            guide.push_str("\n## Version Control\n\n");
            guide.push_str(
                "- **git**: Structured git operations with parsed output. Use `status` to see the repo \
                 state before making changes. Use `diff` to review changes before committing. Use `log` to \
                 understand recent history. Use `commit` to save work (Guardian scans for secrets \
                 automatically). Use `branch`/`checkout` for branch management, `stash` to save/restore \
                 work-in-progress, `merge` to combine branches, and `show_conflict`/`resolve` for \
                 conflict resolution. Prefer this over `exec` with raw git commands -- it returns \
                 structured JSON instead of raw text.\n",
            );
        }

        // ── Testing & Debugging ──
        if tool_names
            .iter()
            .any(|n| matches!(*n, "test_runner" | "debugger" | "repl"))
        {
            guide.push_str("\n## Testing & Debugging\n\n");
            guide.push_str(
                "**Selection guide**: Use `test_runner` to run tests and parse results. \
                 Use `debugger` for interactive debugging sessions. Use `repl` for quick code \
                 experiments or to test snippets in isolation.\n\n",
            );

            if tool_names.contains(&"test_runner") {
                guide.push_str(
                    "- **test_runner**: Run tests with automatic framework detection. Supports cargo test, \
                     jest, vitest, mocha, pytest, go test, and dotnet test. Returns structured results with \
                     pass/fail counts and failure details. Use `pattern` to run specific tests. \
                     Prefer this over `exec` with raw test commands -- it parses output into structured JSON.\n",
                );
            }
            if tool_names.contains(&"debugger") {
                guide.push_str(
                    "- **debugger**: Interactive debugging via DAP (Debug Adapter Protocol). \
                     `launch` a program or `attach` to a running process. Set `breakpoint`s, \
                     step through code (`step_over`/`step_into`/`step_out`), inspect `variables`, \
                     `evaluate` expressions, and view `stacktrace`. Auto-detects debug adapters \
                     (debugpy for Python, codelldb for Rust, node --inspect for Node.js).\n",
                );
            }
            if tool_names.contains(&"repl") {
                guide.push_str(
                    "- **repl**: Persistent REPL sessions for Python or JavaScript. \
                     State (variables, imports) persists between `execute` calls within the same session. \
                     `start` a session, `execute` code multiple times, then `stop` when done. \
                     Max 3 concurrent sessions.\n",
                );
            }
        }

        // ── Web & Research ──
        if tool_names
            .iter()
            .any(|n| matches!(*n, "web_fetch" | "web_search" | "web_scrape"))
        {
            guide.push_str("\n## Web & Research\n\n");
            guide.push_str(
                "**Selection guide**: Use `web_search` to find information or URLs. \
                 Use `web_fetch` to retrieve a specific URL (APIs, documentation pages). \
                 Use `web_scrape` for heavy-duty content extraction, JavaScript-rendered pages, \
                 or multi-page crawling.\n\n",
            );

            if tool_names.contains(&"web_search") {
                guide.push_str(
                    "- **web_search**: Search the web and get results with titles, URLs, and snippets. \
                     Use this when you need to find information, documentation, or solutions online.\n",
                );
            }
            if tool_names.contains(&"web_fetch") {
                guide.push_str(
                    "- **web_fetch**: Make HTTP requests (GET, POST, PUT, DELETE, PATCH, HEAD). \
                     Returns status code and response body. Use for reading web pages, calling APIs, \
                     downloading documentation. Supports custom headers and request bodies.\n",
                );
            }
            if tool_names.contains(&"web_scrape") {
                guide.push_str(
                    "- **web_scrape**: Extract content from web pages. Three modes: \
                     `extract` (fast HTML parsing, no browser -- good for most sites), \
                     `browser` (headless browser with anti-bot stealth -- for JS-heavy or protected sites), \
                     `crawl` (follow links across multiple pages with depth limits). \
                     Returns clean markdown/text. Use CSS `selectors` to target specific content.\n",
                );
            }
        }

        // ── Communication ──
        if tool_names
            .iter()
            .any(|n| matches!(*n, "send_message" | "list_channels" | "notify"))
        {
            guide.push_str("\n## Communication\n\n");
            guide.push_str(
                "**Selection guide**: Use `list_channels` first to discover available channels and their \
                 connection status. Use `send_message` to deliver a message through a connected channel. \
                 Use `notify` for local system notifications (not external messaging).\n\n\
                 Omni supports 21+ messaging platforms: Discord, Telegram, Slack, WhatsApp, Signal, \
                 Matrix, IRC, Microsoft Teams, Google Chat, LINE, Mattermost, Twitch, iMessage, \
                 BlueBubbles, Nostr, Feishu, Nextcloud Talk, Synology Chat, Zalo, Urbit, WebChat, \
                 and Twitter/X. Channels use compound keys in the format `type:instance` \
                 (e.g., `discord:production`, `telegram:default`). Always call `list_channels` \
                 to get the correct `channel_id` values -- never guess them.\n\n",
            );

            if tool_names.contains(&"list_channels") {
                guide.push_str(
                    "- **list_channels**: Returns all channel instances with their `id` (compound key), \
                     `channel_type`, `instance_id`, `name`, `status` (connected/disconnected/connecting/error), \
                     and `features` (direct_messages, group_messages, media_attachments, reactions, \
                     read_receipts, typing_indicators). Only channels with status `connected` can send messages. \
                     Always call this before `send_message`.\n",
                );
            }
            if tool_names.contains(&"send_message") {
                guide.push_str(
                    "- **send_message**: Send a message through a connected channel. Requires `channel_id` \
                     (compound key from `list_channels`), `recipient`, and `text`. The recipient format \
                     varies by channel type:\n\
                     \x20\x20- **Discord**: numeric channel/user ID (e.g., `\"123456789\"`)\n\
                     \x20\x20- **Telegram**: numeric chat ID (e.g., `\"123456\"` or `\"-100123456\"` for groups)\n\
                     \x20\x20- **Slack**: channel or user ID (e.g., `\"C0123456789\"`, `\"U0123456789\"`)\n\
                     \x20\x20- **WhatsApp/Signal**: phone number (e.g., `\"+15551234567\"`)\n\
                     \x20\x20- **Matrix**: room ID (e.g., `\"!room_id:matrix.org\"`)\n\
                     \x20\x20- **IRC**: channel name or nickname (e.g., `\"#general\"`, `\"username\"`)\n\
                     \x20\x20- **Teams**: conversation ID\n\
                     \x20\x20- **iMessage/BlueBubbles**: chat GUID\n\
                     \x20\x20Always confirm with the user before sending. Messages to wrong recipients \
                     cannot be unsent.\n",
                );
            }
            if tool_names.contains(&"notify") {
                guide.push_str(
                    "- **notify**: Show a local system notification (toast/alert) to the user. \
                     Use for task completion notices, important alerts, or time-sensitive information. \
                     This does NOT send messages to external platforms -- use `send_message` for that. \
                     Set `urgency` to 'critical' only for genuinely urgent matters.\n",
                );
            }
        }

        // ── Memory & Sessions ──
        if tool_names.iter().any(|n| {
            matches!(
                *n,
                "memory_save"
                    | "memory_search"
                    | "memory_get"
                    | "session_list"
                    | "session_history"
            )
        }) {
            guide.push_str("\n## Memory & Sessions\n\n");

            if tool_names.contains(&"memory_save") {
                guide.push_str(
                    "- **memory_save**: Save information to persistent memory for recall across sessions. \
                     Use `tags` for categorization and `category` to organize into separate files. \
                     Save important facts, user preferences, project decisions, and learned context.\n",
                );
            }
            if tool_names.contains(&"memory_search") {
                guide.push_str(
                    "- **memory_search**: Search persistent memory by keywords. \
                     Use this to recall previously saved information, user preferences, or project context.\n",
                );
            }
            if tool_names.contains(&"memory_get") {
                guide.push_str(
                    "- **memory_get**: Read a memory file directly (MEMORY.md or memory/<category>.md). \
                     Use this when you know the specific file to read, rather than searching.\n",
                );
            }
            if tool_names.contains(&"session_list") {
                guide.push_str(
                    "- **session_list**: List recent conversation sessions with their IDs and timestamps. \
                     Use this to find a session ID, then use `session_history` to retrieve its messages.\n",
                );
            }
            if tool_names.contains(&"session_history") {
                guide.push_str(
                    "- **session_history**: Retrieve messages from a past conversation session. \
                     Requires a `session_id` from `session_list`. Use to recall previous conversations.\n",
                );
            }
        }

        // ── System & Automation ──
        if tool_names
            .iter()
            .any(|n| matches!(*n, "exec" | "clipboard" | "app_interact" | "cron_schedule"))
        {
            guide.push_str("\n## System & Automation\n\n");

            if tool_names.contains(&"exec") {
                guide.push_str(
                    "- **exec**: Execute a shell command. Returns stdout, stderr, and exit code. \
                     Prefer dedicated tools when available (`git` over `exec git ...`, `test_runner` \
                     over `exec cargo test`, etc.) -- dedicated tools return structured results. \
                     Use `exec` for commands that have no dedicated tool.\n",
                );
            }
            if tool_names.contains(&"clipboard") {
                guide.push_str(
                    "- **clipboard**: Read from or write to the system clipboard. \
                     `read` gets current clipboard text; `write` sets it.\n",
                );
            }
            if tool_names.contains(&"app_interact") {
                guide.push_str(
                    "- **app_interact**: Automate desktop applications via UI Automation. \
                     `launch` apps, `find_element`/`find_elements` to locate UI controls, \
                     `click`/`type_text`/`read_text` to interact, `get_tree`/`get_subtree` to inspect UI structure, \
                     and `screenshot` to capture window contents. Use `list_windows` to find open windows. \
                     Password fields are blocked for security. Use `automation_id` when available \
                     -- it's the most reliable element identifier.\n",
                );
            }
            if tool_names.contains(&"cron_schedule") {
                guide.push_str(
                    "- **cron_schedule**: Create recurring scheduled tasks. \
                     `add` a task with a cron expression and a task description that the agent will \
                     execute on each trigger. `list` active schedules or `remove` them by job ID. \
                     Standard cron syntax: minute hour day-of-month month day-of-week.\n",
                );
            }
        }

        // ── Image Analysis ──
        if tool_names.contains(&"image_analyze") {
            guide.push_str("\n## Image Analysis\n\n");
            guide.push_str(
                "- **image_analyze**: Analyze an image file using AI vision. \
                 Reads an image from disk and describes its contents. Use a custom `prompt` to \
                 focus the analysis (e.g., \"What error is shown in this screenshot?\"). \
                 Supports PNG, JPEG, GIF, and WebP.\n",
            );
        }

        // ── Sub-Agents ──
        if tool_names.contains(&"agent_spawn") {
            guide.push_str("\n## Sub-Agents\n\n");
            guide.push_str(
                "- **agent_spawn**: Spawn a sub-agent to handle a task in parallel. \
                 Sub-agents get their own conversation and tool access (except `agent_spawn` to prevent \
                 recursion). Use for independent tasks: writing tests while refactoring, researching \
                 while coding, analyzing multiple files simultaneously. Set `wait: true` to get the \
                 result immediately, or `wait: false` for background work. Provide `context_files` \
                 so the sub-agent has the information it needs.\n",
            );
        }

        // ── MCP Tools ──
        let mcp_tools: Vec<&ToolSchema> = tools
            .iter()
            .filter(|t| t.name.starts_with("mcp_"))
            .collect();
        if !mcp_tools.is_empty() {
            guide.push_str("\n## MCP Tools (External Servers)\n\n");
            guide.push_str(
                "Tools prefixed with `mcp_` come from external MCP (Model Context Protocol) servers. \
                 They follow the pattern `mcp_<server>_<tool>`. These tools extend your capabilities \
                 with external integrations (databases, APIs, specialized services). Use them as \
                 documented in their descriptions.\n",
            );
        }

        // ── Flowchart Tools ──
        let flowchart_tools: Vec<&ToolSchema> = tools
            .iter()
            .filter(|t| t.name.starts_with("flow."))
            .collect();
        if !flowchart_tools.is_empty() {
            guide.push_str("\n## Flowchart Tools (Visual Extensions)\n\n");
            guide.push_str(
                "Tools prefixed with `flow.` (e.g., `flow.my-workflow.process`) are visual flowchart \
                 extensions built with the drag-and-drop flowchart editor. They execute a graph of \
                 connected nodes (LLM calls, HTTP requests, conditions, loops, sub-flows, etc.) as \
                 a single operation. Flowchart tools can:\n\
                 - Chain multiple LLM calls with intermediate processing\n\
                 - Branch on conditions and switch on values\n\
                 - Call native tools, MCP tools, and other flowcharts (sub-flows)\n\
                 - Access channels, send messages, and make HTTP requests\n\
                 - Run within a security sandbox (Guardian scanning, permission checks)\n\n\
                 Use flowchart tools when they match the task -- they encapsulate complex multi-step \
                 workflows into a single tool call. The tool's parameters and description explain \
                 what it does.\n",
            );
        }

        // ── Extension Tools (WASM) ──
        let ext_tools: Vec<&ToolSchema> = tools
            .iter()
            .filter(|t| t.name.contains('.') && !t.name.starts_with("mcp_") && !t.name.starts_with("flow."))
            .collect();
        if !ext_tools.is_empty() {
            guide.push_str("\n## Extension Tools (WASM)\n\n");
            guide.push_str(
                "Tools containing a dot (e.g., `extension-id.tool-name`) that don't start with `flow.` \
                 come from installed WASM extensions. They run in a sandboxed environment with their own \
                 permissions. Each extension declares the capabilities it needs (network access, channel send, \
                 AI inference, etc.) -- if a capability is denied, the tool call will fail with a \
                 permission error. Check the Environment section for each extension's granted \
                 permissions and channel bindings.\n",
            );
        }

        guide
    }

    fn build_workflow_patterns(&self) -> String {
        "# Workflow Patterns

Common multi-step workflows -- follow these patterns for best results:

**Fix a bug**: `grep_search` (find the code) → `read_file` (understand context) → `edit_file` (apply fix) → `test_runner` (verify the fix)

**Understand a codebase**: `list_files` (project structure) → `grep_search` / `code_search` (find relevant code) → `read_file` (read key files) → `memory_save` (save findings for later)

**Add a feature**: `grep_search` (find where to add) → `read_file` (understand existing code) → `edit_file` or `write_file` (implement) → `test_runner` (test) → `git commit` (save work)

**Refactor safely**: `git status` (ensure clean state) → `code_search` / `lsp find_references` (find all usages) → `edit_file` (make changes) → `test_runner` (verify nothing broke) → `git commit`

**Research and implement**: `web_search` (find approach) → `web_fetch` / `web_scrape` (read documentation) → implement → `test_runner` (verify)

**Debug a failure**: `test_runner run` (reproduce) → `read_file` (examine failing code) → `debugger launch` / `repl execute` (investigate) → `edit_file` (fix) → `test_runner` (confirm fix)

**Send a message**: `list_channels` (find available channels, verify status is 'connected') → identify correct recipient format for the channel type → confirm with user (recipient, channel, and message content) → `send_message` (send)"
            .to_string()
    }

    fn build_safety_rules(&self) -> String {
        "# Safety & Security

- **Read before writing**: Always read a file before editing it. Never blindly overwrite files.
- **Confirm destructive actions**: Ask the user before deleting files, force-pushing, dropping data, or other irreversible operations.
- **No secrets in commits**: Never commit credentials, API keys, tokens, or passwords. The Guardian system scans for these automatically, but be proactive.
- **Prefer dedicated tools**: Use `git` over `exec git ...`, `test_runner` over `exec cargo test`, etc. Dedicated tools are safer and return structured data.
- **Respect permissions**: If a tool call is denied, explain why it might have been denied and ask the user to adjust permissions if needed. Don't retry the same denied action.
- **Validate before executing**: When running shell commands via `exec`, ensure inputs are properly sanitized. Avoid shell injection risks.
- **Messaging caution**: Always confirm with the user before sending messages to external recipients via `send_message`. Don't send messages on the user's behalf without explicit approval. Verify the channel is connected (status 'connected') and use the correct recipient format for the channel type. Messages cannot be unsent once delivered.
- **Password fields**: The `app_interact` tool blocks interaction with password fields for security. Don't try to work around this."
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
        assert!(prompt.contains("# Tool Usage Guide"));
        assert!(prompt.contains("## File Operations"));
        assert!(prompt.contains("read_file"));
        assert!(prompt.contains("edit_file"));
    }

    #[test]
    fn test_prompt_contains_workflow_patterns() {
        let prompt = default_system_prompt(&sample_tools());
        assert!(prompt.contains("# Workflow Patterns"));
        assert!(prompt.contains("Fix a bug"));
    }

    #[test]
    fn test_prompt_contains_safety_rules() {
        let prompt = default_system_prompt(&sample_tools());
        assert!(prompt.contains("# Safety & Security"));
        assert!(prompt.contains("Read before writing"));
        assert!(prompt.contains("No secrets in commits"));
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
        assert!(!prompt.contains("## File Operations"));
        // Should contain system section (exec is there)
        assert!(prompt.contains("## System & Automation"));
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
        assert!(prompt.contains("## MCP Tools (External Servers)"));
        assert!(prompt.contains("mcp_<server>_<tool>"));
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
        assert!(prompt.contains("## Extension Tools"));
        assert!(prompt.contains("extension-id.tool-name"));
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
        assert!(!prompt.contains("# Tool Usage Guide"));
        assert!(!prompt.contains("# Workflow Patterns"));
        // Identity and safety should still be present
        assert!(prompt.contains("# Identity"));
        assert!(prompt.contains("# Safety & Security"));
    }

    #[test]
    fn test_empty_runtime_context_no_section() {
        let tools = sample_tools();
        let prompt = SystemPromptBuilder::new().build(&tools);
        assert!(!prompt.contains("# Environment"));
    }

    #[test]
    fn test_all_29_tools_covered() {
        // Create all 29 tools and verify every category appears
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

        // Verify all categories appear
        assert!(prompt.contains("## File Operations"), "Missing File Operations");
        assert!(
            prompt.contains("## Code Intelligence"),
            "Missing Code Intelligence"
        );
        assert!(
            prompt.contains("## Version Control"),
            "Missing Version Control"
        );
        assert!(
            prompt.contains("## Testing & Debugging"),
            "Missing Testing & Debugging"
        );
        assert!(prompt.contains("## Web & Research"), "Missing Web & Research");
        assert!(
            prompt.contains("## Communication"),
            "Missing Communication"
        );
        assert!(
            prompt.contains("## Memory & Sessions"),
            "Missing Memory & Sessions"
        );
        assert!(
            prompt.contains("## System & Automation"),
            "Missing System & Automation"
        );
        assert!(
            prompt.contains("## Image Analysis"),
            "Missing Image Analysis"
        );
        assert!(prompt.contains("## Sub-Agents"), "Missing Sub-Agents");

        // Verify every tool is mentioned by name in the guidance
        for name in &all_tool_names {
            assert!(
                prompt.contains(&format!("**{}**", name)),
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
