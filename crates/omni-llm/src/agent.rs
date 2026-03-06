use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;

use omni_core::events::{EventBus, OmniEvent};
use omni_extensions::flowchart::FlowchartRegistry;
use omni_extensions::host::ExtensionHost;
use omni_guardian::Guardian;

use crate::bridge::LLMBridge;
use crate::error::{LlmError, Result};
use crate::guardian_bridge::ExtensionToolRegistry;
use crate::hooks::{HookContext, HookPoint, HookRegistry, HookResult, ToolCallInfo};
use crate::mcp::McpManager;
use crate::system_prompt::SystemPromptBuilder;
use crate::tools::NativeToolRegistry;
use crate::types::{AgentResult, ChatChunk, ChatMessage, ChatRole, ImageContent, ToolCall, ToolSchema};

/// Maximum number of tokens to spend on message history (approximate).
/// Keeps the sliding window from growing unbounded.
const MAX_HISTORY_TOKENS: usize = 32_000;

/// Maximum length (in chars) for a single tool result before truncation.
const MAX_TOOL_RESULT_CHARS: usize = 8_000;

/// Core tools that are always included in every request.
/// These are cheap (small schemas) and universally useful.
const CORE_TOOLS: &[&str] = &[
    "read_file",
    "write_file",
    "edit_file",
    "exec",
    "list_files",
    "grep_search",
];

/// Handler for delegated tool actions (send_message, list_channels, etc.).
///
/// Tools like `send_message` and `list_channels` return structured intents
/// rather than executing directly. This trait lets the caller (e.g. Tauri app)
/// provide an implementation that routes these actions to the actual services
/// (ChannelManager, scheduler, etc.) without coupling omni-llm to omni-channels.
#[async_trait]
pub trait DelegatedActionHandler: Send + Sync {
    /// Send a message via a channel plugin.
    /// Returns the actual result to pass back to the LLM.
    async fn send_message(
        &self,
        channel_id: &str,
        recipient: &str,
        text: &str,
    ) -> std::result::Result<serde_json::Value, String>;

    /// List available channels with their connection status.
    async fn list_channels(&self) -> std::result::Result<serde_json::Value, String>;

    /// Check if a send is allowed by channel-extension bindings.
    ///
    /// - If `caller_extension_id` is None (native tool call), always allowed.
    /// - If the extension has no bindings, always allowed (backward compat).
    /// - If the extension has bindings, only allowed for bound channel instances.
    ///
    /// Default implementation always allows (for backward compat with existing impls).
    async fn check_send_binding(
        &self,
        _caller_extension_id: Option<&str>,
        _channel_id: &str,
    ) -> std::result::Result<(), String> {
        Ok(())
    }

    /// List channel instances an extension is bound to.
    /// Returns empty if the extension has no bindings.
    async fn list_bindings(
        &self,
        _extension_id: &str,
    ) -> std::result::Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

/// The agent loop orchestrates multi-turn conversations where the LLM
/// can invoke tools. This is inspired by OpenClaw's agent orchestration
/// but implemented natively in Rust.
///
/// Tool resolution order:
/// 1. Native tools (exec, read_file, write_file, etc.) -- run as Rust code
/// 2. MCP tools (mcp_<server>_<tool>) -- routed to external MCP servers
/// 3. Extension tools (WASM) -- run in sandboxed WASM runtime
pub struct AgentLoop {
    llm_bridge: Arc<LLMBridge>,
    extension_host: Arc<ExtensionHost>,
    guardian: Arc<Guardian>,
    tool_registry: Arc<ExtensionToolRegistry>,
    native_tools: Arc<NativeToolRegistry>,
    hook_registry: Arc<HookRegistry>,
    mcp_manager: Option<Arc<McpManager>>,
    flowchart_registry: Option<Arc<FlowchartRegistry>>,
    action_handler: Option<Arc<dyn DelegatedActionHandler>>,
    system_prompt_builder: Option<SystemPromptBuilder>,
    event_bus: EventBus,
    session_id: Option<String>,
    provider_id: String,
    model: String,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    thinking: Option<crate::types::ThinkingMode>,
    effort: Option<crate::types::ThinkingEffort>,
}

impl AgentLoop {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        llm_bridge: Arc<LLMBridge>,
        extension_host: Arc<ExtensionHost>,
        guardian: Arc<Guardian>,
        tool_registry: Arc<ExtensionToolRegistry>,
        native_tools: Arc<NativeToolRegistry>,
        hook_registry: Arc<HookRegistry>,
        event_bus: EventBus,
        provider_id: &str,
        model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Self {
        Self {
            llm_bridge,
            extension_host,
            guardian,
            tool_registry,
            native_tools,
            hook_registry,
            mcp_manager: None,
            flowchart_registry: None,
            action_handler: None,
            system_prompt_builder: None,
            event_bus,
            session_id: None,
            provider_id: provider_id.to_string(),
            model: model.to_string(),
            max_tokens,
            temperature,
            thinking: None,
            effort: None,
        }
    }

    /// Set the session ID for correlating events emitted during the loop.
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Configure thinking mode for the agent loop.
    pub fn with_thinking(mut self, thinking: crate::types::ThinkingMode, effort: Option<crate::types::ThinkingEffort>) -> Self {
        self.thinking = Some(thinking);
        self.effort = effort;
        self
    }

    /// Set a handler for delegated tool actions (send_message, list_channels).
    /// When set, tools that return structured intents will be routed through
    /// this handler for actual execution.
    pub fn with_action_handler(mut self, handler: Arc<dyn DelegatedActionHandler>) -> Self {
        self.action_handler = Some(handler);
        self
    }

