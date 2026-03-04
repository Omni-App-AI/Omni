//! Callback implementations that bridge extension host functions to runtime services.
//!
//! # Sync→Async Bridging Pattern
//!
//! Extensions run in synchronous WASM sandboxes and the flowchart engine invokes
//! callbacks from `spawn_blocking` threads. Both need to call async services
//! (LLMBridge, ChannelManager, McpManager).
//!
//! The pattern used throughout: `std::thread::scope` + `Handle::block_on`.
//! The scoped thread avoids nesting `block_on` calls on the tokio runtime
//! thread, while the scope guarantees borrows (like `&dyn LlmProgressReporter`)
//! are valid for the thread's lifetime.
//!
//! **Thread overhead**: ~50μs per OS thread creation, negligible compared to
//! the I/O latencies of the services called (LLM: seconds, HTTP: 100s of ms,
//! channels: 10s of ms). This is a deliberate design trade-off -- the alternative
//! (unsafe static refs or Arc-wrapped closures) adds complexity without
//! meaningful performance gain.

use std::sync::Arc;


use omni_channels::manager::ChannelManager;
use omni_channels::OutgoingMessage;
use omni_extensions::sandbox::{AgentCallback, ChannelCallback, GuardianCallback, LlmCallback, LlmProgressReporter, McpCallback, NativeToolCallback};
use omni_guardian::Guardian;
use omni_llm::bridge::LLMBridge;
use omni_llm::mcp::McpManager;
use omni_llm::tools::NativeToolRegistry;
use omni_llm::types::{ChatChunk, ChatMessage, ToolCall, ToolSchema};

/// LLM inference callback for WASM extensions.
///
/// Bridges the sync `llm_request` host function to the async `LLMBridge`.
/// Uses a scoped thread to avoid nesting `block_on` on the tokio runtime thread.
pub struct AppLlmCallback {
    bridge: Arc<LLMBridge>,
    handle: tokio::runtime::Handle,
}

impl AppLlmCallback {
    pub fn new(bridge: Arc<LLMBridge>, handle: tokio::runtime::Handle) -> Self {
        Self { bridge, handle }
    }
}

impl LlmCallback for AppLlmCallback {
    fn request(&self, prompt: &str, max_tokens: Option<u32>) -> Result<String, String> {
        self.request_with_progress(prompt, max_tokens, &NoopProgressReporter)
    }

    fn request_with_progress(
        &self,
        prompt: &str,
        max_tokens: Option<u32>,
        progress: &dyn LlmProgressReporter,
    ) -> Result<String, String> {
        let bridge = self.bridge.clone();
        let handle = self.handle.clone();
        let prompt = prompt.to_string();

        // Use a channel to send progress chunks from the worker thread
        // back to the calling thread (which owns the progress reference).
        // This avoids sending raw pointers across thread boundaries.
        let (chunk_tx, chunk_rx) = std::sync::mpsc::channel::<String>();

        std::thread::scope(|s| {
            // Worker thread: streams LLM response and sends chunks via channel.
            let join_handle = s.spawn(move || {
                handle.block_on(async {
                    let provider_ids = bridge.list_provider_ids().await;
                    let provider_id = provider_ids
                        .first()
                        .ok_or_else(|| "No LLM providers registered".to_string())?;

                    let messages = vec![ChatMessage::user(&prompt)];

                    let stream = bridge
                        .chat_stream(provider_id, messages, vec![], "default", max_tokens, None)
                        .await
                        .map_err(|e| format!("LLM stream error: {e}"))?;

                    use futures::StreamExt;
                    let mut full_text = String::new();
                    tokio::pin!(stream);
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(ChatChunk::TextDelta(text)) => {
                                let _ = chunk_tx.send(text.clone());
                                full_text.push_str(&text);
                            }
                            Ok(ChatChunk::Done) => break,
                            Ok(_) => {}
                            Err(e) => return Err(format!("LLM chunk error: {e}")),
                        }
                    }

                    Ok(full_text)
                })
            });

            // Calling thread: forward progress chunks to the reporter.
            // This runs on the thread that owns the `progress` reference.
            while let Ok(chunk) = chunk_rx.recv() {
                progress.on_chunk(&chunk);
            }

            join_handle
                .join()
                .map_err(|_| "LLM callback thread panicked".to_string())?
        })
    }
}

/// No-op progress reporter (used when streaming is not needed).
struct NoopProgressReporter;
impl LlmProgressReporter for NoopProgressReporter {
    fn on_chunk(&self, _text: &str) {}
}

/// Channel send callback for WASM extensions.
///
/// Bridges the sync `channel_send` host function to the async `ChannelManager`.
pub struct AppChannelCallback {
    channel_manager: Arc<ChannelManager>,
    handle: tokio::runtime::Handle,
}

