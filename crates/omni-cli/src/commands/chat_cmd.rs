use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};

use omni_core::config::OmniConfig;
use omni_core::database::Database;
use omni_core::events::EventBus;
use omni_extensions::host::ExtensionHost;
use omni_guardian::Guardian;
use omni_llm::agent::AgentLoop;
use omni_llm::bridge::LLMBridge;
use omni_llm::credentials::CredentialVault;
use omni_llm::guardian_bridge::ExtensionToolRegistry;
use omni_llm::hooks::HookRegistry;
use omni_llm::tools::NativeToolRegistry;
use omni_llm::providers::anthropic::AnthropicProvider;
use omni_llm::providers::ollama::OllamaProvider;
use omni_llm::providers::openai::OpenAIProvider;
use omni_llm::types::ChatMessage;
use omni_permissions::policy::{DefaultPolicy, PolicyEngine};

pub async fn run(
    session_id: Option<String>,
    config: &OmniConfig,
    db: Arc<Mutex<Database>>,
    event_bus: EventBus,
    extensions_dir: std::path::PathBuf,
) -> anyhow::Result<()> {
    // 1. Set up credential vault
    let vault = Arc::new(CredentialVault::new());

    // 2. Create LLM bridge and register configured providers
    let bridge = Arc::new(LLMBridge::new(vault.clone()));

    for (name, provider_config) in &config.providers {
        if !provider_config.enabled {
            continue;
        }

        match provider_config.provider_type.as_str() {
            "openai" => {
                let transport = match provider_config.transport.as_deref() {
                    Some("auto") => omni_llm::providers::openai_ws::OpenAITransport::Auto,
                    Some("ws") | Some("websocket") => omni_llm::providers::openai_ws::OpenAITransport::WebSocket,
                    _ => omni_llm::providers::openai_ws::OpenAITransport::Sse,
                };
                let provider = Arc::new(OpenAIProvider::with_transport(
                    provider_config.endpoint.as_deref(),
                    transport,
                ));
                bridge.register_provider_as(name, provider).await;
            }
            "anthropic" => {
                let provider = Arc::new(AnthropicProvider::new(
                    provider_config.endpoint.as_deref(),
                ));
                bridge.register_provider_as(name, provider).await;
            }
            "ollama" => {
                let provider = Arc::new(OllamaProvider::new(
                    provider_config.endpoint.as_deref(),
                ));
                bridge.register_provider_as(name, provider).await;
            }
            "google" => {
                let provider = Arc::new(omni_llm::providers::google::GoogleProvider::new(
                    provider_config.endpoint.as_deref(),
                ));
                bridge.register_provider_as(name, provider).await;
            }
            "bedrock" => {
                let provider = Arc::new(omni_llm::providers::bedrock::BedrockProvider::new(
                    None,
                ));
                bridge.register_provider_as(name, provider).await;
            }
            "custom" => {
                let provider = Arc::new(omni_llm::providers::custom::CustomProvider::new(
                    provider_config.endpoint.as_deref().unwrap_or("http://localhost:8080/v1"),
                    Some(name.as_str()),
                ));
                bridge.register_provider_as(name, provider).await;
            }
            other => {
                tracing::warn!(
                    provider = name,
                    provider_type = other,
                    "Unknown provider type, skipping"
                );
            }
        }
    }

    // 3. Determine which provider/model to use
    let (provider_id, model) = select_provider_and_model(config)?;

    // 4. Create session
    let session_id = match session_id {
        Some(id) => {
            // Verify session exists
            let db_clone = db.clone();
            let id_clone = id.clone();
            let session = tokio::task::spawn_blocking(move || {
                let db = db_clone.lock().unwrap();
                db.get_session(&id_clone)
            })
            .await??;

            if session.is_none() {
                anyhow::bail!("Session '{}' not found", id);
            }
            println!("Continuing session: {}", id);
            id
        }
        None => {
            let db_clone = db.clone();
            let id = tokio::task::spawn_blocking(move || {
                let db = db_clone.lock().unwrap();
                db.create_session(None)
            })
            .await??;
            println!("New session: {}", id);
            id
        }
    };

    // 5. Load existing messages for the session
    let db_clone = db.clone();
    let sid_clone = session_id.clone();
    let existing_messages = tokio::task::spawn_blocking(move || {
        let db = db_clone.lock().unwrap();
        db.get_messages_for_session(&sid_clone)
    })
    .await??;

    let mut messages: Vec<ChatMessage> = Vec::new();

    // Add system prompt if configured
    if let Some(ref system_prompt) = config.agent.system_prompt {
        messages.push(ChatMessage::system(system_prompt));
    }

    // Replay existing messages
    for msg in &existing_messages {
        let chat_msg = match msg.role.as_str() {
            "user" => ChatMessage::user(&msg.content),
            "assistant" => ChatMessage::assistant(&msg.content),
            "system" => ChatMessage::system(&msg.content),
            _ => continue,
        };
        messages.push(chat_msg);
    }

    // 6. Set up extension host for tool calls
    let policy = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));
    // ExtensionHost uses tokio::sync::RwLock internally; Arc is needed for AgentLoop.
    // WasmSandbox (wasmtime) is !Sync but we only access it from the async runtime.
    #[allow(clippy::arc_with_non_send_sync)]
    let ext_host = Arc::new(
        ExtensionHost::new(policy.clone(), event_bus.clone(), db.clone(), extensions_dir)?,
    );

    // 7. Create Guardian with cached tool registry
    let tool_registry = Arc::new(ExtensionToolRegistry::new());
    let guardian = Arc::new(
        Guardian::new(
            &config.guardian,
            event_bus.clone(),
            Box::new(tool_registry.as_ref().clone()),
            None,
        )
        .map_err(|e| anyhow::anyhow!("Failed to initialize Guardian: {}", e))?,
    );

    if guardian.is_enabled() {
        println!("Guardian anti-injection: enabled (sensitivity: {})", config.guardian.sensitivity);
    }

    // 8. Create agent loop
    let provider_config = config.providers.get(&provider_id);
    let max_tokens = provider_config.and_then(|c| c.max_tokens);
    let temperature = provider_config.and_then(|c| c.temperature);

    let native_tools = Arc::new(NativeToolRegistry::new_with_db(policy.clone(), db.clone()));
    let hook_registry = Arc::new(HookRegistry::new());

    let mut agent = AgentLoop::new(
        bridge,
        ext_host,
        guardian,
        tool_registry,
        native_tools,
        hook_registry,
        event_bus,
        &provider_id,
        &model,
        max_tokens,
        temperature,
    );

    // Wire thinking config from agent settings
    if let Some(ref thinking_mode) = config.agent.thinking_mode {
        let mode = match thinking_mode.as_str() {
            "adaptive" => Some(omni_llm::types::ThinkingMode::Adaptive),
            "enabled" => {
                let budget = config.agent.thinking_budget.unwrap_or(10000);
                Some(omni_llm::types::ThinkingMode::Enabled { budget_tokens: budget })
            }
            _ => None,
        };
        if let Some(mode) = mode {
            let effort = config.agent.thinking_effort.as_deref().map(|e| match e {
                "low" => omni_llm::types::ThinkingEffort::Low,
                "medium" => omni_llm::types::ThinkingEffort::Medium,
                "max" => omni_llm::types::ThinkingEffort::Max,
                _ => omni_llm::types::ThinkingEffort::High,
            });
            agent = agent.with_thinking(mode, effort);
        }
    }

    // 8. Interactive chat loop
    println!("Omni Chat (provider: {}, model: {})", provider_id, model);
    println!("Type 'exit' or 'quit' to end the session.");
    println!("---");

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        print!("\nYou: ");
        io::stdout().flush()?;

        let mut input = String::new();
        if reader.read_line(&mut input)? == 0 {
            break; // EOF
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "exit" || input == "quit" {
            println!("Goodbye!");
            break;
        }

        // Save user message to DB
        let db_clone = db.clone();
        let sid_clone = session_id.clone();
        let input_clone = input.to_string();
        tokio::task::spawn_blocking(move || {
            let db = db_clone.lock().unwrap();
            db.insert_message(&omni_core::database::NewMessage {
                session_id: sid_clone,
                role: "user".to_string(),
                content: input_clone,
                tool_call_id: None,
                tool_calls: None,
                token_count: None,
            })
        })
        .await??;

        // Run agent loop
        print!("\nAssistant: ");
        io::stdout().flush()?;

        match agent
            .run(
                &mut messages,
                input,
                config.agent.max_iterations,
            )
            .await
        {
            Ok(result) => {
                println!("{}", result.text);

                // Save assistant message to DB
                let db_clone = db.clone();
                let sid_clone = session_id.clone();
                tokio::task::spawn_blocking(move || {
                    let db = db_clone.lock().unwrap();
                    db.insert_message(&omni_core::database::NewMessage {
                        session_id: sid_clone,
                        role: "assistant".to_string(),
                        content: result.text,
                        tool_call_id: None,
                        tool_calls: None,
                        token_count: None,
                    })
                })
                .await??;
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
            }
        }
    }

    Ok(())
}

fn select_provider_and_model(
    config: &OmniConfig,
) -> anyhow::Result<(String, String)> {
    // Find the first enabled provider with a default model
    for (name, provider_config) in &config.providers {
        if !provider_config.enabled {
            continue;
        }

        let model = provider_config
            .default_model
            .clone()
            .unwrap_or_else(|| default_model_for_type(&provider_config.provider_type));

        return Ok((name.clone(), model));
    }

    // No providers configured -- default to ollama
    Ok(("ollama".to_string(), "llama3.3".to_string()))
}

fn default_model_for_type(provider_type: &str) -> String {
    match provider_type {
        "openai" => "gpt-5.2".to_string(),
        "anthropic" => "claude-opus-4-6".to_string(),
        "ollama" => "llama4".to_string(),
        "google" => "gemini-3.1-pro-preview".to_string(),
        _ => "default".to_string(),
    }
}
