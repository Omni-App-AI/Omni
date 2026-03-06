use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;

use crate::dto::{
    AuditEntryDto, BindingDto, ChannelDto, ChannelFeaturesDto, ChannelTypeDto, ExtensionDto,
    ExtensionInstanceDto, ExtensionUpdateDto, FlowchartDefinitionDto, FlowchartDto,
    FlowchartPermissionDto, FlowchartTestResultDto, FlowchartToolDefDto,
    FlowchartValidationDto, GuardianMetricsDto, MarketplaceCategoryDto, MarketplaceDetailDto,
    MarketplaceExtensionDto, MarketplaceSearchResultDto, McpServerDto, McpToolDto, MessageDto,
    PendingBlockDto, ProviderDto, ProviderTypeInfoDto, SessionDto,
};
use crate::state::AppState;
use omni_core::config::ProviderConfig;
use omni_llm::bridge::LLMBridge;
use omni_llm::providers::LLMProvider;

/// Instantiate and register an LLM provider with the bridge from config.
///
/// Called on startup (from state.rs) and when a provider is added/updated.
pub(crate) async fn register_provider_from_config(
    bridge: &LLMBridge,
    name: &str,
    cfg: &ProviderConfig,
) {
    let provider: Arc<dyn LLMProvider> = match cfg.provider_type.as_str() {
        "openai" => {
            let transport = match cfg.transport.as_deref() {
                Some("auto") => omni_llm::providers::openai_ws::OpenAITransport::Auto,
                Some("ws") | Some("websocket") => {
                    omni_llm::providers::openai_ws::OpenAITransport::WebSocket
                }
                _ => omni_llm::providers::openai_ws::OpenAITransport::Sse,
            };
            Arc::new(omni_llm::providers::openai::OpenAIProvider::with_transport(
                cfg.endpoint.as_deref(),
                transport,
            ))
        }
        "anthropic" => Arc::new(omni_llm::providers::anthropic::AnthropicProvider::new(
            cfg.endpoint.as_deref(),
        )),
        "google" => Arc::new(omni_llm::providers::google::GoogleProvider::new(
            cfg.endpoint.as_deref(),
        )),
        "ollama" => Arc::new(omni_llm::providers::ollama::OllamaProvider::new(
            cfg.endpoint.as_deref(),
        )),
        "bedrock" => Arc::new(omni_llm::providers::bedrock::BedrockProvider::new(None)),
        "custom" => Arc::new(omni_llm::providers::custom::CustomProvider::new(
            cfg.endpoint
                .as_deref()
                .unwrap_or("http://localhost:8080/v1"),
            Some(name),
        )),
        other => {
            tracing::warn!(
                provider = name,
                provider_type = other,
                "Unknown provider type, skipping registration"
            );
            return;
        }
    };
    bridge.register_provider_as(name, provider).await;
    tracing::info!(
        provider = name,
        provider_type = cfg.provider_type.as_str(),
        "Registered LLM provider"
    );
}

/// Remove all channel bindings for a given extension (registry + database + events).
async fn cleanup_bindings_for_extension(state: &AppState, extension_id: &str) {
    let bindings = state.binding_registry.list_for_extension(extension_id);
    if bindings.is_empty() {
        return;
    }
    tracing::info!(
        extension = %extension_id,
        count = bindings.len(),
        "Removing channel bindings for extension"
    );
    for binding in bindings {
        state.binding_registry.remove(&binding.id);
        // Delete from database (best-effort)
        let db = state.db.clone();
        let bid = binding.id.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let db = db.lock().unwrap();
            db.delete_binding(&bid)
        })
        .await;
        state
            .event_bus
            .emit(omni_core::events::OmniEvent::ChannelBindingRemoved {
                binding_id: binding.id,
            });
    }
}

/// Send a message in a chat session. Returns the session ID.
/// The actual response streams via omni:llm-chunk / omni:llm-complete events.
#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    session_id: String,
    message: String,
) -> Result<String, String> {
    // Store user message in database
    let db = state.db.clone();
    let sid = session_id.clone();
    let msg = message.clone();
    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        // Check if this is the first message — if so, set a title from it
        let msg_count = db.count_messages_for_session(&sid).unwrap_or(0);
        db.insert_message(&omni_core::database::NewMessage {
            session_id: sid.clone(),
            role: "user".to_string(),
            content: msg.clone(),
            tool_call_id: None,
            tool_calls: None,
            token_count: None,
        })?;
        if msg_count == 0 {
            let title: String = msg.chars().take(80).collect();
            let title = title.trim().to_string();
            let metadata = serde_json::json!({ "title": title }).to_string();
            let _ = db.update_session_metadata(&sid, &metadata);
        }
        Ok::<String, anyhow::Error>(sid)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Emit event so the frontend knows the message was received
    state
        .event_bus
        .emit(omni_core::events::OmniEvent::MessageReceived {
            session_id: session_id.clone(),
            message_id: String::new(),
        });

    // Clone everything we need for the background agent task
    let bridge = state.llm_bridge.clone();
    let db = state.db.clone();
    let config = state.config.clone();
    let event_bus = state.event_bus.clone();
    let guardian = state.guardian.clone();
    let native_tools = Arc::new(omni_llm::tools::NativeToolRegistry::new_with_db(
        state.policy_engine.clone(),
        state.db.clone(),
    ));
    let mcp_manager = state.mcp_manager.clone();
    let session_id_ret = session_id.clone();

    // Spawn background task to run the LLM agent loop
    tokio::spawn(async move {
        if let Err(e) = run_chat_agent(
            &bridge,
            &db,
            &config,
            &event_bus,
            &guardian,
            &native_tools,
            &mcp_manager,
            &session_id,
            &message,
        )
        .await
        {
            tracing::error!(session = %session_id, "Agent loop failed: {}", e);
            event_bus.emit(omni_core::events::OmniEvent::LlmError {
                session_id,
                error: e,
            });
        }
    });

    Ok(session_id_ret)
}