impl AppChannelCallback {
    pub fn new(channel_manager: Arc<ChannelManager>, handle: tokio::runtime::Handle) -> Self {
        Self {
            channel_manager,
            handle,
        }
    }
}

impl ChannelCallback for AppChannelCallback {
    fn send_message(
        &self,
        channel_id: &str,
        recipient: &str,
        text: &str,
    ) -> Result<String, String> {
        let cm = self.channel_manager.clone();
        let handle = self.handle.clone();
        let channel_id = channel_id.to_string();
        let recipient = recipient.to_string();
        let text = text.to_string();

        std::thread::scope(|s| {
            s.spawn(|| {
                handle.block_on(async {
                    let msg = OutgoingMessage {
                        text,
                        media_url: None,
                        reply_to: None,
                        thread_id: None,
                    };

                    cm.send_message(&channel_id, &recipient, msg)
                        .await
                        .map_err(|e| format!("Channel send error: {e}"))?;

                    Ok(serde_json::json!({
                        "status": "sent",
                        "channel_id": channel_id,
                        "recipient": recipient,
                    })
                    .to_string())
                })
            })
            .join()
            .map_err(|_| "Channel callback thread panicked".to_string())?
        })
    }
}

/// MCP tool invocation callback for WASM extensions.
///
/// Bridges the sync `mcp_call` host function to the async `McpManager`.
pub struct AppMcpCallback {
    mcp_manager: Arc<McpManager>,
    handle: tokio::runtime::Handle,
}

impl AppMcpCallback {
    pub fn new(mcp_manager: Arc<McpManager>, handle: tokio::runtime::Handle) -> Self {
        Self {
            mcp_manager,
            handle,
        }
    }
}

impl McpCallback for AppMcpCallback {
    fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        params_json: &str,
    ) -> Result<String, String> {
        let mcp = self.mcp_manager.clone();
        let handle = self.handle.clone();
        let server_name = server_name.to_string();
        let tool_name = tool_name.to_string();
        let params_json = params_json.to_string();

        std::thread::scope(|s| {
            s.spawn(|| {
                handle.block_on(async {
                    // The McpManager expects the namespaced tool name: mcp_{server}_{tool}
                    let namespaced = format!("mcp_{}_{}", server_name, tool_name);
                    let params: serde_json::Value = serde_json::from_str(&params_json)
                        .map_err(|e| format!("Invalid params JSON: {e}"))?;

                    let result = mcp
                        .execute_tool(&namespaced, params)
                        .await
                        .map_err(|e| format!("MCP tool error: {e}"))?;

                    serde_json::to_string(&result)
                        .map_err(|e| format!("Failed to serialize MCP result: {e}"))
                })
            })
            .join()
            .map_err(|_| "MCP callback thread panicked".to_string())?
        })
    }
}

/// Flowchart sub-flow callback for flowchart SubFlow nodes.
///
/// Bridges the sync `FlowchartCallback` trait to the async `FlowchartRegistry`.
pub struct AppFlowchartCallback {
    registry: Arc<omni_extensions::flowchart::FlowchartRegistry>,
    handle: tokio::runtime::Handle,
}

impl AppFlowchartCallback {
    pub fn new(
        registry: Arc<omni_extensions::flowchart::FlowchartRegistry>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        Self { registry, handle }
    }
}

impl omni_extensions::sandbox::FlowchartCallback for AppFlowchartCallback {
    fn invoke(
        &self,
        flowchart_id: &str,
        tool_name: &str,
        params_json: &str,
        depth: u32,
    ) -> Result<String, String> {
        let registry = self.registry.clone();
        let handle = self.handle.clone();
        let flowchart_id = flowchart_id.to_string();
        let tool_name = tool_name.to_string();
        let params_json = params_json.to_string();

        std::thread::scope(|s| {
            s.spawn(|| {
                handle.block_on(async {
                    let params: serde_json::Value = serde_json::from_str(&params_json)
                        .map_err(|e| format!("Invalid params JSON: {e}"))?;

                    let result = registry
                        .invoke_tool_with_depth(&flowchart_id, &tool_name, &params, depth)
                        .await
                        .map_err(|e| format!("{e}"))?;

                    serde_json::to_string(&result)
                        .map_err(|e| format!("Failed to serialize result: {e}"))
                })
            })
            .join()
            .map_err(|_| "Flowchart callback thread panicked".to_string())?
        })
    }
}

/// Native tool callback for flowchart NativeTool nodes.
///
/// Bridges the sync `NativeToolCallback` trait to the async `NativeToolRegistry`.
pub struct AppNativeToolCallback {
    registry: Arc<NativeToolRegistry>,
    handle: tokio::runtime::Handle,
}