    /// Set the MCP manager for external tool server support.
    /// When set, tools from connected MCP servers are injected into the tool schema
    /// list and routed to external servers for execution.
    pub fn with_mcp_manager(mut self, mcp_manager: Arc<McpManager>) -> Self {
        self.mcp_manager = Some(mcp_manager);
        self
    }

    /// Set the flowchart registry for visual extension support.
    /// When set, tools from enabled flowcharts are injected into the tool schema
    /// list and executed via the native flowchart engine.
    pub fn with_flowchart_registry(mut self, registry: Arc<FlowchartRegistry>) -> Self {
        self.flowchart_registry = Some(registry);
        self
    }

    /// Set the system prompt builder for generating context-aware system messages.
    /// When set, a system prompt is injected at the start of new conversations,
    /// providing the LLM with identity, tool guidance, safety rules, and runtime context.
    pub fn with_system_prompt(mut self, builder: SystemPromptBuilder) -> Self {
        self.system_prompt_builder = Some(builder);
        self
    }

    /// Access the hook registry to register or unregister hooks.
    pub fn hook_registry(&self) -> &Arc<HookRegistry> {
        &self.hook_registry
    }

    /// Run the agent loop for a user message.
    ///
    /// This may involve multiple LLM calls if the model uses tools.
    /// Maximum iterations are configurable to prevent infinite loops.
    ///
    /// `trust_modifier` optionally adjusts Guardian scanning sensitivity:
    /// - `None` or `Some(1.0)` -- default thresholds (trusted/authenticated sources)
    /// - `Some(0.8)` -- 20% stricter (recommended for unauthenticated sources)
    pub async fn run(
        &self,
        messages: &mut Vec<ChatMessage>,
        user_message: &str,
        max_iterations: u32,
    ) -> Result<AgentResult> {
        self.run_with_trust(messages, user_message, max_iterations, None).await
    }

    /// Select relevant tools for the current turn.
    ///
    /// Instead of sending ALL 29+ tool schemas on every LLM call (~25K tokens),
    /// this filters to: core tools + tools already used in conversation + tools
    /// that match the current context. Extensions, MCP, and flowchart tools are
    /// always included since they're user-installed and expected.
    fn select_relevant_tools(
        all_tools: &[ToolSchema],
        messages: &[ChatMessage],
    ) -> Vec<ToolSchema> {
        // Collect names of tools already used in the conversation
        let mut used_tools: std::collections::HashSet<String> = std::collections::HashSet::new();
        for msg in messages {
            if let Some(ref tcs) = msg.tool_calls {
                for tc in tcs {
                    used_tools.insert(tc.name.clone());
                }
            }
        }

        let mut selected: Vec<ToolSchema> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for tool in all_tools {
            let dominated_name = tool.name.as_str();
            let include =
                // Always include core tools
                CORE_TOOLS.contains(&dominated_name) ||
                // Always include tools already used in this conversation
                used_tools.contains(&tool.name) ||
                // Always include extension/MCP/flowchart tools (user-installed, expected)
                tool.name.contains('.') ||
                tool.name.starts_with("mcp_") ||
                // Include app_interact (needed for desktop automation tasks)
                dominated_name == "app_interact" ||
                // Include web tools (commonly needed)
                dominated_name == "web_search" ||
                dominated_name == "web_fetch" ||
                // Include send_message and list_channels (communication)
                dominated_name == "send_message" ||
                dominated_name == "list_channels" ||
                // Include git (version control)
                dominated_name == "git" ||
                // Include test_runner
                dominated_name == "test_runner";

            if include && seen.insert(tool.name.clone()) {
                selected.push(tool.clone());
            }
        }

        // If we filtered out less than 20% of tools, just send them all —
        // the overhead of filtering isn't worth it.
        if selected.len() * 5 >= all_tools.len() * 4 {
            return all_tools.to_vec();
        }

        selected
    }