/// Background agent loop for chat messages.
///
/// Loads conversation history, resolves the provider/model from config,
/// streams the LLM response (emitting chunk events), handles tool calls,
/// and saves the assistant message to the database.
async fn run_chat_agent(
    bridge: &Arc<LLMBridge>,
    db: &Arc<std::sync::Mutex<omni_core::database::Database>>,
    config: &Arc<tokio::sync::RwLock<omni_core::config::OmniConfig>>,
    event_bus: &omni_core::events::EventBus,
    guardian: &Arc<omni_guardian::Guardian>,
    native_tools: &Arc<omni_llm::tools::NativeToolRegistry>,
    mcp_manager: &Arc<omni_llm::mcp::McpManager>,
    session_id: &str,
    user_message: &str,
) -> Result<(), String> {
    use futures::StreamExt;
    use omni_llm::types::{ChatChunk, ChatMessage, ToolCall, ToolSchema};

    // 1. Guardian: scan user input
    if guardian.is_enabled() {
        let scan = guardian.scan_input(user_message);
        if scan.blocked {
            return Err(scan.reason.unwrap_or_else(|| "Blocked by Guardian".into()));
        }
    }

    // 1b. Pre-approve all native tool capabilities for the chat caller.
    // Without this, the default-deny policy would block every tool call.
    native_tools.pre_approve_all("omni.chat").await;

    // 2. Resolve provider + model from config
    let (provider_id, model, max_tokens, temperature) = {
        let cfg = config.read().await;
        let mut found = None;
        for (name, pcfg) in &cfg.providers {
            if pcfg.enabled {
                let model = pcfg
                    .default_model
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                found = Some((name.clone(), model, pcfg.max_tokens, pcfg.temperature));
                break;
            }
        }
        found.ok_or_else(|| "No enabled LLM provider configured".to_string())?
    };

    // 3. Load conversation history from DB
    let db_clone = db.clone();
    let sid = session_id.to_string();
    let existing = tokio::task::spawn_blocking(move || {
        let db = db_clone.lock().unwrap();
        db.get_messages_for_session(&sid)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let mut messages: Vec<ChatMessage> = Vec::new();
    // Add system prompt from config if configured
    {
        let cfg = config.read().await;
        if let Some(ref sp) = cfg.agent.system_prompt {
            messages.push(ChatMessage::system(sp));
        }
    }
    // Replay existing messages (including the one we just stored)
    for msg in &existing {
        let chat_msg = match msg.role.as_str() {
            "user" => ChatMessage::user(&msg.content),
            "assistant" => ChatMessage::assistant(&msg.content),
            "system" => ChatMessage::system(&msg.content),
            _ => continue,
        };
        messages.push(chat_msg);
    }

    // 4. Collect tool schemas (native + MCP)
    let mut tool_schemas: Vec<ToolSchema> = native_tools.get_all_schemas();
    let mcp_schemas = mcp_manager.get_all_schemas().await;
    tool_schemas.extend(mcp_schemas);

    // 5. Multi-turn agent loop
    let max_iterations = {
        let cfg = config.read().await;
        cfg.agent.max_iterations
    };

    for iteration in 0..max_iterations {
        // Call LLM with streaming
        let stream = bridge
            .chat_stream(
                &provider_id,
                messages.clone(),
                tool_schemas.clone(),
                &model,
                max_tokens,
                temperature,
            )
            .await
            .map_err(|e| format!("LLM error: {e}"))?;

        let mut assistant_text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        tokio::pin!(stream);

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {e}"))?;
            match chunk {
                ChatChunk::TextDelta(text) => {
                    // Emit streaming chunk to frontend
                    event_bus.emit(omni_core::events::OmniEvent::LlmChunk {
                        session_id: session_id.to_string(),
                        chunk: text.clone(),
                    });
                    assistant_text.push_str(&text);
                }
                ChatChunk::ThinkingDelta(text) => {
                    event_bus.emit(omni_core::events::OmniEvent::LlmThinking {
                        session_id: session_id.to_string(),
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
                ChatChunk::Done => break,
                _ => {}
            }
        }

        // Guardian: scan output
        if guardian.is_enabled() && !assistant_text.is_empty() {
            let scan = guardian.scan_output_chunk(&assistant_text);
            if scan.blocked {
                return Err(scan.reason.unwrap_or_else(|| "Output blocked by Guardian".into()));
            }
        }

        // No tool calls -- we have the final response
        if tool_calls.is_empty() {
            // Strip <think>...</think> tags from models that output raw thinking tokens
            let clean_text = strip_think_tags(&assistant_text);

            // Save assistant message to DB
            let db_clone = db.clone();
            let sid = session_id.to_string();
            let text = clean_text;
            tokio::task::spawn_blocking(move || {
                let db = db_clone.lock().unwrap();
                db.insert_message(&omni_core::database::NewMessage {
                    session_id: sid,
                    role: "assistant".to_string(),
                    content: text,
                    tool_call_id: None,
                    tool_calls: None,
                    token_count: None,
                })
            })
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;

            // Signal completion to frontend
            event_bus.emit(omni_core::events::OmniEvent::LlmComplete {
                session_id: session_id.to_string(),
                message_id: String::new(),
            });

            return Ok(());
        }

        // Record assistant message with tool calls
        messages.push(ChatMessage::assistant_with_tool_calls(
            &assistant_text,
            tool_calls.clone(),
        ));

        // Execute tool calls and collect results
        for tc in &tool_calls {
            let result =
                crate::callbacks::execute_tool_call(native_tools, mcp_manager, tc, "omni.chat").await;

            // Guardian: scan tool result
            if guardian.is_enabled() {
                if let Ok(ref result_text) = result {
                    let scan = guardian.scan_output_chunk(result_text);
                    if scan.blocked {
                        let blocked_msg = scan
                            .reason
                            .unwrap_or_else(|| "Tool result blocked".to_string());
                        messages.push(ChatMessage::tool_result(
                            &tc.id,
                            &format!("Error: Guardian blocked: {blocked_msg}"),
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

        tracing::debug!(
            iteration = iteration + 1,
            tool_count = tool_calls.len(),
            session = %session_id,
            "Chat agent loop iteration"
        );
    }

    Err(format!(
        "Agent reached maximum iterations ({max_iterations}) without final response"
    ))
}

/// Strip `<think>...</think>` blocks from model output.
///
/// Some models (e.g. NVIDIA minimax) output raw thinking tokens as
/// `<think>...</think>` in the text content. This strips them before
/// saving to the database so stored messages are clean.
fn strip_think_tags(text: &str) -> String {
    // Fast path: no think tags at all
    if !text.contains("<think>") {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut remaining = text;

    while let Some(start) = remaining.find("<think>") {
        // Add everything before the tag
        result.push_str(&remaining[..start]);

        // Find the closing tag
        if let Some(end) = remaining[start..].find("</think>") {
            // Skip past the closing tag
            remaining = &remaining[start + end + "</think>".len()..];
        } else {
            // Unclosed <think> at end -- discard the rest
            remaining = "";
            break;
        }
    }

    // Add any remaining text after the last </think>
    result.push_str(remaining);

    // Clean up whitespace artifacts from removal
    let trimmed = result.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.to_string()
}

/// Get all messages for a session.
#[tauri::command]
pub async fn get_session_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<MessageDto>, String> {
    let db = state.db.clone();
    let messages = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.get_messages_for_session(&session_id)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(messages
        .into_iter()
        .map(|m| MessageDto {
            role: m.role,
            content: m.content,
            tool_calls: m.tool_calls,
        })
        .collect())
}

/// List all chat sessions.
#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionDto>, String> {
    let db = state.db.clone();
    let sessions = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.list_sessions()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(sessions
        .into_iter()
        .map(|s| SessionDto {
            id: s.id,
            created_at: s.created_at,
            updated_at: s.updated_at,
            metadata: s.metadata,
        })
        .collect())
}

/// Create a new chat session.
#[tauri::command]
pub async fn create_session(
    state: State<'_, AppState>,
    metadata: Option<String>,
) -> Result<String, String> {
    let db = state.db.clone();
    let id = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.create_session(metadata.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(id)
}

/// Respond to a permission prompt.
#[tauri::command]
pub async fn permission_respond(
    state: State<'_, AppState>,
    prompt_id: String,
    decision: String,
    duration: String,
) -> Result<(), String> {
    let mut pending = state.pending_prompts.write().await;
    if let Some(sender) = pending.remove(&prompt_id) {
        let _ = sender.send(crate::state::PromptResponse { decision, duration });
        Ok(())
    } else {
        Err(format!("No pending prompt with id: {}", prompt_id))
    }
}

/// Revoke all permissions for an extension.
#[tauri::command]
pub async fn permission_revoke(
    state: State<'_, AppState>,
    extension_id: String,
) -> Result<(), String> {
    state
        .policy_engine
        .revoke_all(&extension_id)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Emergency kill switch: revoke ALL permissions for ALL extensions.
#[tauri::command]
pub async fn kill_switch(state: State<'_, AppState>) -> Result<u64, String> {
    let count = state
        .policy_engine
        .revoke_everything()
        .await
        .map_err(|e| e.to_string())?;

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::PermissionRevoked {
            extension_id: "*".to_string(),
            capability: "*".to_string(),
        });

    Ok(count)
}

/// Install an extension from a path and auto-activate it.
/// Returns the extension ID on success. If installation succeeds but activation
/// fails, the extension is still installed (enabled but inactive) and the error
/// is included in the returned string as a warning suffix.
#[tauri::command]
pub async fn install_extension(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<String, String> {
    let source = omni_extensions::host::ExtensionSource::Path(source_path.into());
    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    let (ext_id, mcp_servers) = tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().map_err(|e| {
            omni_extensions::ExtensionError::Other(format!("Lock poisoned: {}", e))
        })?;
        let id = handle.block_on(host.0.install(&source))?;
        // Auto-activate after successful install
        if let Err(e) = handle.block_on(host.0.activate(&id)) {
            tracing::warn!(extension = %id, "Auto-activate after install failed: {}", e);
        }
        let mcp = handle.block_on(host.0.get_mcp_servers(&id));
        Ok::<(String, Vec<_>), omni_extensions::ExtensionError>((id, mcp))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Auto-register extension MCP servers after install+activate
    if !mcp_servers.is_empty() {
        let mcp = state.mcp_manager.clone();
        let event_bus = state.event_bus.clone();
        for decl in mcp_servers {
            let server_name = format!("{}:{}", ext_id, decl.name);
            let config = omni_llm::mcp::McpServerConfig {
                name: server_name.clone(),
                command: decl.command,
                args: decl.args,
                env: decl.env,
                working_dir: decl.working_dir,
                auto_start: true,
            };
            match mcp.add_server(config).await {
                Ok(()) => {
                    let tool_count = mcp
                        .list_servers()
                        .await
                        .iter()
                        .find(|s| s.name == server_name)
                        .map(|s| s.tool_count)
                        .unwrap_or(0);
                    event_bus.emit(omni_core::events::OmniEvent::McpServerConnected {
                        server_name,
                        tool_count,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        extension = %ext_id,
                        server = %decl.name,
                        "Failed to auto-register MCP server: {}",
                        e
                    );
                }
            }
        }
    }

    Ok(ext_id)
}

/// List installed extensions with full manifest details.
#[tauri::command]
pub async fn list_extensions(state: State<'_, AppState>) -> Result<Vec<ExtensionDto>, String> {
    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    let details = tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.list_installed_details())
    })
    .await
    .map_err(|e| e.to_string())?;
    let extensions: Vec<ExtensionDto> = details
        .into_iter()
        .map(|d| ExtensionDto {
            id: d.id,
            name: d.name,
            version: d.version,
            author: d.author,
            description: d.description,
            enabled: d.enabled,
            active: d.active,
            tools: d.tools,
            permissions: d.permissions,
            instance_count: d.instance_count,
        })
        .collect();

    Ok(extensions)
}

/// Activate an installed extension (load its WASM sandbox).
/// If the extension declares MCP servers, they are automatically registered.
#[tauri::command]
pub async fn activate_extension(
    state: State<'_, AppState>,
    extension_id: String,
) -> Result<(), String> {
    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    let ext_id_for_mcp = extension_id.clone();

    // 1. Activate the extension
    let mcp_servers = tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(async {
            host.0.activate(&extension_id).await?;
            Ok::<_, omni_extensions::error::ExtensionError>(
                host.0.get_mcp_servers(&extension_id).await,
            )
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // 2. Auto-register extension's MCP servers
    if !mcp_servers.is_empty() {
        let mcp = state.mcp_manager.clone();
        let event_bus = state.event_bus.clone();
        for decl in mcp_servers {
            // Prefix server name with extension ID to avoid collisions
            let server_name = format!("{}:{}", ext_id_for_mcp, decl.name);
            let config = omni_llm::mcp::McpServerConfig {
                name: server_name.clone(),
                command: decl.command,
                args: decl.args,
                env: decl.env,
                working_dir: decl.working_dir,
                auto_start: true,
            };
            match mcp.add_server(config).await {
                Ok(()) => {
                    let tool_count = mcp
                        .list_servers()
                        .await
                        .iter()
                        .find(|s| s.name == server_name)
                        .map(|s| s.tool_count)
                        .unwrap_or(0);
                    event_bus.emit(omni_core::events::OmniEvent::McpServerConnected {
                        server_name: server_name.clone(),
                        tool_count,
                    });
                    tracing::info!(
                        extension = %ext_id_for_mcp,
                        server = %server_name,
                        "Auto-registered extension MCP server"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        extension = %ext_id_for_mcp,
                        server = %decl.name,
                        "Failed to auto-register MCP server: {}",
                        e
                    );
                }
            }
        }
    }
    Ok(())
}

/// Deactivate a running extension (stop its sandbox, keep installed).
/// If the extension had MCP servers registered, they are automatically removed.
#[tauri::command]
pub async fn deactivate_extension(
    state: State<'_, AppState>,
    extension_id: String,
) -> Result<(), String> {
    // 1. Get MCP servers before deactivation (so we know what to remove)
    let mcp_servers = {
        let ext_host = state.extension_host_clone();
        let ext_id = extension_id.clone();
        let handle = tauri::async_runtime::handle();
        tokio::task::spawn_blocking(move || {
            let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
            handle.block_on(host.0.get_mcp_servers(&ext_id))
        })
        .await
        .map_err(|e| e.to_string())?
    };

    // 2. Deactivate the extension
    let ext_host = state.extension_host_clone();
    let ext_id_for_deactivate = extension_id.clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.deactivate(&ext_id_for_deactivate))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // 3. Remove extension's MCP servers
    if !mcp_servers.is_empty() {
        let mcp = state.mcp_manager.clone();
        let event_bus = state.event_bus.clone();
        for decl in mcp_servers {
            let server_name = format!("{}:{}", extension_id, decl.name);
            if let Err(e) = mcp.remove_server(&server_name).await {
                tracing::warn!(
                    extension = %extension_id,
                    server = %server_name,
                    "Failed to remove extension MCP server: {}",
                    e
                );
            } else {
                event_bus.emit(omni_core::events::OmniEvent::McpServerDisconnected {
                    server_name: server_name.clone(),
                });
                tracing::info!(
                    extension = %extension_id,
                    server = %server_name,
                    "Auto-removed extension MCP server"
                );
            }
        }
    }

    // 4. Remove extension's channel bindings
    cleanup_bindings_for_extension(&state, &extension_id).await;

    Ok(())
}

/// Uninstall an extension completely (deactivate, revoke permissions, delete files).
/// If the extension had MCP servers registered, they are automatically removed.
#[tauri::command]
pub async fn uninstall_extension(
    state: State<'_, AppState>,
    extension_id: String,
) -> Result<(), String> {
    // 1. Get MCP servers before uninstall (so we know what to remove)
    let mcp_servers = {
        let ext_host = state.extension_host_clone();
        let ext_id = extension_id.clone();
        let handle = tauri::async_runtime::handle();
        tokio::task::spawn_blocking(move || {
            let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
            handle.block_on(host.0.get_mcp_servers(&ext_id))
        })
        .await
        .map_err(|e| e.to_string())?
    };

    // 2. Uninstall (deactivates, revokes permissions, deletes files)
    let ext_host = state.extension_host_clone();
    let ext_id_for_uninstall = extension_id.clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.uninstall(&ext_id_for_uninstall))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // 3. Remove extension's channel bindings
    cleanup_bindings_for_extension(&state, &extension_id).await;

    // 4. Remove extension's MCP servers
    if !mcp_servers.is_empty() {
        let mcp = state.mcp_manager.clone();
        let event_bus = state.event_bus.clone();
        for decl in mcp_servers {
            let server_name = format!("{}:{}", extension_id, decl.name);
            if let Err(e) = mcp.remove_server(&server_name).await {
                tracing::warn!(
                    extension = %extension_id,
                    server = %server_name,
                    "Failed to remove extension MCP server during uninstall: {}",
                    e
                );
            } else {
                event_bus.emit(omni_core::events::OmniEvent::McpServerDisconnected {
                    server_name,
                });
            }
        }
    }
    Ok(())
}

/// Toggle the enabled state of an installed extension.
/// Manages MCP server lifecycle: registers on enable, removes on disable.
#[tauri::command]
pub async fn toggle_extension_enabled(
    state: State<'_, AppState>,
    extension_id: String,
    enabled: bool,
) -> Result<(), String> {
    // Get MCP servers before toggling (needed for both enable and disable paths)
    let mcp_servers = {
        let ext_host = state.extension_host_clone();
        let ext_id = extension_id.clone();
        let handle = tauri::async_runtime::handle();
        tokio::task::spawn_blocking(move || {
            let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
            handle.block_on(host.0.get_mcp_servers(&ext_id))
        })
        .await
        .map_err(|e| e.to_string())?
    };

    // Toggle enabled state (activates/deactivates internally)
    let ext_host = state.extension_host_clone();
    let ext_id_for_toggle = extension_id.clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.set_enabled(&ext_id_for_toggle, enabled))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Manage MCP server lifecycle
    if !mcp_servers.is_empty() {
        let mcp = state.mcp_manager.clone();
        let event_bus = state.event_bus.clone();
        if enabled {
            // Register MCP servers on enable
            for decl in mcp_servers {
                let server_name = format!("{}:{}", extension_id, decl.name);
                let config = omni_llm::mcp::McpServerConfig {
                    name: server_name.clone(),
                    command: decl.command,
                    args: decl.args,
                    env: decl.env,
                    working_dir: decl.working_dir,
                    auto_start: true,
                };
                match mcp.add_server(config).await {
                    Ok(()) => {
                        let tool_count = mcp
                            .list_servers()
                            .await
                            .iter()
                            .find(|s| s.name == server_name)
                            .map(|s| s.tool_count)
                            .unwrap_or(0);
                        event_bus.emit(omni_core::events::OmniEvent::McpServerConnected {
                            server_name,
                            tool_count,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            extension = %extension_id,
                            "Failed to register MCP server on enable: {}",
                            e
                        );
                    }
                }
            }
        } else {
            // Remove MCP servers on disable
            for decl in mcp_servers {
                let server_name = format!("{}:{}", extension_id, decl.name);
                if let Err(e) = mcp.remove_server(&server_name).await {
                    tracing::warn!(
                        extension = %extension_id,
                        server = %server_name,
                        "Failed to remove MCP server on disable: {}",
                        e
                    );
                } else {
                    event_bus.emit(omni_core::events::OmniEvent::McpServerDisconnected {
                        server_name,
                    });
                }
            }
        }
    }

    // Remove channel bindings on disable
    if !enabled {
        cleanup_bindings_for_extension(&state, &extension_id).await;
    }

    Ok(())
}

/// Get an extension's configuration value.
#[tauri::command]
pub async fn extension_config_get(
    state: State<'_, AppState>,
    extension_id: String,
    key: String,
) -> Result<Option<String>, String> {
    let db = state.db.clone();
    let config_key = format!("_config.{}", key);
    tokio::task::spawn_blocking(move || {
        let db = db.lock().map_err(|e| format!("Database lock poisoned: {}", e))?;
        db.get_extension_state(&extension_id, &config_key)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Set an extension's configuration value.
#[tauri::command]
pub async fn extension_config_set(
    state: State<'_, AppState>,
    extension_id: String,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = state.db.clone();
    let config_key = format!("_config.{}", key);
    tokio::task::spawn_blocking(move || {
        let db = db.lock().map_err(|e| format!("Database lock poisoned: {}", e))?;
        db.set_extension_state(&extension_id, &config_key, &value)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get filtered audit log entries.
#[tauri::command]
pub async fn get_audit_log(
    state: State<'_, AppState>,
    extension_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<AuditEntryDto>, String> {
    let db = state.db.clone();
    let records = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        if let Some(ext_id) = extension_id.as_deref() {
            db.query_audit_log_filtered(Some(ext_id), None, None, None, None, limit.unwrap_or(100))
        } else {
            db.get_audit_log(limit.unwrap_or(100))
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(records
        .into_iter()
        .map(|r| AuditEntryDto {
            timestamp: r.timestamp,
            event_type: r.event_type,
            extension_id: r.extension_id.unwrap_or_default(),
            capability: r.capability.unwrap_or_default(),
            decision: r.decision.unwrap_or_default(),
        })
        .collect())
}

/// Get Guardian scan metrics.
#[tauri::command]
pub async fn get_guardian_metrics(
    state: State<'_, AppState>,
) -> Result<GuardianMetricsDto, String> {
    let snapshot = state.guardian.metrics().snapshot();

    // Also get DB stats
    let db = state.db.clone();
    let db_stats = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.get_guardian_stats()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let avg_scan_ms = if snapshot.scan_count > 0 {
        (snapshot.total_scan_us as f64 / snapshot.scan_count as f64) / 1000.0
    } else {
        0.0
    };

    Ok(GuardianMetricsDto {
        scan_count: snapshot.scan_count,
        block_count: snapshot.block_count,
        signature_blocks: snapshot.signature_blocks,
        heuristic_blocks: snapshot.heuristic_blocks,
        ml_blocks: snapshot.ml_blocks,
        policy_blocks: snapshot.policy_blocks,
        avg_scan_ms,
        total_scans_db: db_stats.total_scans as u64,
        total_blocked_db: db_stats.total_blocked as u64,
    })
}

/// Override a Guardian block.
#[tauri::command]
pub async fn guardian_override(
    state: State<'_, AppState>,
    scan_id: String,
) -> Result<(), String> {
    state
        .guardian
        .override_block(&scan_id)
        .await
        .map(|_| ())
        .ok_or_else(|| format!("No pending block with scan_id: {}", scan_id))
}

/// Update application settings.
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings_json: String,
) -> Result<(), String> {
    let partial: serde_json::Value =
        serde_json::from_str(&settings_json).map_err(|e| e.to_string())?;

    let mut config = state.config.write().await;

    // Apply partial updates
    if let Some(theme) = partial.get("theme").and_then(|v| v.as_str()) {
        config.ui.theme = theme.to_string();
    }
    if let Some(font_size) = partial.get("fontSize").and_then(|v| v.as_u64()) {
        config.ui.font_size = font_size as u32;
    }
    if let Some(show_feed) = partial.get("showActionFeed").and_then(|v| v.as_bool()) {
        config.ui.show_action_feed = show_feed;
    }
    if let Some(sensitivity) = partial.get("guardianSensitivity").and_then(|v| v.as_str()) {
        config.guardian.sensitivity = sensitivity.to_string();
        // Update live Guardian thresholds immediately
        if let Some(s) = omni_guardian::Sensitivity::from_str_config(sensitivity) {
            state.guardian.set_sensitivity(s);
        }
    }
    if let Some(telemetry) = partial.get("telemetry").and_then(|v| v.as_bool()) {
        config.general.telemetry = telemetry;
    }
    if let Some(accent) = partial.get("accentColor").and_then(|v| v.as_str()) {
        config.ui.accent_color = accent.to_string();
    }
    if let Some(ff) = partial.get("fontFamily").and_then(|v| v.as_str()) {
        config.ui.font_family = ff.to_string();
    }
    if let Some(lh) = partial.get("lineHeight").and_then(|v| v.as_str()) {
        config.ui.line_height = lh.to_string();
    }
    if let Some(density) = partial.get("uiDensity").and_then(|v| v.as_str()) {
        config.ui.ui_density = density.to_string();
    }
    if let Some(sw) = partial.get("sidebarWidth").and_then(|v| v.as_u64()) {
        config.ui.sidebar_width = sw as u32;
    }
    if let Some(ms) = partial.get("messageStyle").and_then(|v| v.as_str()) {
        config.ui.message_style = ms.to_string();
    }
    if let Some(mmw) = partial.get("maxMessageWidth").and_then(|v| v.as_u64()) {
        config.ui.max_message_width = mmw as u32;
    }
    if let Some(ct) = partial.get("codeTheme").and_then(|v| v.as_str()) {
        config.ui.code_theme = ct.to_string();
    }
    if let Some(st) = partial.get("showTimestamps").and_then(|v| v.as_bool()) {
        config.ui.show_timestamps = st;
    }
    if let Some(br) = partial.get("borderRadius").and_then(|v| v.as_u64()) {
        config.ui.border_radius = br as u32;
    }
    if let Some(ra) = partial.get("reduceAnimations").and_then(|v| v.as_bool()) {
        config.ui.reduce_animations = ra;
    }
    if let Some(hc) = partial.get("highContrast").and_then(|v| v.as_bool()) {
        config.ui.high_contrast = hc;
    }
    if let Some(au) = partial.get("autoUpdate").and_then(|v| v.as_bool()) {
        config.ui.auto_update = au;
    }

    // Save to disk
    config
        .save(&state.paths.config_file())
        .map_err(|e| e.to_string())?;

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ConfigChanged);

    Ok(())
}

/// Get pending Guardian blocks.
#[tauri::command]
pub async fn get_pending_blocks(
    state: State<'_, AppState>,
) -> Result<Vec<PendingBlockDto>, String> {
    let blocks = state.guardian.get_pending_blocks().await;
    Ok(blocks
        .into_iter()
        .map(|b| PendingBlockDto {
            scan_id: b.scan_id,
            scan_type: b.scan_type,
            layer: b.layer,
            reason: b.reason,
            confidence: b.confidence,
            content_preview: b.content_preview,
            created_at: b.created_at.to_rfc3339(),
        })
        .collect())
}

// ─── Channel Commands ───────────────────────────────────────────────

/// List all registered channel instances and their status.
#[tauri::command]
pub async fn channel_list(state: State<'_, AppState>) -> Result<Vec<ChannelDto>, String> {
    let channels = state.channel_manager.list_channels().await;
    Ok(channels
        .into_iter()
        .map(|c| ChannelDto {
            id: c.id,
            channel_type: c.channel_type,
            instance_id: c.instance_id,
            name: c.name,
            status: c.status.to_string(),
            features: ChannelFeaturesDto {
                direct_messages: c.features.direct_messages,
                group_messages: c.features.group_messages,
                media_attachments: c.features.media_attachments,
                reactions: c.features.reactions,
                read_receipts: c.features.read_receipts,
                typing_indicators: c.features.typing_indicators,
                threads: c.features.threads,
            },
        })
        .collect())
}

/// List all registered channel type factories.
#[tauri::command]
pub async fn channel_list_types(
    state: State<'_, AppState>,
) -> Result<Vec<ChannelTypeDto>, String> {
    let types = state.channel_manager.list_channel_types().await;
    Ok(types
        .into_iter()
        .map(|t| ChannelTypeDto {
            channel_type: t.channel_type,
            name: t.name,
            features: ChannelFeaturesDto {
                direct_messages: t.features.direct_messages,
                group_messages: t.features.group_messages,
                media_attachments: t.features.media_attachments,
                reactions: t.features.reactions,
                read_receipts: t.features.read_receipts,
                typing_indicators: t.features.typing_indicators,
                threads: t.features.threads,
            },
        })
        .collect())
}

/// Create a new channel instance (e.g. a second Discord bot).
#[tauri::command]
pub async fn channel_create_instance(
    state: State<'_, AppState>,
    channel_type: String,
    instance_id: String,
    display_name: Option<String>,
) -> Result<String, String> {
    // Create in ChannelManager
    state
        .channel_manager
        .create_instance(&channel_type, &instance_id)
        .await
        .map_err(|e| e.to_string())?;

    let compound_key = format!("{}:{}", channel_type, instance_id);

    // Persist to database
    let db = state.db.clone();
    let ct = channel_type.clone();
    let iid = instance_id.clone();
    let dn = display_name.clone();
    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.upsert_channel_instance(&ct, &iid, dn.as_deref(), None, None, false)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Emit event
    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ChannelInstanceCreated {
            channel_id: compound_key.clone(),
            channel_type,
            instance_id,
        });

    Ok(compound_key)
}

/// Remove a channel instance.
#[tauri::command]
pub async fn channel_remove_instance(
    state: State<'_, AppState>,
    channel_type: String,
    instance_id: String,
) -> Result<(), String> {
    let compound_key = format!("{}:{}", channel_type, instance_id);

    // Remove from ChannelManager
    state
        .channel_manager
        .remove_instance(&compound_key)
        .await
        .map_err(|e| e.to_string())?;

    // Remove from database
    let db = state.db.clone();
    let ct = channel_type.clone();
    let iid = instance_id.clone();
    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.delete_channel_instance(&ct, &iid)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Emit event
    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ChannelInstanceRemoved {
            channel_id: compound_key,
        });

    Ok(())
}

/// Connect a channel plugin with configuration.
#[tauri::command]
pub async fn channel_connect(
    state: State<'_, AppState>,
    channel_id: String,
    settings: HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    let config = omni_channels::ChannelConfig { settings };
    state
        .channel_manager
        .connect(&channel_id, config)
        .await
        .map_err(|e| e.to_string())
}

/// Disconnect a channel plugin.
#[tauri::command]
pub async fn channel_disconnect(
    state: State<'_, AppState>,
    channel_id: String,
) -> Result<(), String> {
    state
        .channel_manager
        .disconnect(&channel_id)
        .await
        .map_err(|e| e.to_string())
}

/// Login/authenticate with a channel.
#[tauri::command]
pub async fn channel_login(
    state: State<'_, AppState>,
    channel_id: String,
    credential_type: String,
    data: HashMap<String, String>,
) -> Result<String, String> {
    let credentials = omni_channels::ChannelCredentials {
        credential_type,
        data,
    };
    let status = state
        .channel_manager
        .login(&channel_id, credentials)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_string(&status).unwrap_or_else(|_| "\"Unknown\"".to_string()))
}

/// Send a message via a channel.
#[tauri::command]
pub async fn channel_send(
    state: State<'_, AppState>,
    channel_id: String,
    recipient: String,
    text: String,
    media_url: Option<String>,
    reply_to: Option<String>,
) -> Result<(), String> {
    let message = omni_channels::OutgoingMessage {
        text,
        media_url,
        reply_to,
        thread_id: None,
    };
    state
        .channel_manager
        .send_message(&channel_id, &recipient, message)
        .await
        .map_err(|e| e.to_string())
}

/// Get the API key for a channel instance (if applicable).
/// Currently only WebChat channels return a key.
#[tauri::command]
pub async fn channel_get_api_key(
    state: State<'_, AppState>,
    channel_id: String,
) -> Result<Option<String>, String> {
    state
        .channel_manager
        .get_channel_api_key(&channel_id)
        .await
        .map_err(|e| e.to_string())
}

// ─── Binding Commands ───────────────────────────────────────────────

/// Add or update a channel-extension binding.
#[tauri::command]
pub async fn binding_add(
    state: State<'_, AppState>,
    channel_instance: String,
    extension_id: String,
    peer_filter: Option<String>,
    group_filter: Option<String>,
    priority: Option<i32>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let priority = priority.unwrap_or(0);

    // Add to registry
    state.binding_registry.add(omni_channels::bindings::ChannelBinding {
        id: id.clone(),
        channel_instance: channel_instance.clone(),
        extension_id: extension_id.clone(),
        peer_filter: peer_filter.clone(),
        group_filter: group_filter.clone(),
        priority,
        enabled: true,
    });

    // Persist to database
    let db = state.db.clone();
    let bid = id.clone();
    let ci = channel_instance.clone();
    let eid = extension_id.clone();
    let pf = peer_filter.clone();
    let gf = group_filter.clone();
    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.upsert_binding(&bid, &ci, &eid, pf.as_deref(), gf.as_deref(), priority, true)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Emit event
    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ChannelBindingAdded {
            binding_id: id.clone(),
            channel_instance,
            extension_id,
        });

    Ok(id)
}

/// Remove a channel-extension binding.
#[tauri::command]
pub async fn binding_remove(
    state: State<'_, AppState>,
    binding_id: String,
) -> Result<(), String> {
    // Remove from registry
    state.binding_registry.remove(&binding_id);

    // Remove from database
    let db = state.db.clone();
    let bid = binding_id.clone();
    tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.delete_binding(&bid)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Emit event
    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ChannelBindingRemoved {
            binding_id,
        });

    Ok(())
}

/// List all channel-extension bindings.
#[tauri::command]
pub async fn binding_list(
    state: State<'_, AppState>,
) -> Result<Vec<BindingDto>, String> {
    let bindings = state.binding_registry.list();
    Ok(bindings
        .into_iter()
        .map(|b| BindingDto {
            id: b.id,
            channel_instance: b.channel_instance,
            extension_id: b.extension_id,
            peer_filter: b.peer_filter,
            group_filter: b.group_filter,
            priority: b.priority,
            enabled: b.enabled,
        })
        .collect())
}

/// List bindings for a specific extension.
#[tauri::command]
pub async fn binding_list_for_extension(
    state: State<'_, AppState>,
    extension_id: String,
) -> Result<Vec<BindingDto>, String> {
    let bindings = state.binding_registry.list_for_extension(&extension_id);
    Ok(bindings
        .into_iter()
        .map(|b| BindingDto {
            id: b.id,
            channel_instance: b.channel_instance,
            extension_id: b.extension_id,
            peer_filter: b.peer_filter,
            group_filter: b.group_filter,
            priority: b.priority,
            enabled: b.enabled,
        })
        .collect())
}

// ─── Extension Instance Commands ────────────────────────────────────

/// Create a new named instance of an installed extension.
#[tauri::command]
pub async fn create_extension_instance(
    state: State<'_, AppState>,
    extension_id: String,
    instance_name: String,
    display_name: Option<String>,
) -> Result<String, String> {
    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.create_instance(&extension_id, &instance_name, display_name))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// Delete an extension instance.
#[tauri::command]
pub async fn delete_extension_instance(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    // Clean up bindings for this instance
    cleanup_bindings_for_extension(&state, &instance_id).await;

    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.delete_instance(&instance_id))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// List all extension instances, optionally filtered by extension_id.
#[tauri::command]
pub async fn list_extension_instances(
    state: State<'_, AppState>,
    extension_id: Option<String>,
) -> Result<Vec<ExtensionInstanceDto>, String> {
    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    let dtos = tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        let metas = handle.block_on(host.0.list_instances(extension_id.as_deref()));
        let details = handle.block_on(host.0.list_installed_details());

        metas
            .into_iter()
            .map(|(iid, meta)| {
                let ext_detail = details.iter().find(|d| d.id == meta.extension_id);
                let active = handle.block_on(host.0.is_active(&iid));

                ExtensionInstanceDto {
                    instance_id: iid,
                    extension_id: meta.extension_id,
                    instance_name: meta.instance_name,
                    display_name: meta.display_name,
                    enabled: meta.enabled,
                    active,
                    tools: ext_detail.map(|d| d.tools.clone()).unwrap_or_default(),
                    permissions: ext_detail.map(|d| d.permissions.clone()).unwrap_or_default(),
                }
            })
            .collect::<Vec<ExtensionInstanceDto>>()
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(dtos)
}

/// Update an extension instance's display name.
#[tauri::command]
pub async fn update_extension_instance(
    state: State<'_, AppState>,
    instance_id: String,
    display_name: Option<String>,
) -> Result<(), String> {
    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.update_instance(&instance_id, display_name))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// Activate an extension instance.
#[tauri::command]
pub async fn activate_extension_instance(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.activate(&instance_id))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// Deactivate an extension instance.
#[tauri::command]
pub async fn deactivate_extension_instance(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    // Clean up bindings for this instance
    cleanup_bindings_for_extension(&state, &instance_id).await;

    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.deactivate(&instance_id))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// Toggle the enabled state of an extension instance.
#[tauri::command]
pub async fn toggle_instance_enabled(
    state: State<'_, AppState>,
    instance_id: String,
    enabled: bool,
) -> Result<(), String> {
    let ext_host = state.extension_host_clone();
    let handle = tauri::async_runtime::handle();
    tokio::task::spawn_blocking(move || {
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        handle.block_on(host.0.set_instance_enabled(&instance_id, enabled))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

// ─── Provider Commands ──────────────────────────────────────────────

/// List all supported provider types (static metadata).
#[tauri::command]
pub async fn provider_list_types() -> Result<Vec<ProviderTypeInfoDto>, String> {
    Ok(vec![
        ProviderTypeInfoDto {
            provider_type: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            auth_type: "api_key".to_string(),
            env_var_hint: Some("OPENAI_API_KEY".to_string()),
            default_endpoint: Some("https://api.openai.com/v1".to_string()),
            description: "GPT-5.2, GPT-5.2 Pro, GPT-5-mini, GPT-5-nano".to_string(),
        },
        ProviderTypeInfoDto {
            provider_type: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            auth_type: "api_key".to_string(),
            env_var_hint: Some("ANTHROPIC_API_KEY".to_string()),
            default_endpoint: Some("https://api.anthropic.com/v1".to_string()),
            description: "Claude Opus 4.6, Sonnet 4.6, Haiku 4.5".to_string(),
        },
        ProviderTypeInfoDto {
            provider_type: "google".to_string(),
            display_name: "Google Gemini".to_string(),
            auth_type: "api_key".to_string(),
            env_var_hint: Some("GOOGLE_API_KEY".to_string()),
            default_endpoint: None,
            description: "Gemini 3.1 Pro, Gemini 3 Flash, Gemini 2.5 Pro".to_string(),
        },
        ProviderTypeInfoDto {
            provider_type: "ollama".to_string(),
            display_name: "Ollama".to_string(),
            auth_type: "none".to_string(),
            env_var_hint: None,
            default_endpoint: Some("http://localhost:11434".to_string()),
            description: "Local models via Ollama. No API key required.".to_string(),
        },
        ProviderTypeInfoDto {
            provider_type: "bedrock".to_string(),
            display_name: "AWS Bedrock".to_string(),
            auth_type: "aws".to_string(),
            env_var_hint: None,
            default_endpoint: None,
            description: "AWS-hosted models (Claude, Titan, Llama)".to_string(),
        },
        ProviderTypeInfoDto {
            provider_type: "custom".to_string(),
            display_name: "Custom / OpenAI-Compatible".to_string(),
            auth_type: "api_key".to_string(),
            env_var_hint: None,
            default_endpoint: None,
            description: "Any OpenAI-compatible API endpoint".to_string(),
        },
    ])
}

/// List all configured providers with credential status.
#[tauri::command]
pub async fn provider_list(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderDto>, String> {
    let config = state.config.read().await;
    let providers: Vec<_> = config
        .providers
        .iter()
        .map(|(id, cfg)| (id.clone(), cfg.clone()))
        .collect();
    drop(config);

    let vault = state.llm_bridge.vault().clone();
    let result = tokio::task::spawn_blocking(move || {
        providers
            .into_iter()
            .map(|(id, cfg)| {
                let has_credential = match vault.retrieve(&id) {
                    Ok(opt) => opt.is_some(),
                    Err(e) => {
                        tracing::warn!(provider = %id, "Failed to check credential: {}", e);
                        false
                    }
                };

                let (auth_type, env_var_hint) = match cfg.provider_type.as_str() {
                    "openai" => ("api_key", Some("OPENAI_API_KEY")),
                    "anthropic" => ("api_key", Some("ANTHROPIC_API_KEY")),
                    "google" => ("api_key", Some("GOOGLE_API_KEY")),
                    "ollama" => ("none", None),
                    "bedrock" => ("aws", None),
                    "custom" => ("api_key", None),
                    _ => ("api_key", None),
                };

                let display_name = match cfg.provider_type.as_str() {
                    "openai" => "OpenAI",
                    "anthropic" => "Anthropic",
                    "google" => "Google Gemini",
                    "ollama" => "Ollama",
                    "bedrock" => "AWS Bedrock",
                    "custom" => "Custom / OpenAI-Compatible",
                    _ => "Unknown",
                };

                ProviderDto {
                    id,
                    provider_type: cfg.provider_type,
                    display_name: display_name.to_string(),
                    default_model: cfg.default_model,
                    endpoint: cfg.endpoint,
                    max_tokens: cfg.max_tokens,
                    temperature: cfg.temperature,
                    enabled: cfg.enabled,
                    has_credential,
                    auth_type: auth_type.to_string(),
                    env_var_hint: env_var_hint.map(|s| s.to_string()),
                }
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Add a new provider configuration.
#[tauri::command]
pub async fn provider_add(
    state: State<'_, AppState>,
    id: String,
    provider_type: String,
    default_model: Option<String>,
    endpoint: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<(), String> {
    let mut config = state.config.write().await;

    if config.providers.contains_key(&id) {
        return Err(format!("Provider '{}' already exists", id));
    }

    let provider_cfg = omni_core::config::ProviderConfig {
        provider_type,
        default_model,
        endpoint,
        max_tokens,
        temperature: temperature.map(|t| (t * 10.0).round() / 10.0),
        enabled: true,
        transport: None,
    };

    // Register the provider with the LLM bridge so it can serve requests immediately
    register_provider_from_config(&state.llm_bridge, &id, &provider_cfg).await;

    config.providers.insert(id, provider_cfg);

    config
        .save(&state.paths.config_file())
        .map_err(|e| e.to_string())?;

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ConfigChanged);

    Ok(())
}

/// Update an existing provider's settings.
#[tauri::command]
pub async fn provider_update(
    state: State<'_, AppState>,
    id: String,
    default_model: Option<String>,
    endpoint: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let mut config = state.config.write().await;

    let provider = config
        .providers
        .get_mut(&id)
        .ok_or_else(|| format!("Provider '{}' not found", id))?;

    if let Some(model) = default_model {
        provider.default_model = if model.is_empty() { None } else { Some(model) };
    }
    if let Some(ep) = endpoint {
        provider.endpoint = if ep.is_empty() { None } else { Some(ep) };
    }
    if max_tokens.is_some() {
        provider.max_tokens = max_tokens;
    }
    if let Some(t) = temperature {
        // Round to 1 decimal place to avoid f32 precision artifacts (e.g. 0.699999988 → 0.7)
        provider.temperature = Some((t * 10.0).round() / 10.0);
    }
    if let Some(en) = enabled {
        provider.enabled = en;
    }

    if let Some(temp) = provider.temperature {
        if !(0.0..=2.0).contains(&temp) {
            return Err(format!("Temperature {} out of range [0.0, 2.0]", temp));
        }
    }

    // Re-register the provider with the LLM bridge to pick up changed settings
    if provider.enabled {
        register_provider_from_config(&state.llm_bridge, &id, provider).await;
    }

    config
        .save(&state.paths.config_file())
        .map_err(|e| e.to_string())?;

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ConfigChanged);

    Ok(())
}

/// Remove a provider and its credential.
#[tauri::command]
pub async fn provider_remove(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut config = state.config.write().await;

    if config.providers.remove(&id).is_none() {
        return Err(format!("Provider '{}' not found", id));
    }

    config
        .save(&state.paths.config_file())
        .map_err(|e| e.to_string())?;
    drop(config);

    // Also remove credential from vault
    let vault = state.llm_bridge.vault().clone();
    let pid = id.clone();
    tokio::task::spawn_blocking(move || vault.delete(&pid))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ConfigChanged);

    Ok(())
}

/// Store a credential (API key or AWS) in the OS keyring.
#[tauri::command]
pub async fn provider_set_credential(
    state: State<'_, AppState>,
    provider_id: String,
    credential_type: String,
    api_key: Option<String>,
    aws_access_key_id: Option<String>,
    aws_secret_access_key: Option<String>,
    aws_session_token: Option<String>,
    aws_region: Option<String>,
) -> Result<(), String> {
    let credential = match credential_type.as_str() {
        "api_key" => {
            let key = api_key.ok_or("API key is required")?;
            if key.is_empty() {
                return Err("API key cannot be empty".to_string());
            }
            omni_llm::Credential::ApiKey { key }
        }
        "aws" => {
            let access_key_id =
                aws_access_key_id.ok_or("AWS access key ID is required")?;
            let secret_access_key =
                aws_secret_access_key.ok_or("AWS secret access key is required")?;
            let region = aws_region.unwrap_or_else(|| "us-east-1".to_string());
            omni_llm::Credential::AwsCredentials {
                access_key_id,
                secret_access_key,
                session_token: aws_session_token,
                region,
            }
        }
        _ => return Err(format!("Unknown credential type: {}", credential_type)),
    };

    let vault = state.llm_bridge.vault().clone();
    let pid = provider_id.clone();
    let pid2 = pid.clone();
    tokio::task::spawn_blocking(move || {
        tracing::info!(provider = %pid, "Storing credential in OS keyring");
        vault.store(&pid, &credential)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| {
        tracing::error!(provider = %pid2, "Failed to store credential: {}", e);
        e.to_string()
    })?;

    tracing::info!(provider = %pid2, "Credential stored successfully");
    Ok(())
}

/// Remove a credential from the OS keyring.
#[tauri::command]
pub async fn provider_delete_credential(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    let vault = state.llm_bridge.vault().clone();
    tokio::task::spawn_blocking(move || vault.delete(&provider_id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Test a credential by attempting to list models.
#[tauri::command]
pub async fn provider_test_credential(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<String, String> {
    match state.llm_bridge.list_models(&provider_id).await {
        Ok(models) => Ok(format!("Success! Found {} models.", models.len())),
        Err(e) => Err(format!("Credential test failed: {}", e)),
    }
}

/// Get current UI settings for frontend hydration.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<String, String> {
    let config = state.config.read().await;
    // Start from UI config, then inject guardian sensitivity
    let mut value = serde_json::to_value(&config.ui).map_err(|e| e.to_string())?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "guardian_sensitivity".to_string(),
            serde_json::Value::String(config.guardian.sensitivity.clone()),
        );
    }
    serde_json::to_string(&value).map_err(|e| e.to_string())
}

// ─── Marketplace Commands ───────────────────────────────────────

/// Search the marketplace for extensions.
#[tauri::command]
pub async fn marketplace_search(
    state: State<'_, AppState>,
    query: Option<String>,
    category: Option<String>,
    sort: Option<String>,
    trust: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
    force_refresh: Option<bool>,
) -> Result<MarketplaceSearchResultDto, String> {
    tracing::info!("[CMD] marketplace_search ENTER");
    let result = match state
        .marketplace
        .search(
            query.as_deref(),
            category.as_deref(),
            sort.as_deref(),
            trust.as_deref(),
            page,
            limit,
            force_refresh.unwrap_or(false),
        )
        .await
    {
        Ok(r) => {
            tracing::info!("[CMD] marketplace_search OK -- {} results", r.extensions.len());
            r
        }
        Err(e) => {
            tracing::error!("[CMD] marketplace_search FAILED: {}", e);
            return Err(e);
        }
    };

    Ok(MarketplaceSearchResultDto {
        extensions: result
            .extensions
            .into_iter()
            .map(|e| {
                let publisher_name = e
                    .publisher
                    .as_ref()
                    .and_then(|p| p.display_name.clone())
                    .or_else(|| e.publisher.as_ref().map(|p| p.username.clone()))
                    .unwrap_or_else(|| "Unknown".to_string());
                let publisher_verified = e
                    .publisher
                    .as_ref()
                    .map(|p| p.verified_publisher)
                    .unwrap_or(false);

                MarketplaceExtensionDto {
                    id: e.id,
                    name: e.name,
                    short_description: e.short_description.unwrap_or_default(),
                    icon_url: e.icon_url,
                    categories: e.categories,
                    tags: e.tags,
                    trust_level: e.trust_level,
                    latest_version: e.latest_version.unwrap_or_else(|| "0.0.0".to_string()),
                    total_downloads: e.total_downloads,
                    average_rating: e.average_rating,
                    review_count: e.review_count,
                    publisher_name,
                    publisher_verified,
                }
            })
            .collect(),
        total: result.total,
        page: result.page,
        limit: result.limit,
        total_pages: result.total_pages,
    })
}

/// Get full detail for a marketplace extension.
#[tauri::command]
pub async fn marketplace_get_detail(
    state: State<'_, AppState>,
    extension_id: String,
    force_refresh: Option<bool>,
) -> Result<MarketplaceDetailDto, String> {
    tracing::info!("[CMD] marketplace_get_detail ENTER -- id={}", extension_id);
    let detail = match state.marketplace.get_detail(&extension_id, force_refresh.unwrap_or(false)).await {
        Ok(d) => {
            tracing::info!("[CMD] marketplace_get_detail OK -- {}", extension_id);
            d
        }
        Err(e) => {
            tracing::error!("[CMD] marketplace_get_detail FAILED -- {}: {}", extension_id, e);
            return Err(e);
        }
    };

    let publisher_name = detail
        .publisher
        .as_ref()
        .and_then(|p| p.display_name.clone())
        .or_else(|| detail.publisher.as_ref().map(|p| p.username.clone()))
        .unwrap_or_else(|| "Unknown".to_string());
    let publisher_verified = detail
        .publisher
        .as_ref()
        .map(|p| p.verified_publisher)
        .unwrap_or(false);

    let latest = detail.latest.as_ref();

    // Extract tool/permission names from JSON arrays
    let tools = latest
        .and_then(|v| v.tools.as_ref())
        .and_then(|t| serde_json::from_value::<Vec<serde_json::Value>>(t.clone()).ok())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let permissions = latest
        .and_then(|v| v.permissions.as_ref())
        .and_then(|p| serde_json::from_value::<Vec<serde_json::Value>>(p.clone()).ok())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.get("capability")
                        .and_then(|c| c.as_str())
                        .map(String::from)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let short_desc = detail.short_description.clone().unwrap_or_default();
    let description = detail
        .description
        .clone()
        .unwrap_or_else(|| short_desc.clone());

    Ok(MarketplaceDetailDto {
        id: detail.id,
        name: detail.name,
        short_description: short_desc,
        description,
        icon_url: detail.icon_url,
        categories: detail.categories,
        tags: detail.tags,
        trust_level: detail.trust_level,
        latest_version: detail
            .latest_version
            .unwrap_or_else(|| "0.0.0".to_string()),
        total_downloads: detail.total_downloads,
        average_rating: detail.average_rating,
        review_count: detail.review_count,
        publisher_name,
        publisher_verified,
        homepage: detail.homepage,
        repository: detail.repository,
        license: detail.license,
        min_omni_version: latest.and_then(|v| v.min_omni_version.clone()),
        tools,
        permissions,
        changelog: latest.and_then(|v| v.changelog.clone()),
        screenshots: detail.screenshots,
        scan_status: latest.and_then(|v| v.scan_status.clone()),
        scan_score: latest.and_then(|v| v.scan_score),
        wasm_size_bytes: latest.and_then(|v| v.wasm_size_bytes),
    })
}

/// Get marketplace categories with extension counts.
#[tauri::command]
pub async fn marketplace_get_categories(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<MarketplaceCategoryDto>, String> {
    tracing::info!("[CMD] marketplace_get_categories ENTER");
    let categories = match state.marketplace.get_categories(force_refresh.unwrap_or(false)).await {
        Ok(c) => {
            tracing::info!("[CMD] marketplace_get_categories OK -- {} categories", c.len());
            c
        }
        Err(e) => {
            tracing::error!("[CMD] marketplace_get_categories FAILED: {}", e);
            return Err(e);
        }
    };
    Ok(categories
        .into_iter()
        .map(|c| MarketplaceCategoryDto {
            id: c.id,
            name: c.name,
            icon: c.icon,
            count: c.count,
        })
        .collect())
}

/// Download and install an extension from the marketplace.
#[tauri::command]
pub async fn marketplace_install(
    state: State<'_, AppState>,
    extension_id: String,
) -> Result<String, String> {
    tracing::info!("[INSTALL] marketplace_install called for {}", extension_id);

    // 1. Get extension detail (for manifest reconstruction) -- always fresh
    let detail = state.marketplace.get_detail(&extension_id, true).await?;

    tracing::info!(
        "[INSTALL] detail received -- latest_version={:?}, latest.version={:?}, latest.manifest_present={}, latest.checksum={:?}",
        detail.latest_version,
        detail.latest.as_ref().map(|v| &v.version),
        detail.latest.as_ref().and_then(|v| v.manifest.as_ref()).is_some(),
        detail.latest.as_ref().and_then(|v| v.checksum.clone()),
    );

    // 2. Download WASM and create temp directory with manifest
    let temp_path = state
        .marketplace
        .download_extension(&extension_id, &detail)
        .await?;

    tracing::info!("[INSTALL] download complete, temp_path={}", temp_path.display());

    // Read back the manifest that was written to verify
    let written_manifest_path = temp_path.join("omni-extension.toml");
    if let Ok(content) = std::fs::read_to_string(&written_manifest_path) {
        // Log first 500 chars of the reconstructed manifest
        let preview: String = content.chars().take(500).collect();
        tracing::info!("[INSTALL] reconstructed manifest:\n{}", preview);
    }

    // 3. Install via existing install_from_path flow
    let source = omni_extensions::host::ExtensionSource::Path(temp_path.clone());
    let ext_host = state.extension_host_clone();
    let rt = tauri::async_runtime::handle();

    let (ext_id, mcp_servers) = tokio::task::spawn_blocking(move || {
        let host = ext_host
            .lock()
            .map_err(|e| format!("Lock poisoned: {}", e))?;

        let id = rt
            .block_on(host.0.install(&source))
            .map_err(|e| format!("Install failed: {}", e))?;

        tracing::info!("[INSTALL] install_from_path succeeded for {}", id);

        // Auto-activate
        if let Err(e) = rt.block_on(host.0.activate(&id)) {
            tracing::warn!(
                extension = %id,
                "Auto-activate after marketplace install failed: {}",
                e
            );
        }

        let mcp = rt.block_on(host.0.get_mcp_servers(&id));
        Ok::<(String, Vec<_>), String>((id, mcp))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| e.to_string())?;

    // 4. Cleanup temp directory (best-effort)
    if let Err(e) = std::fs::remove_dir_all(&temp_path) {
        tracing::warn!("Failed to clean up marketplace temp dir: {}", e);
    }

    // 5. Auto-register extension MCP servers after marketplace install
    if !mcp_servers.is_empty() {
        let mcp = state.mcp_manager.clone();
        let event_bus = state.event_bus.clone();
        for decl in mcp_servers {
            let server_name = format!("{}:{}", ext_id, decl.name);
            let config = omni_llm::mcp::McpServerConfig {
                name: server_name.clone(),
                command: decl.command,
                args: decl.args,
                env: decl.env,
                working_dir: decl.working_dir,
                auto_start: true,
            };
            match mcp.add_server(config).await {
                Ok(()) => {
                    let tool_count = mcp
                        .list_servers()
                        .await
                        .iter()
                        .find(|s| s.name == server_name)
                        .map(|s| s.tool_count)
                        .unwrap_or(0);
                    event_bus.emit(omni_core::events::OmniEvent::McpServerConnected {
                        server_name,
                        tool_count,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        extension = %ext_id,
                        "Failed to auto-register MCP server after marketplace install: {}",
                        e
                    );
                }
            }
        }
    }

    Ok(ext_id)
}

/// Check for available updates on all installed extensions.
#[tauri::command]
pub async fn marketplace_check_updates(
    state: State<'_, AppState>,
) -> Result<Vec<ExtensionUpdateDto>, String> {
    tracing::info!("[CMD] marketplace_check_updates ENTER");

    // 1. Get installed extensions
    let ext_host = state.extension_host_clone();
    let rt = tauri::async_runtime::handle();
    let installed = tokio::task::spawn_blocking(move || {
        tracing::info!("[CMD] marketplace_check_updates -- acquiring extension host lock");
        let host = ext_host.lock().unwrap_or_else(|e| e.into_inner());
        tracing::info!("[CMD] marketplace_check_updates -- lock acquired, listing installed");
        rt.block_on(host.0.list_installed_details())
    })
    .await
    .map_err(|e| {
        tracing::error!("[CMD] marketplace_check_updates -- spawn_blocking failed: {}", e);
        format!("Task join error: {}", e)
    })?;

    tracing::info!("[CMD] marketplace_check_updates -- {} installed extensions", installed.len());

    // 2. For each installed extension, query marketplace for latest version
    let mut updates = Vec::new();
    for ext in installed {
        tracing::info!("[CMD] marketplace_check_updates -- checking {} v{}", ext.id, ext.version);
        match state.marketplace.get_detail(&ext.id, true).await {
            Ok(detail) => {
                let marketplace_version = detail
                    .latest_version
                    .unwrap_or_else(|| "0.0.0".to_string());

                let installed_ver = semver::Version::parse(&ext.version)
                    .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
                let latest_ver = semver::Version::parse(&marketplace_version)
                    .unwrap_or_else(|_| semver::Version::new(0, 0, 0));

                updates.push(ExtensionUpdateDto {
                    extension_id: ext.id,
                    installed_version: ext.version,
                    latest_version: marketplace_version,
                    has_update: latest_ver > installed_ver,
                });
            }
            Err(e) => {
                tracing::warn!("[CMD] marketplace_check_updates -- {} not in marketplace: {}", ext.id, e);
                continue;
            }
        }
    }

    tracing::info!("[CMD] marketplace_check_updates DONE -- {} updates", updates.len());
    Ok(updates)
}

// ─── MCP Server Commands ──────────────────────────────────────────────

/// List all configured MCP servers, merging config with live connection status.
#[tauri::command]
pub async fn mcp_list_servers(state: State<'_, AppState>) -> Result<Vec<McpServerDto>, String> {
    let config = state.config.read().await;
    let connected = state.mcp_manager.list_servers().await;
    let schemas = state.mcp_manager.get_all_schemas().await;

    let mut servers = Vec::new();
    for entry in &config.mcp.servers {
        let connected_info = connected.iter().find(|s| s.name == entry.name);
        let is_connected = connected_info.is_some();
        let tool_count = connected_info.map(|s| s.tool_count).unwrap_or(0);

        // Build tool list from schemas for connected servers
        let tools: Vec<McpToolDto> = if is_connected {
            let prefix = format!("mcp_{}_", entry.name);
            let mcp_prefix = format!("[MCP:{}] ", entry.name);
            schemas
                .iter()
                .filter(|s| s.name.starts_with(&prefix))
                .map(|s| {
                    let name = s.name.strip_prefix(&prefix).unwrap_or(&s.name).to_string();
                    let description = if s.description.starts_with(&mcp_prefix) {
                        Some(s.description[mcp_prefix.len()..].to_string())
                    } else {
                        Some(s.description.clone())
                    };
                    McpToolDto { name, description }
                })
                .collect()
        } else {
            Vec::new()
        };

        servers.push(McpServerDto {
            name: entry.name.clone(),
            status: if is_connected {
                "connected".to_string()
            } else {
                "disconnected".to_string()
            },
            tool_count,
            tools,
            command: entry.command.clone(),
            args: entry.args.clone(),
            env: entry.env.clone(),
            working_dir: entry.working_dir.clone(),
            auto_start: entry.auto_start,
        });
    }

    Ok(servers)
}

/// Add a new MCP server to the configuration and optionally connect.
#[tauri::command]
pub async fn mcp_add_server(
    state: State<'_, AppState>,
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    working_dir: Option<String>,
    auto_start: bool,
    connect_now: bool,
) -> Result<(), String> {
    // Validate and persist to config
    {
        let mut config = state.config.write().await;
        if config.mcp.servers.iter().any(|s| s.name == name) {
            return Err(format!("MCP server '{}' already exists", name));
        }
        config.mcp.servers.push(omni_core::config::McpServerEntry {
            name: name.clone(),
            command: command.clone(),
            args: args.clone(),
            env: env.clone(),
            working_dir: working_dir.clone(),
            transport: "stdio".to_string(),
            url: None,
            auto_start,
        });
        config
            .save(&state.paths.config_file())
            .map_err(|e| e.to_string())?;
    }

    // Optionally connect
    if connect_now {
        let server_config = omni_llm::mcp::McpServerConfig {
            name: name.clone(),
            command,
            args,
            env,
            working_dir,
            auto_start,
        };
        state
            .mcp_manager
            .add_server(server_config)
            .await
            .map_err(|e| e.to_string())?;

        let tool_count = state
            .mcp_manager
            .list_servers()
            .await
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.tool_count)
            .unwrap_or(0);
        state
            .event_bus
            .emit(omni_core::events::OmniEvent::McpServerConnected {
                server_name: name,
                tool_count,
            });
    }

    Ok(())
}

/// Remove an MCP server from config and disconnect if running.
#[tauri::command]
pub async fn mcp_remove_server(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    // Disconnect if running (ignore errors if not connected)
    let _ = state.mcp_manager.remove_server(&name).await;
    state
        .event_bus
        .emit(omni_core::events::OmniEvent::McpServerDisconnected {
            server_name: name.clone(),
        });

    // Remove from config
    {
        let mut config = state.config.write().await;
        config.mcp.servers.retain(|s| s.name != name);
        config
            .save(&state.paths.config_file())
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Update an existing MCP server's configuration.
#[tauri::command]
pub async fn mcp_update_server(
    state: State<'_, AppState>,
    name: String,
    command: Option<String>,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    working_dir: Option<String>,
    auto_start: Option<bool>,
) -> Result<(), String> {
    let mut config = state.config.write().await;
    let entry = config
        .mcp
        .servers
        .iter_mut()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("MCP server '{}' not found in config", name))?;

    if let Some(cmd) = command {
        entry.command = cmd;
    }
    if let Some(a) = args {
        entry.args = a;
    }
    if let Some(e) = env {
        entry.env = e;
    }
    if let Some(wd) = working_dir {
        entry.working_dir = if wd.is_empty() { None } else { Some(wd) };
    }
    if let Some(auto) = auto_start {
        entry.auto_start = auto;
    }

    config
        .save(&state.paths.config_file())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Start (connect to) an MCP server that is configured but not running.
#[tauri::command]
pub async fn mcp_start_server(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    // Find config entry
    let server_config = {
        let config = state.config.read().await;
        let entry = config
            .mcp
            .servers
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| format!("MCP server '{}' not found in config", name))?;
        let cfg: omni_llm::mcp::McpServerConfig = entry.clone().into();
        cfg
    };

    state
        .mcp_manager
        .add_server(server_config)
        .await
        .map_err(|e| e.to_string())?;

    let tool_count = state
        .mcp_manager
        .list_servers()
        .await
        .iter()
        .find(|s| s.name == name)
        .map(|s| s.tool_count)
        .unwrap_or(0);

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::McpServerConnected {
            server_name: name,
            tool_count,
        });

    Ok(())
}

/// Stop (disconnect from) a running MCP server.
#[tauri::command]
pub async fn mcp_stop_server(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    state
        .mcp_manager
        .remove_server(&name)
        .await
        .map_err(|e| e.to_string())?;

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::McpServerDisconnected {
            server_name: name,
        });

    Ok(())
}

/// Restart (disconnect + reconnect) a running MCP server.
#[tauri::command]
pub async fn mcp_restart_server(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    state
        .mcp_manager
        .restart_server(&name)
        .await
        .map_err(|e| e.to_string())?;

    let tool_count = state
        .mcp_manager
        .list_servers()
        .await
        .iter()
        .find(|s| s.name == name)
        .map(|s| s.tool_count)
        .unwrap_or(0);

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::McpServerConnected {
            server_name: name,
            tool_count,
        });

    Ok(())
}

/// Get the list of tools provided by a specific connected MCP server.
#[tauri::command]
pub async fn mcp_get_server_tools(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<McpToolDto>, String> {
    let schemas = state.mcp_manager.get_all_schemas().await;
    let prefix = format!("mcp_{}_", name);
    let mcp_prefix = format!("[MCP:{}] ", name);

    let tools: Vec<McpToolDto> = schemas
        .iter()
        .filter(|s| s.name.starts_with(&prefix))
        .map(|s| {
            let tool_name = s.name.strip_prefix(&prefix).unwrap_or(&s.name).to_string();
            let description = if s.description.starts_with(&mcp_prefix) {
                Some(s.description[mcp_prefix.len()..].to_string())
            } else {
                Some(s.description.clone())
            };
            McpToolDto {
                name: tool_name,
                description,
            }
        })
        .collect();

    Ok(tools)
}

// ─── Flowchart Commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn flowchart_list(state: State<'_, AppState>) -> Result<Vec<FlowchartDto>, String> {
    let summaries = state.flowchart_registry.list().await;
    Ok(summaries
        .into_iter()
        .map(|s| FlowchartDto {
            id: s.id,
            name: s.name,
            version: s.version,
            author: s.author,
            description: s.description,
            enabled: s.enabled,
            tool_count: s.tool_count,
            permission_count: s.permission_count,
        })
        .collect())
}

#[tauri::command]
pub async fn flowchart_get(
    state: State<'_, AppState>,
    flowchart_id: String,
) -> Result<FlowchartDefinitionDto, String> {
    let def = state
        .flowchart_registry
        .get(&flowchart_id)
        .await
        .ok_or_else(|| format!("Flowchart '{}' not found", flowchart_id))?;

    Ok(flowchart_def_to_dto(def))
}

#[tauri::command]
pub async fn flowchart_save(
    state: State<'_, AppState>,
    definition: FlowchartDefinitionDto,
) -> Result<(), String> {
    let flowchart_id = definition.id.clone();
    let def = dto_to_flowchart_def(definition);
    state
        .flowchart_registry
        .save(def)
        .await
        .map_err(|e| e.to_string())?;
    state
        .event_bus
        .emit(omni_core::events::OmniEvent::FlowchartSaved { flowchart_id });
    Ok(())
}

#[tauri::command]
pub async fn flowchart_delete(
    state: State<'_, AppState>,
    flowchart_id: String,
) -> Result<(), String> {
    state
        .flowchart_registry
        .delete(&flowchart_id)
        .await
        .map_err(|e| e.to_string())?;
    state.event_bus.emit(omni_core::events::OmniEvent::FlowchartDeleted {
        flowchart_id,
    });
    Ok(())
}

#[tauri::command]
pub async fn flowchart_toggle_enabled(
    state: State<'_, AppState>,
    flowchart_id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .flowchart_registry
        .set_enabled(&flowchart_id, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn flowchart_validate(
    state: State<'_, AppState>,
    definition: FlowchartDefinitionDto,
) -> Result<FlowchartValidationDto, String> {
    let def = dto_to_flowchart_def(definition);
    let issues = state.flowchart_registry.validate(&def);

    let errors: Vec<String> = issues
        .iter()
        .filter(|i| {
            i.level
                == omni_extensions::flowchart::registry::ValidationLevel::Error
        })
        .map(|i| i.message.clone())
        .collect();
    let warnings: Vec<String> = issues
        .iter()
        .filter(|i| {
            i.level
                == omni_extensions::flowchart::registry::ValidationLevel::Warning
        })
        .map(|i| i.message.clone())
        .collect();

    Ok(FlowchartValidationDto {
        valid: errors.is_empty(),
        warnings,
        errors,
    })
}

#[tauri::command]
pub async fn flowchart_test(
    state: State<'_, AppState>,
    flowchart_id: String,
    tool_name: String,
    test_params: serde_json::Value,
) -> Result<FlowchartTestResultDto, String> {
    // Guardian: scan test params before execution
    if state.guardian.is_enabled() {
        let params_str = serde_json::to_string(&test_params).unwrap_or_default();
        let scan = state.guardian.scan_input(&params_str);
        if scan.blocked {
            return Err(format!(
                "Guardian blocked test input: {}",
                scan.reason.unwrap_or_else(|| "potentially unsafe content".to_string())
            ));
        }
    }

    let result = state
        .flowchart_registry
        .test_execute(&flowchart_id, &tool_name, test_params)
        .await
        .map_err(|e| e.to_string())?;

    Ok(FlowchartTestResultDto {
        success: result.success,
        output: result.output,
        error: result.error,
        execution_time_ms: result.execution_time_ms,
        node_trace: result
            .node_trace
            .into_iter()
            .map(|t| crate::dto::FlowchartNodeTraceDto {
                node_id: t.node_id,
                node_type: t.node_type,
                label: t.label,
                duration_ms: t.duration_ms,
                error: t.error,
                input: t.input,
                output: t.output,
            })
            .collect(),
    })
}

// ── Flowchart DTO conversion helpers ────────────────────────────────────

fn flowchart_def_to_dto(
    def: omni_extensions::flowchart::types::FlowchartDefinition,
) -> FlowchartDefinitionDto {
    FlowchartDefinitionDto {
        id: def.id,
        name: def.name,
        version: def.version,
        author: def.author,
        description: def.description,
        enabled: def.enabled,
        tools: def
            .tools
            .into_iter()
            .map(|t| FlowchartToolDefDto {
                name: t.name,
                description: t.description,
                parameters: t.parameters,
                trigger_node_id: t.trigger_node_id,
            })
            .collect(),
        permissions: def
            .permissions
            .into_iter()
            .map(|p| FlowchartPermissionDto {
                capability: p.capability,
                reason: p.reason,
                required: p.required,
            })
            .collect(),
        config: def
            .config
            .into_iter()
            .map(|(k, v)| (k, serde_json::to_value(v).unwrap_or_default()))
            .collect(),
        nodes: def
            .nodes
            .into_iter()
            .map(|n| serde_json::to_value(n).unwrap_or_default())
            .collect(),
        edges: def
            .edges
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or_default())
            .collect(),
        viewport: def.viewport.map(|v| serde_json::to_value(v).unwrap_or_default()),
        created_at: def.created_at,
        updated_at: def.updated_at,
    }
}

fn dto_to_flowchart_def(
    dto: FlowchartDefinitionDto,
) -> omni_extensions::flowchart::types::FlowchartDefinition {
    use omni_extensions::flowchart::types::*;

    FlowchartDefinition {
        id: dto.id,
        name: dto.name,
        version: dto.version,
        author: dto.author,
        description: dto.description,
        created_at: dto.created_at,
        updated_at: dto.updated_at,
        enabled: dto.enabled,
        tools: dto
            .tools
            .into_iter()
            .map(|t| FlowchartToolDef {
                name: t.name,
                description: t.description,
                parameters: t.parameters,
                trigger_node_id: t.trigger_node_id,
            })
            .collect(),
        permissions: dto
            .permissions
            .into_iter()
            .map(|p| FlowchartPermission {
                capability: p.capability,
                reason: p.reason,
                required: p.required,
            })
            .collect(),
        config: dto
            .config
            .into_iter()
            .map(|(k, v)| {
                let field: FlowchartConfigField = serde_json::from_value(v).unwrap_or_else(|_| {
                    FlowchartConfigField {
                        field_type: "string".to_string(),
                        label: k.clone(),
                        help: None,
                        sensitive: false,
                        required: false,
                        default: None,
                    }
                });
                (k, field)
            })
            .collect(),
        nodes: dto
            .nodes
            .into_iter()
            .filter_map(|n| {
                serde_json::from_value(n.clone()).map_err(|e| {
                    tracing::warn!("Dropping malformed node in DTO conversion: {e} -- value: {n}");
                    e
                }).ok()
            })
            .collect(),
        edges: dto
            .edges
            .into_iter()
            .filter_map(|e| {
                serde_json::from_value(e.clone()).map_err(|err| {
                    tracing::warn!("Dropping malformed edge in DTO conversion: {err} -- value: {e}");
                    err
                }).ok()
            })
            .collect(),
        auto_triggers: vec![],
        viewport: dto
            .viewport
            .and_then(|v| {
                serde_json::from_value(v.clone()).map_err(|e| {
                    tracing::warn!("Dropping malformed viewport in DTO conversion: {e}");
                    e
                }).ok()
            }),
    }
}

// ─── Environment Variable Commands ──────────────────────────────────

/// Env var entry returned to the frontend (value is masked for security).
#[derive(serde::Serialize)]
pub struct EnvVarEntryDto {
    pub key: String,
    pub masked_value: String,
    pub is_set: bool,
}

/// List all user-configured environment variables.
/// Values are masked (first 4 chars + ***) for security.
#[tauri::command]
pub async fn env_vars_list(state: State<'_, AppState>) -> Result<Vec<EnvVarEntryDto>, String> {
    let config = state.config.read().await;
    let entries: Vec<EnvVarEntryDto> = config
        .env_vars
        .iter()
        .map(|(key, value)| {
            let masked = if value.len() <= 4 {
                "****".to_string()
            } else {
                format!("{}***", &value[..4])
            };
            EnvVarEntryDto {
                key: key.clone(),
                masked_value: masked,
                is_set: true,
            }
        })
        .collect();
    Ok(entries)
}

/// Set an environment variable. Persists to config.toml and injects into
/// the current process immediately so tools pick it up without restart.
#[tauri::command]
pub async fn env_vars_set(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    // Validate key: must be non-empty, alphanumeric + underscores
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Key must be non-empty and contain only A-Z, 0-9, and underscores".to_string());
    }

    // Inject into current process immediately
    std::env::set_var(&key, &value);

    // Persist to config
    let mut config = state.config.write().await;
    config.env_vars.insert(key.clone(), value);
    config
        .save(&state.paths.config_file())
        .map_err(|e| e.to_string())?;

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ConfigChanged);

    tracing::info!(key = %key, "Environment variable set via UI");
    Ok(())
}

/// Delete an environment variable. Removes from config.toml and un-sets
/// it from the current process.
#[tauri::command]
pub async fn env_vars_delete(
    state: State<'_, AppState>,
    key: String,
) -> Result<(), String> {
    // Remove from current process
    std::env::remove_var(&key);

    // Remove from config
    let mut config = state.config.write().await;
    config.env_vars.remove(&key);
    config
        .save(&state.paths.config_file())
        .map_err(|e| e.to_string())?;

    state
        .event_bus
        .emit(omni_core::events::OmniEvent::ConfigChanged);

    tracing::info!(key = %key, "Environment variable deleted via UI");
    Ok(())
}