impl AppNativeToolCallback {
    pub fn new(registry: Arc<NativeToolRegistry>, handle: tokio::runtime::Handle) -> Self {
        Self { registry, handle }
    }
}

impl NativeToolCallback for AppNativeToolCallback {
    fn execute(
        &self,
        tool_name: &str,
        params_json: &str,
    ) -> Result<String, String> {
        let registry = self.registry.clone();
        let handle = self.handle.clone();
        let tool_name = tool_name.to_string();
        let params_json = params_json.to_string();

        std::thread::scope(|s| {
            s.spawn(|| {
                handle.block_on(async {
                    let params: serde_json::Value = serde_json::from_str(&params_json)
                        .map_err(|e| format!("Invalid params JSON: {e}"))?;

                    let result = registry
                        .execute(&tool_name, params, "omni.flowchart")
                        .await
                        .map_err(|e| format!("{e}"))?;

                    serde_json::to_string(&result)
                        .map_err(|e| format!("Failed to serialize result: {e}"))
                })
            })
            .join()
            .map_err(|_| "Native tool callback thread panicked".to_string())?
        })
    }

    fn list_tools(&self) -> Result<String, String> {
        let schemas = self.registry.get_all_schemas();
        serde_json::to_string(&schemas)
            .map_err(|e| format!("Failed to serialize tool schemas: {e}"))
    }
}

/// Guardian anti-injection callback for flowchart engine.
///
/// Bridges the sync `GuardianCallback` trait to the `Guardian` scanner.
/// Used by the flowchart engine to scan content at every external boundary
/// (LLM prompts, HTTP responses, native tool params, channel messages, etc.).
pub struct AppGuardianCallback {
    guardian: Arc<Guardian>,
}

impl AppGuardianCallback {
    pub fn new(guardian: Arc<Guardian>) -> Self {
        Self { guardian }
    }
}

impl GuardianCallback for AppGuardianCallback {
    fn scan_input(&self, content: &str) -> Result<(), String> {
        if !self.guardian.is_enabled() {
            return Ok(());
        }
        let result = self.guardian.scan_input(content);
        if result.blocked {
            Err(result
                .reason
                .unwrap_or_else(|| "Blocked by Guardian".to_string()))
        } else {
            Ok(())
        }
    }

    fn scan_output(&self, source_id: &str, content: &str) -> Result<(), String> {
        if !self.guardian.is_enabled() {
            return Ok(());
        }
        let result = self.guardian.scan_extension_output(source_id, content);
        if result.blocked {
            Err(result
                .reason
                .unwrap_or_else(|| "Blocked by Guardian".to_string()))
        } else {
            Ok(())
        }
    }
}

/// Agent loop callback for flowchart AgentRequest nodes.
///
/// Runs a lightweight multi-turn LLM agent loop with native tool access.
/// This avoids depending on `ExtensionHost` (which is `!Send`) by using
/// `LLMBridge` + `NativeToolRegistry` directly.
pub struct AppAgentCallback {
    bridge: Arc<LLMBridge>,
    native_tools: Arc<NativeToolRegistry>,
    mcp_manager: Arc<McpManager>,
    guardian: Arc<Guardian>,
    handle: tokio::runtime::Handle,
}

impl AppAgentCallback {
    pub fn new(
        bridge: Arc<LLMBridge>,
        native_tools: Arc<NativeToolRegistry>,
        mcp_manager: Arc<McpManager>,
        guardian: Arc<Guardian>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            bridge,
            native_tools,
            mcp_manager,
            guardian,
            handle,
        }
    }
}

impl AgentCallback for AppAgentCallback {
    fn run(
        &self,
        user_message: &str,
        system_prompt: Option<&str>,
        max_iterations: Option<u32>,
    ) -> Result<String, String> {
        let bridge = self.bridge.clone();
        let native_tools = self.native_tools.clone();
        let mcp_manager = self.mcp_manager.clone();
        let guardian = self.guardian.clone();
        let handle = self.handle.clone();
        let user_message = user_message.to_string();
        let system_prompt = system_prompt.map(|s| s.to_string());
        let max_iterations = max_iterations.unwrap_or(5).min(20);

        std::thread::scope(|s| {
            s.spawn(|| {
                handle.block_on(async {
                    run_agent_loop(
                        &bridge,
                        &native_tools,
                        &mcp_manager,
                        &guardian,
                        &user_message,
                        system_prompt.as_deref(),
                        max_iterations,
                    )
                    .await
                })
            })
            .join()
            .map_err(|_| "Agent callback thread panicked".to_string())?
        })
    }
}