    /// Truncate message history to fit within a token budget.
    ///
    /// Strategy:
    /// 1. Always keep the system message (index 0) and the last user message.
    /// 2. Always keep tool_call + tool_result pairs that are in the most recent
    ///    agent loop iteration (needed for correct API calls).
    /// 3. For older messages, keep only the most recent ones that fit the budget.
    /// 4. Truncate individual tool results that exceed MAX_TOOL_RESULT_CHARS.
    fn truncate_history(messages: &mut Vec<ChatMessage>) {
        // First pass: truncate oversized tool results
        for msg in messages.iter_mut() {
            if msg.role == ChatRole::Tool && msg.content.len() > MAX_TOOL_RESULT_CHARS {
                let truncated = &msg.content[..MAX_TOOL_RESULT_CHARS];
                // Find last complete line or JSON boundary
                let cut_at = truncated.rfind('\n')
                    .or_else(|| truncated.rfind('}'))
                    .or_else(|| truncated.rfind(','))
                    .unwrap_or(MAX_TOOL_RESULT_CHARS);
                msg.content = format!(
                    "{}...\n[truncated: {} chars total, showing first {}]",
                    &msg.content[..cut_at],
                    msg.content.len(),
                    cut_at
                );
            }
        }

        // Estimate tokens (rough: 1 token ≈ 4 chars for English text)
        let estimate_tokens = |msgs: &[ChatMessage]| -> usize {
            msgs.iter().map(|m| {
                let base = m.content.len() / 4 + 4; // +4 per-message overhead
                let tc_tokens = m.tool_calls.as_ref().map_or(0, |tcs| {
                    tcs.iter().map(|tc| tc.arguments.len() / 4 + 10).sum()
                });
                base + tc_tokens
            }).sum()
        };

        let total_estimate = estimate_tokens(messages);
        if total_estimate <= MAX_HISTORY_TOKENS {
            return; // Fits within budget, no truncation needed
        }

        // Need to drop older messages. Strategy:
        // - Keep system message (first message if role == System)
        // - Keep the last N messages (most recent context)
        // - Drop middle messages from oldest first
        let has_system = messages.first().map_or(false, |m| m.role == ChatRole::System);
        let protected_start = if has_system { 1 } else { 0 };

        // Find how many messages from the end we can keep within budget
        // Start from the end and walk backward
        let system_tokens = if has_system {
            messages[0].content.len() / 4 + 4
        } else {
            0
        };

        let mut budget_remaining = MAX_HISTORY_TOKENS.saturating_sub(system_tokens);
        let mut keep_from = messages.len();

        for i in (protected_start..messages.len()).rev() {
            let msg_tokens = messages[i].content.len() / 4 + 4
                + messages[i].tool_calls.as_ref().map_or(0, |tcs| {
                    tcs.iter().map(|tc| tc.arguments.len() / 4 + 10).sum()
                });
            if msg_tokens > budget_remaining {
                break;
            }
            budget_remaining -= msg_tokens;
            keep_from = i;
        }

        // Don't drop anything if keep_from is already at the start
        if keep_from <= protected_start {
            return;
        }

        // Build the truncated message list
        let dropped_count = keep_from - protected_start;
        if dropped_count == 0 {
            return;
        }

        let mut new_messages = Vec::new();
        if has_system {
            new_messages.push(messages[0].clone());
        }
        // Insert a summary marker so the LLM knows history was truncated
        new_messages.push(ChatMessage::system(
            &format!("[{} older messages truncated to save context space]", dropped_count),
        ));
        new_messages.extend_from_slice(&messages[keep_from..]);

        let remaining = new_messages.len();
        *messages = new_messages;

        tracing::info!(
            dropped = dropped_count,
            remaining = remaining,
            "Truncated message history to fit token budget"
        );
    }

    /// Run the agent loop with an explicit trust-level threshold modifier.
    pub async fn run_with_trust(
        &self,
        messages: &mut Vec<ChatMessage>,
        user_message: &str,
        max_iterations: u32,
        trust_modifier: Option<f64>,
    ) -> Result<AgentResult> {
        // ── SP-1: Scan user input ────────────────────────────────────
        if self.guardian.is_enabled() {
            let scan = match trust_modifier {
                Some(m) if (m - 1.0).abs() > f64::EPSILON => {
                    self.guardian.scan_input_with_trust(user_message, m)
                }
                _ => self.guardian.scan_input(user_message),
            };
            if scan.blocked {
                return Err(LlmError::GuardianBlocked(
                    scan.reason.unwrap_or_else(|| "Blocked by Guardian".to_string()),
                ));
            }
        }

        // ── Hook: message_received ───────────────────────────────────
        // Hooks can modify the user input text before it enters the conversation.
        let user_message = {
            let ctx = HookContext {
                hook_point: HookPoint::MessageReceived,
                session_id: None,
                text: Some(user_message.to_string()),
                tool_call: None,
                messages: None,
                metadata: serde_json::json!({}),
            };
            match self.hook_registry.run_modifying(HookPoint::MessageReceived, ctx).await {
                HookResult::Continue(ctx) => {
                    ctx.text.unwrap_or_else(|| user_message.to_string())
                }
                HookResult::Block { reason } => {
                    return Err(LlmError::HookBlocked(reason));
                }
            }
        };

        // 1. Add user message to conversation
        messages.push(ChatMessage::user(&user_message));

        // 2. Collect available tools: native built-in tools + WASM extensions
        let native_schemas = self.native_tools.get_all_schemas();

        let extension_tools = self.extension_host.get_all_tools().await;

        // Refresh the Guardian's tool registry cache for SP-4 validation
        self.tool_registry.refresh_from(extension_tools.clone()).await;

        // Also register flowchart tools in Guardian's registry so SP-4 can validate them
        if let Some(ref fc_registry) = self.flowchart_registry {
            let fc_tools = fc_registry.get_all_tools().await;
            self.tool_registry.append_from(fc_tools).await;
        }

        let extension_schemas: Vec<ToolSchema> = extension_tools
            .iter()
            .map(|(ext_id, tool)| ToolSchema {
                name: format!("{}.{}", ext_id, tool.name),
                description: tool.description.clone(),
                parameters: tool.parameters.clone(),
                required_permission: None, // Extension-level permissions shown in RuntimeContext
            })
            .collect();

        // Collect MCP tool schemas from connected servers
        let mcp_schemas = if let Some(ref mcp) = self.mcp_manager {
            mcp.get_all_schemas().await
        } else {
            Vec::new()
        };

        // Collect flowchart tool schemas from enabled visual extensions
        let flowchart_schemas: Vec<ToolSchema> = if let Some(ref fc_registry) = self.flowchart_registry {
            fc_registry
                .get_all_tools()
                .await
                .iter()
                .map(|(fc_id, tool)| ToolSchema {
                    name: format!("{}.{}", fc_id, tool.name),
                    description: tool.description.clone(),
                    parameters: tool.parameters.clone(),
                    required_permission: None,
                })
                .collect()
        } else {
            Vec::new()
        };

        // Native tools first, then MCP, then flowcharts, then WASM extensions
        let mut all_tool_schemas = native_schemas;
        all_tool_schemas.extend(mcp_schemas);
        all_tool_schemas.extend(flowchart_schemas);
        all_tool_schemas.extend(extension_schemas);

        // ── Inject system prompt on first turn ──────────────────────────
        // If a system prompt builder is configured and there's no system
        // message yet, build and inject one. This gives the LLM identity,
        // tool guidance, safety rules, and runtime context.
        if let Some(ref builder) = self.system_prompt_builder {
            let has_system_msg = messages
                .iter()
                .any(|m| m.role == crate::types::ChatRole::System);
            if !has_system_msg {
                let system_prompt = builder.build(&all_tool_schemas);
                messages.insert(0, ChatMessage::system(&system_prompt));
            }
        }

        // ── Filter tool schemas to reduce per-request token overhead ────
        // Instead of sending all 29+ tool schemas (~25K tokens) on every
        // call, select only relevant ones based on context.
        let tool_schemas = Self::select_relevant_tools(&all_tool_schemas, messages);

        let mut iteration = 0;
        let final_text;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                tracing::warn!(iterations = iteration, "Agent loop hit max iterations");
                return Err(LlmError::MaxIterationsExceeded(max_iterations));
            }

            // ── SP-2: Scan assembled prompt before LLM call ──────────
            if self.guardian.is_enabled() && iteration == 1 {
                let prompt_text: String = messages
                    .iter()
                    .map(|m| format!("{}: {}", m.role.as_str(), m.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                let scan = self.guardian.scan_prompt_assembly(&prompt_text);
                if scan.blocked {
                    return Err(LlmError::GuardianBlocked(
                        scan.reason
                            .unwrap_or_else(|| "Prompt assembly blocked by Guardian".to_string()),
                    ));
                }
            }

            // ── Hook: llm_input ────────────────────────────────────────
            // Hooks can inspect or modify the messages before they go to the LLM.
            {
                let ctx = HookContext {
                    hook_point: HookPoint::LlmInput,
                    session_id: None,
                    text: None,
                    tool_call: None,
                    messages: Some(messages.clone()),
                    metadata: serde_json::json!({}),
                };
                match self.hook_registry.run_modifying(HookPoint::LlmInput, ctx).await {
                    HookResult::Continue(ctx) => {
                        if let Some(modified_msgs) = ctx.messages {
                            *messages = modified_msgs;
                        }
                    }
                    HookResult::Block { reason } => {
                        return Err(LlmError::HookBlocked(reason));
                    }
                }
            }

            // ── Truncate history to stay within token budget ────────────
            // Prevents unbounded growth of the message list across iterations.
            Self::truncate_history(messages);

            // 3. Call the LLM with streaming
            let mut stream = self
                .llm_bridge
                .chat_stream_with_thinking(
                    &self.provider_id,
                    messages.clone(),
                    tool_schemas.clone(),
                    &self.model,
                    self.max_tokens,
                    self.temperature,
                    self.thinking.clone(),
                    self.effort,
                )
                .await?;

            let mut assistant_text = String::new();
            let mut thinking_text = String::new();
            let mut thinking_signature = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();
            let mut accumulated_for_scan = String::new();

            // 4. Process the stream
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                match chunk {
                    ChatChunk::ThinkingDelta(text) => {
                        if !text.is_empty() {
                            thinking_text.push_str(&text);
                            self.event_bus.emit(OmniEvent::LlmThinking {
                                session_id: self.session_id.clone().unwrap_or_default(),
                                chunk: text,
                            });
                        }
                    }
                    ChatChunk::SignatureDelta(sig) => {
                        thinking_signature.push_str(&sig);
                    }
                    ChatChunk::TextDelta(text) => {
                        assistant_text.push_str(&text);
                        accumulated_for_scan.push_str(&text);

                        // ── SP-3: Scan output chunks periodically ────
                        if self.guardian.is_enabled() && accumulated_for_scan.len() >= 500 {
                            let scan =
                                self.guardian.scan_output_chunk(&accumulated_for_scan);
                            if scan.blocked {
                                return Err(LlmError::GuardianBlocked(
                                    scan.reason.unwrap_or_else(|| {
                                        "Output blocked by Guardian".to_string()
                                    }),
                                ));
                            }
                            accumulated_for_scan.clear();
                        }

                        self.event_bus.emit(OmniEvent::LlmChunk {
                            session_id: self.session_id.clone().unwrap_or_default(),
                            chunk: text,
                        });
                    }
                    ChatChunk::ToolCallDelta {
                        index,
                        id,
                        name,
                        arguments_delta,
                    } => {
                        while tool_calls.len() <= index {
                            tool_calls.push(ToolCall::default());
                        }
                        let tc = &mut tool_calls[index];
                        if let Some(id) = id {
                            tc.id = id;
                        }
                        if let Some(name) = name {
                            tc.name = name;
                        }
                        tc.arguments.push_str(&arguments_delta);
                    }
                    ChatChunk::Usage {
                        prompt_tokens,
                        completion_tokens,
                        ..
                    } => {
                        tracing::debug!(prompt_tokens, completion_tokens, "Token usage");
                    }
                    ChatChunk::Done => break,
                }
            }

            // Scan any remaining accumulated output
            if self.guardian.is_enabled() && !accumulated_for_scan.is_empty() {
                let scan = self.guardian.scan_output_chunk(&accumulated_for_scan);
                if scan.blocked {
                    return Err(LlmError::GuardianBlocked(
                        scan.reason
                            .unwrap_or_else(|| "Output blocked by Guardian".to_string()),
                    ));
                }
            }

            // ── Hook: llm_output ───────────────────────────────────────
            // Hooks can modify or filter the LLM's response text.
            {
                let ctx = HookContext {
                    hook_point: HookPoint::LlmOutput,
                    session_id: None,
                    text: Some(assistant_text.clone()),
                    tool_call: None,
                    messages: None,
                    metadata: serde_json::json!({}),
                };
                match self.hook_registry.run_modifying(HookPoint::LlmOutput, ctx).await {
                    HookResult::Continue(ctx) => {
                        if let Some(modified_text) = ctx.text {
                            assistant_text = modified_text;
                        }
                    }
                    HookResult::Block { reason } => {
                        return Err(LlmError::HookBlocked(reason));
                    }
                }
            }

            // Build thinking content for message preservation
            let thinking_content = if !thinking_text.is_empty() {
                vec![crate::types::ThinkingContent::Thinking(crate::types::ThinkingBlock {
                    thinking: thinking_text.clone(),
                    signature: thinking_signature.clone(),
                })]
            } else {
                vec![]
            };

            // 5. If no tool calls, we're done
            if tool_calls.is_empty() {
                if thinking_content.is_empty() {
                    messages.push(ChatMessage::assistant(&assistant_text));
                } else {
                    messages.push(ChatMessage::assistant_with_thinking(&assistant_text, thinking_content));
                }
                final_text = assistant_text;
                break;
            }

            // ── SP-4: Validate tool calls before invocation ──────────
            if self.guardian.is_enabled() {
                let guardian_tool_calls: Vec<omni_guardian::ToolCallInfo> = tool_calls
                    .iter()
                    .map(|tc| omni_guardian::ToolCallInfo {
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    })
                    .collect();

                let validations = self
                    .guardian
                    .validate_tool_calls(&guardian_tool_calls)
                    .await;

                for (i, validation) in validations.iter().enumerate() {
                    if let omni_guardian::ToolCallValidation::Blocked { reason } = validation {
                        tracing::warn!(
                            tool = tool_calls[i].name,
                            reason = reason.as_str(),
                            "Guardian blocked tool call"
                        );
                        return Err(LlmError::GuardianBlocked(format!(
                            "Tool call '{}' blocked: {}",
                            tool_calls[i].name, reason
                        )));
                    }
                }
            }

            // 6. Process tool calls
            if thinking_content.is_empty() {
                messages.push(ChatMessage::assistant_with_tool_calls(
                    &assistant_text,
                    tool_calls.clone(),
                ));
            } else {
                messages.push(ChatMessage::assistant_with_tool_calls_and_thinking(
                    &assistant_text,
                    tool_calls.clone(),
                    thinking_content,
                ));
            }

            for tc in &tool_calls {
                let params: serde_json::Value = serde_json::from_str(&tc.arguments)
                    .map_err(|e| LlmError::ToolCall(format!("Invalid tool arguments: {}", e)))?;

                // ── Hook: before_tool_call ─────────────────────────────
                // Hooks can block or modify tool call params.
                let params = {
                    let ctx = HookContext {
                        hook_point: HookPoint::BeforeToolCall,
                        session_id: None,
                        text: None,
                        tool_call: Some(ToolCallInfo {
                            name: tc.name.clone(),
                            arguments: serde_json::to_string(&params).unwrap_or_default(),
                            result: None,
                        }),
                        messages: None,
                        metadata: serde_json::json!({}),
                    };
                    match self.hook_registry.run_modifying(HookPoint::BeforeToolCall, ctx).await {
                        HookResult::Continue(ctx) => {
                            if let Some(ref tc_info) = ctx.tool_call {
                                serde_json::from_str(&tc_info.arguments).unwrap_or(params)
                            } else {
                                params
                            }
                        }
                        HookResult::Block { reason } => {
                            tracing::info!(
                                tool = tc.name.as_str(),
                                reason = reason.as_str(),
                                "Hook blocked tool call"
                            );
                            messages.push(ChatMessage::tool_result(
                                &tc.id,
                                &format!("Blocked by hook: {}", reason),
                            ));
                            continue;
                        }
                    }
                };

                // Try native tool first (no dots in name), then extension tool
                if self.native_tools.has_tool(&tc.name) {
                    // ── Native tool execution ─────────────────────────
                    self.event_bus.emit(OmniEvent::ExtensionInvoked {
                        extension_id: "omni.native".to_string(),
                        tool_name: tc.name.clone(),
                        params: params.clone(),
                    });

                    match self
                        .native_tools
                        .execute(&tc.name, params.clone(), "omni.native")
                        .await
                    {
                        Ok(result) => {
                            // Process delegated actions (send_message, list_channels, notify)
                            let result = self.process_delegated_action(&tc.name, result).await;

                            let result_str = serde_json::to_string(&result)
                                .unwrap_or_else(|_| "{}".to_string());

                            // SP-5: Scan native tool output
                            if self.guardian.is_enabled() {
                                let scan = self
                                    .guardian
                                    .scan_extension_output("omni.native", &result_str);
                                if scan.blocked {
                                    tracing::warn!(
                                        tool = tc.name.as_str(),
                                        "Guardian blocked native tool output, sanitizing"
                                    );
                                    messages.push(ChatMessage::tool_result(
                                        &tc.id,
                                        "[Content blocked by Guardian: potentially unsafe output]",
                                    ));
                                    continue;
                                }
                            }

                            self.event_bus.emit(OmniEvent::ExtensionResult {
                                extension_id: "omni.native".to_string(),
                                tool_name: tc.name.clone(),
                                result: result.clone(),
                            });

                            // ── Hook: after_tool_call (native) ────────────
                            let result_str = self.run_after_tool_call_hook(
                                &tc.name, &tc.arguments, result_str,
                            ).await;

                            // Extract _image_data for multimodal tool results
                            messages.push(Self::build_tool_result_message(
                                &tc.id, &result_str, &result,
                            ));
                        }
                        Err(e) => {
                            self.event_bus.emit(OmniEvent::ExtensionError {
                                extension_id: "omni.native".to_string(),
                                error: e.to_string(),
                            });
                            messages.push(ChatMessage::tool_result(
                                &tc.id,
                                &format!("Error: {}", e),
                            ));
                        }
                    }
                } else if tc.name.starts_with("mcp_") && self.mcp_manager.is_some() {
                    // ── MCP tool execution ─────────────────────────────
                    let mcp = self.mcp_manager.as_ref().unwrap();

                    self.event_bus.emit(OmniEvent::ExtensionInvoked {
                        extension_id: "omni.mcp".to_string(),
                        tool_name: tc.name.clone(),
                        params: params.clone(),
                    });

                    match mcp.execute_tool(&tc.name, params.clone()).await {
                        Ok(result) => {
                            let result_str = serde_json::to_string(&result)
                                .unwrap_or_else(|_| "{}".to_string());

                            // SP-6: Scan MCP tool output
                            if self.guardian.is_enabled() {
                                let scan = self
                                    .guardian
                                    .scan_extension_output("omni.mcp", &result_str);
                                if scan.blocked {
                                    tracing::warn!(
                                        tool = tc.name.as_str(),
                                        "Guardian blocked MCP tool output, sanitizing"
                                    );
                                    messages.push(ChatMessage::tool_result(
                                        &tc.id,
                                        "[Content blocked by Guardian: potentially unsafe MCP tool output]",
                                    ));
                                    continue;
                                }
                            }

                            self.event_bus.emit(OmniEvent::ExtensionResult {
                                extension_id: "omni.mcp".to_string(),
                                tool_name: tc.name.clone(),
                                result: result.clone(),
                            });

                            // ── Hook: after_tool_call (MCP) ────────────
                            let result_str = self.run_after_tool_call_hook(
                                &tc.name, &tc.arguments, result_str,
                            ).await;

                            messages.push(ChatMessage::tool_result(&tc.id, &result_str));
                        }
                        Err(e) => {
                            self.event_bus.emit(OmniEvent::ExtensionError {
                                extension_id: "omni.mcp".to_string(),
                                error: e.to_string(),
                            });
                            messages.push(ChatMessage::tool_result(
                                &tc.id,
                                &format!("MCP tool error: {}", e),
                            ));
                        }
                    }
                } else if self.is_flowchart_tool(&tc.name).await {
                    // ── Flowchart tool execution ──────────────────────
                    let (fc_id, tool_name) = parse_tool_name(&tc.name)?;
                    let fc_registry = self.flowchart_registry.as_ref().unwrap();

                    self.event_bus.emit(OmniEvent::FlowchartExecutionStarted {
                        flowchart_id: fc_id.to_string(),
                        tool_name: tool_name.to_string(),
                    });

                    match fc_registry.invoke_tool(fc_id, tool_name, &params).await {
                        Ok(result) => {
                            let result_str = serde_json::to_string(&result)
                                .unwrap_or_else(|_| "{}".to_string());

                            // SP-5: Scan flowchart tool output
                            if self.guardian.is_enabled() {
                                let scan = self
                                    .guardian
                                    .scan_extension_output(fc_id, &result_str);
                                if scan.blocked {
                                    tracing::warn!(
                                        flowchart = fc_id,
                                        tool = tool_name,
                                        "Guardian blocked flowchart output, sanitizing"
                                    );
                                    self.event_bus.emit(OmniEvent::FlowchartExecutionCompleted {
                                        flowchart_id: fc_id.to_string(),
                                        tool_name: tool_name.to_string(),
                                        success: false,
                                    });
                                    messages.push(ChatMessage::tool_result(
                                        &tc.id,
                                        "[Content blocked by Guardian: potentially unsafe flowchart output]",
                                    ));
                                    continue;
                                }
                            }

                            self.event_bus.emit(OmniEvent::FlowchartExecutionCompleted {
                                flowchart_id: fc_id.to_string(),
                                tool_name: tool_name.to_string(),
                                success: true,
                            });

                            let result_str = self.run_after_tool_call_hook(
                                &tc.name, &tc.arguments, result_str,
                            ).await;

                            messages.push(ChatMessage::tool_result(&tc.id, &result_str));
                        }
                        Err(e) => {
                            self.event_bus.emit(OmniEvent::FlowchartExecutionCompleted {
                                flowchart_id: fc_id.to_string(),
                                tool_name: tool_name.to_string(),
                                success: false,
                            });
                            messages.push(ChatMessage::tool_result(
                                &tc.id,
                                &format!("Error: {}", e),
                            ));
                        }
                    }
                } else {
                    // ── Extension tool execution ──────────────────────
                    let (ext_id, tool_name) = parse_tool_name(&tc.name)?;

                    self.event_bus.emit(OmniEvent::ExtensionInvoked {
                        extension_id: ext_id.to_string(),
                        tool_name: tool_name.to_string(),
                        params: params.clone(),
                    });

                    match self
                        .extension_host
                        .invoke_tool(ext_id, tool_name, &params)
                        .await
                    {
                        Ok(result) => {
                            let result_str = serde_json::to_string(&result)
                                .unwrap_or_else(|_| "{}".to_string());

                            // ── SP-5: Scan extension output ──────────────
                            if self.guardian.is_enabled() {
                                let scan = self
                                    .guardian
                                    .scan_extension_output(ext_id, &result_str);
                                if scan.blocked {
                                    tracing::warn!(
                                        extension = ext_id,
                                        tool = tool_name,
                                        "Guardian blocked extension output, sanitizing"
                                    );
                                    messages.push(ChatMessage::tool_result(
                                        &tc.id,
                                        "[Content blocked by Guardian: potentially unsafe extension output]",
                                    ));
                                    continue;
                                }
                            }

                            self.event_bus.emit(OmniEvent::ExtensionResult {
                                extension_id: ext_id.to_string(),
                                tool_name: tool_name.to_string(),
                                result: result.clone(),
                            });

                            // ── Hook: after_tool_call (extension) ─────────
                            let result_str = self.run_after_tool_call_hook(
                                &tc.name, &tc.arguments, result_str,
                            ).await;

                            messages.push(ChatMessage::tool_result(&tc.id, &result_str));
                        }
                        Err(omni_extensions::error::ExtensionError::PermissionDenied(
                            reason,
                        )) => {
                            self.event_bus.emit(OmniEvent::ExtensionError {
                                extension_id: ext_id.to_string(),
                                error: format!("Permission denied: {}", reason),
                            });
                            messages.push(ChatMessage::tool_result(
                                &tc.id,
                                &format!("Permission denied: {}", reason),
                            ));
                        }
                        Err(e) => {
                            self.event_bus.emit(OmniEvent::ExtensionError {
                                extension_id: ext_id.to_string(),
                                error: e.to_string(),
                            });
                            messages.push(ChatMessage::tool_result(
                                &tc.id,
                                &format!("Error: {}", e),
                            ));
                        }
                    }
                }
            }

            // Loop continues -- the LLM will see tool results and generate a response
        }