/// Lightweight multi-turn agent loop for flowchart AgentRequest nodes.
///
/// Sends messages to the LLM, executes tool calls (native + MCP), and
/// feeds results back until the LLM responds without tool calls or
/// the iteration limit is reached.
async fn run_agent_loop(
    bridge: &LLMBridge,
    native_tools: &NativeToolRegistry,
    mcp_manager: &McpManager,
    guardian: &Guardian,
    user_message: &str,
    system_prompt: Option<&str>,
    max_iterations: u32,
) -> Result<String, String> {
    // Guardian: scan user input
    if guardian.is_enabled() {
        let scan = guardian.scan_input(user_message);
        if scan.blocked {
            return Err(scan.reason.unwrap_or_else(|| "Blocked by Guardian".to_string()));
        }
    }

    // Build initial messages
    let mut messages = Vec::new();
    if let Some(sp) = system_prompt {
        messages.push(ChatMessage::system(sp));
    }
    messages.push(ChatMessage::user(user_message));

    // Collect tool schemas: native + MCP
    let mut tool_schemas: Vec<ToolSchema> = native_tools.get_all_schemas();
    let mcp_schemas = mcp_manager.get_all_schemas().await;
    tool_schemas.extend(mcp_schemas);

    // Resolve provider
    let provider_ids = bridge.list_provider_ids().await;
    let provider_id = provider_ids
        .first()
        .ok_or_else(|| "No LLM providers registered".to_string())?
        .clone();

    for iteration in 0..max_iterations {
        // Call LLM with streaming
        let stream = bridge
            .chat_stream(&provider_id, messages.clone(), tool_schemas.clone(), "default", None, None)
            .await
            .map_err(|e| format!("LLM error: {e}"))?;

        use futures::StreamExt;
        let mut assistant_text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        tokio::pin!(stream);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("LLM chunk error: {e}"))?;
            match chunk {
                ChatChunk::TextDelta(text) => {
                    assistant_text.push_str(&text);
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
                ChatChunk::Done => break,
                _ => {}
            }
        }

        // Guardian: scan LLM output
        if guardian.is_enabled() && !assistant_text.is_empty() {
            let scan = guardian.scan_output_chunk(&assistant_text);
            if scan.blocked {
                return Err(scan.reason.unwrap_or_else(|| "Output blocked by Guardian".to_string()));
            }
        }

        // No tool calls -- return the response
        if tool_calls.is_empty() {
            return Ok(assistant_text);
        }

        // Record assistant message with tool calls
        messages.push(ChatMessage::assistant_with_tool_calls(
            &assistant_text,
            tool_calls.clone(),
        ));

        // Execute tool calls and collect results
        for tc in &tool_calls {
            let result = execute_tool_call(native_tools, mcp_manager, tc, "omni.flowchart.agent").await;

            // Guardian: scan tool result
            if guardian.is_enabled() {
                if let Ok(ref result_text) = result {
                    let scan = guardian.scan_output_chunk(result_text);
                    if scan.blocked {
                        let blocked_msg = scan.reason.unwrap_or_else(|| "Tool result blocked".to_string());
                        messages.push(ChatMessage::tool_result(
                            &tc.id,
                            &format!("Error: Guardian blocked result: {blocked_msg}"),
                        ));
                        continue;
                    }
                }
            }

            match result {
                Ok(result_text) => {
                    messages.push(ChatMessage::tool_result(&tc.id, &result_text));
                }
                Err(err) => {
                    messages.push(ChatMessage::tool_result(
                        &tc.id,
                        &format!("Error: {err}"),
                    ));
                }
            }
        }

        // Log iteration progress
        tracing::debug!(
            iteration = iteration + 1,
            tool_count = tool_calls.len(),
            "Agent callback loop iteration"
        );
    }

    // Max iterations reached -- return whatever text we have
    Err(format!(
        "Agent reached maximum iterations ({max_iterations}) without final response"
    ))
}

/// Execute a single tool call against native tools or MCP.
pub(crate) async fn execute_tool_call(
    native_tools: &NativeToolRegistry,
    mcp_manager: &McpManager,
    tc: &ToolCall,
    caller_id: &str,
) -> Result<String, String> {
    let params: serde_json::Value = serde_json::from_str(&tc.arguments)
        .map_err(|e| format!("Invalid tool arguments: {e}"))?;

    // Try native tools first
    if native_tools.has_tool(&tc.name) {
        let result = native_tools
            .execute(&tc.name, params, caller_id)
            .await
            .map_err(|e| format!("{e}"))?;
        return serde_json::to_string(&result)
            .map_err(|e| format!("Failed to serialize result: {e}"));
    }

    // Try MCP tools (prefixed with mcp_)
    if tc.name.starts_with("mcp_") {
        let result = mcp_manager
            .execute_tool(&tc.name, params)
            .await
            .map_err(|e| format!("MCP tool error: {e}"))?;
        return serde_json::to_string(&result)
            .map_err(|e| format!("Failed to serialize result: {e}"));
    }

    Err(format!("Unknown tool: {}", tc.name))
}