        self.event_bus.emit(OmniEvent::LlmComplete {
            session_id: self.session_id.clone().unwrap_or_default(),
            message_id: uuid::Uuid::new_v4().to_string(),
        });

        Ok(AgentResult {
            text: final_text,
            iterations: iteration,
        })
    }

    /// Check if a tool name corresponds to a registered flowchart tool.
    ///
    /// Uses the `flow.` prefix on the namespace to disambiguate from WASM
    /// extension tools (which use reverse-domain IDs like `com.example.ext`).
    /// The flowchart registry enforces that all flowchart IDs start with `flow.`.
    async fn is_flowchart_tool(&self, tool_name: &str) -> bool {
        if let Some(ref fc_registry) = self.flowchart_registry {
            if let Ok((fc_id, _tool)) = parse_tool_name(tool_name) {
                // Only treat as flowchart if namespace starts with "flow."
                if fc_id.starts_with("flow.") {
                    return fc_registry.get(fc_id).await.is_some();
                }
            }
        }
        false
    }

    /// Build a tool result message, extracting `_image_data` for multimodal support.
    ///
    /// When a tool result contains `_image_data` (an array of `{mime_type, data}` objects),
    /// the images are extracted and attached to the ChatMessage. The `_image_data` field
    /// is removed from the text content to keep it clean.
    fn build_tool_result_message(
        tool_call_id: &str,
        result_str: &str,
        result_value: &serde_json::Value,
    ) -> ChatMessage {
        // Check for _image_data in the result
        if let Some(image_arr) = result_value.get("_image_data").and_then(|v| v.as_array()) {
            let images: Vec<ImageContent> = image_arr
                .iter()
                .filter_map(|img| {
                    let mime = img.get("mime_type")?.as_str()?;
                    let data = img.get("data")?.as_str()?;
                    Some(ImageContent {
                        mime_type: mime.to_string(),
                        data: data.to_string(),
                    })
                })
                .collect();

            if !images.is_empty() {
                // Build text content without the _image_data field
                let mut clean = result_value.clone();
                if let Some(obj) = clean.as_object_mut() {
                    obj.remove("_image_data");
                }
                let clean_str = serde_json::to_string(&clean)
                    .unwrap_or_else(|_| result_str.to_string());
                return ChatMessage::tool_result_with_images(tool_call_id, &clean_str, images);
            }
        }

        ChatMessage::tool_result(tool_call_id, result_str)
    }

    /// Run the after_tool_call hook on a tool result, returning the (possibly modified) result.
    async fn run_after_tool_call_hook(
        &self,
        tool_name: &str,
        arguments: &str,
        result_str: String,
    ) -> String {
        let ctx = HookContext {
            hook_point: HookPoint::AfterToolCall,
            session_id: None,
            text: None,
            tool_call: Some(ToolCallInfo {
                name: tool_name.to_string(),
                arguments: arguments.to_string(),
                result: Some(result_str.clone()),
            }),
            messages: None,
            metadata: serde_json::json!({}),
        };
        match self.hook_registry.run_modifying(HookPoint::AfterToolCall, ctx).await {
            HookResult::Continue(ctx) => {
                ctx.tool_call
                    .and_then(|tc| tc.result)
                    .unwrap_or(result_str)
            }
            HookResult::Block { reason } => {
                tracing::info!(
                    tool = tool_name,
                    reason = reason.as_str(),
                    "Hook blocked tool result"
                );
                format!("Blocked by hook: {}", reason)
            }
        }
    }

    /// Process delegated tool actions -- tools that return structured intents
    /// (like send_message, list_channels, notify) instead of executing directly.
    /// Routes them through the action handler or event bus for actual execution.
    async fn process_delegated_action(
        &self,
        _tool_name: &str,
        result: serde_json::Value,
    ) -> serde_json::Value {
        let action = result["action"].as_str().unwrap_or("");

        match action {
            "send_message" => {
                if let Some(handler) = &self.action_handler {
                    let channel_id = result["channel_id"].as_str().unwrap_or("");
                    let recipient = result["recipient"].as_str().unwrap_or("");
                    let text = result["text"].as_str().unwrap_or("");
                    let caller_ext = result["caller_extension_id"].as_str();

                    // Enforce binding: bound extensions can only send through their bound channels
                    if let Err(reason) = handler.check_send_binding(caller_ext, channel_id).await {
                        return serde_json::json!({
                            "action": "send_message",
                            "status": "blocked",
                            "error": reason,
                        });
                    }

                    match handler.send_message(channel_id, recipient, text).await {
                        Ok(actual_result) => actual_result,
                        Err(e) => {
                            serde_json::json!({
                                "action": "send_message",
                                "status": "failed",
                                "error": e,
                            })
                        }
                    }
                } else {
                    // No handler -- return the intent as-is with a note
                    let mut r = result;
                    r["status"] = serde_json::json!("pending");
                    r["note"] = serde_json::json!(
                        "No channel handler configured. Message not delivered."
                    );
                    r
                }
            }
            "list_channels" => {
                if let Some(handler) = &self.action_handler {
                    match handler.list_channels().await {
                        Ok(actual_result) => actual_result,
                        Err(e) => {
                            serde_json::json!({
                                "action": "list_channels",
                                "channels": [],
                                "error": e,
                            })
                        }
                    }
                } else {
                    result
                }
            }
            "notify" => {
                // Emit notification event via event bus for Tauri/CLI to display
                let title = result["title"].as_str().unwrap_or("").to_string();
                let body = result["body"].as_str().unwrap_or("").to_string();
                let urgency = result["urgency"].as_str().unwrap_or("normal").to_string();
                self.event_bus.emit(OmniEvent::Notification {
                    title,
                    body,
                    urgency,
                });
                result
            }
            "spawn_agent" => {
                // Sub-agent delegation: emit event and return task info
                let task = result["task"].as_str().unwrap_or("").to_string();
                let wait = result["wait"].as_bool().unwrap_or(true);
                let task_id = uuid::Uuid::new_v4().to_string();

                self.event_bus.emit(OmniEvent::SubAgentSpawned {
                    task_id: task_id.clone(),
                    task: task.clone(),
                });

                if wait {
                    // Synchronous sub-agent: note that full spawning requires
                    // the runtime layer to create a new AgentLoop. Here we return
                    // the intent so the caller can handle the actual spawning.
                    serde_json::json!({
                        "action": "spawn_agent",
                        "task_id": task_id,
                        "task": task,
                        "status": "pending",
                        "wait": true,
                        "note": "Sub-agent task queued. The runtime will spawn a new AgentLoop for this task."
                    })
                } else {
                    serde_json::json!({
                        "action": "spawn_agent",
                        "task_id": task_id,
                        "task": task,
                        "status": "spawned",
                        "wait": false,
                        "note": "Sub-agent spawned in background. Check status with the task_id."
                    })
                }
            }
            _ => {
                // Not a delegated action -- return as-is
                // This covers image_analyze, cron_schedule, and regular tool results
                result
            }
        }
    }

    /// Fire session_start notification hooks (call when a new session begins).
    pub async fn notify_session_start(&self, session_id: &str) {
        let ctx = HookContext {
            hook_point: HookPoint::SessionStart,
            session_id: Some(session_id.to_string()),
            text: None,
            tool_call: None,
            messages: None,
            metadata: serde_json::json!({}),
        };
        self.hook_registry.run_notification(HookPoint::SessionStart, ctx).await;
    }

    /// Fire session_end notification hooks (call when a session ends).
    pub async fn notify_session_end(&self, session_id: &str) {
        let ctx = HookContext {
            hook_point: HookPoint::SessionEnd,
            session_id: Some(session_id.to_string()),
            text: None,
            tool_call: None,
            messages: None,
            metadata: serde_json::json!({}),
        };
        self.hook_registry.run_notification(HookPoint::SessionEnd, ctx).await;
    }
}

/// Parse a combined tool name "extension_id.tool_name" into components.
pub fn parse_tool_name(name: &str) -> Result<(&str, &str)> {
    // Find the last dot -- extension IDs use dots (e.g., "com.example.ext.tool_name")
    let last_dot = name
        .rfind('.')
        .ok_or_else(|| LlmError::InvalidToolName(name.to_string()))?;

    let ext_id = &name[..last_dot];
    let tool_name = &name[last_dot + 1..];

    if ext_id.is_empty() || tool_name.is_empty() {
        return Err(LlmError::InvalidToolName(name.to_string()));
    }

    Ok((ext_id, tool_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_name_valid() {
        let (ext_id, tool_name) = parse_tool_name("com.example.weather.get_weather").unwrap();
        assert_eq!(ext_id, "com.example.weather");
        assert_eq!(tool_name, "get_weather");
    }

    #[test]
    fn test_parse_tool_name_simple() {
        let (ext_id, tool_name) = parse_tool_name("ext.tool").unwrap();
        assert_eq!(ext_id, "ext");
        assert_eq!(tool_name, "tool");
    }

    #[test]
    fn test_parse_tool_name_no_dot() {
        let result = parse_tool_name("nodot");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tool_name_empty_parts() {
        let result = parse_tool_name(".tool");
        assert!(result.is_err());

        let result = parse_tool_name("ext.");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tool_name_multiple_dots() {
        let (ext_id, tool_name) =
            parse_tool_name("com.omni.test-hello.hello").unwrap();
        assert_eq!(ext_id, "com.omni.test-hello");
        assert_eq!(tool_name, "hello");
    }
}
