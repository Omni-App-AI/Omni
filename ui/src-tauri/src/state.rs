use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use omni_core::config::OmniConfig;
use omni_core::database::{self, Database};
use omni_core::events::EventBus;
use omni_core::paths::OmniPaths;
use omni_channels::bindings::BindingRegistry;
use omni_channels::bluebubbles::BlueBubblesChannelFactory;
use omni_channels::discord::DiscordChannelFactory;
use omni_channels::feishu::FeishuChannelFactory;
use omni_channels::google_chat::GoogleChatChannelFactory;
use omni_channels::imessage::IMessageChannelFactory;
use omni_channels::irc_channel::IrcChannelFactory;
use omni_channels::line::LineChannelFactory;
use omni_channels::manager::ChannelManager;
use omni_channels::matrix::MatrixChannelFactory;
use omni_channels::mattermost::MattermostChannelFactory;
use omni_channels::nextcloud_talk::NextcloudTalkChannelFactory;
use omni_channels::nostr_channel::NostrChannelFactory;
use omni_channels::signal::SignalChannelFactory;
use omni_channels::slack::SlackChannelFactory;
use omni_channels::synology_chat::SynologyChatChannelFactory;
use omni_channels::teams::TeamsChannelFactory;
use omni_channels::telegram::TelegramChannelFactory;
use omni_channels::twitter::TwitterChannelFactory;
use omni_channels::twitch::TwitchChannelFactory;
use omni_channels::urbit::UrbitChannelFactory;
use omni_channels::webchat::WebChatChannelFactory;
use omni_channels::whatsapp_web::WhatsAppWebChannelFactory;
use omni_channels::zalo::ZaloChannelFactory;
use crate::marketplace::MarketplaceClient;
use omni_extensions::flowchart::{FlowchartEngine, FlowchartRegistry};
use omni_extensions::host::ExtensionHost;
use omni_guardian::Guardian;
use omni_llm::bridge::LLMBridge;
use omni_llm::credentials::CredentialVault;
use omni_llm::guardian_bridge::ExtensionToolRegistry;
use omni_llm::mcp::{McpManager, McpServerConfig};
use omni_llm::tools::NativeToolRegistry;
use omni_permissions::policy::{DefaultPolicy, PolicyEngine};
use tokio::sync::RwLock;

/// Wrapper to allow `!Send + !Sync` types (wasmtime-based ExtensionHost)
/// to be held in Tauri managed state. Safety: access is serialized via Mutex.
pub(crate) struct SendSyncWrapper<T>(pub(crate) T);
// SAFETY: Access is serialized through the enclosing Mutex, preventing
// concurrent access. ExtensionHost is only !Send/!Sync because of
// wasmtime::Store internals; it is safe to move between threads when
// accessed exclusively.
unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}

/// Shared application state managed by Tauri.
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub event_bus: EventBus,
    pub config: Arc<RwLock<OmniConfig>>,
    pub paths: OmniPaths,
    pub policy_engine: Arc<PolicyEngine>,
    extension_host: Arc<Mutex<SendSyncWrapper<ExtensionHost>>>,
    pub guardian: Arc<Guardian>,
    pub llm_bridge: Arc<LLMBridge>,
    pub tool_registry: Arc<ExtensionToolRegistry>,
    pub channel_manager: Arc<ChannelManager>,
    pub binding_registry: Arc<BindingRegistry>,
    pub marketplace: Arc<MarketplaceClient>,
    pub mcp_manager: Arc<McpManager>,
    pub flowchart_registry: Arc<FlowchartRegistry>,
    /// Pending permission prompts awaiting user response.
    pub pending_prompts: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<PromptResponse>>>>,
}

impl AppState {
    /// Access the extension host through a locked guard.
    /// Returns the MutexGuard -- caller must drop it before .awaiting.
    pub fn with_extension_host<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ExtensionHost) -> R,
    {
        let guard = self.extension_host.lock().unwrap_or_else(|e| e.into_inner());
        f(&guard.0)
    }

    /// Get a clone of the extension host Arc for spawning onto blocking tasks.
    pub fn extension_host_clone(&self) -> Arc<Mutex<SendSyncWrapper<ExtensionHost>>> {
        self.extension_host.clone()
    }
}

/// Response to a permission prompt from the UI.
#[derive(Debug, Clone)]
pub struct PromptResponse {
    pub decision: String,
    pub duration: String,
}

impl AppState {
    pub fn initialize(app: &tauri::App) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Resolve paths
        let paths = OmniPaths::resolve()?;
        paths.ensure_dirs_exist()?;

        // 2. Load config
        let config = OmniConfig::load(&paths.config_file())?;

        // 2b. Inject user-defined environment variables from config
        for (key, value) in &config.env_vars {
            std::env::set_var(key, value);
        }
        if !config.env_vars.is_empty() {
            tracing::info!("{} environment variable(s) injected from config", config.env_vars.len());
        }

        // 3. Init logging
        omni_core::logging::init_logging(&config.general.log_level, Some(&paths.log_file()))?;

        // 4. Open database (with recovery if key mismatch from CLI-created DB)
        let db_key = database::get_or_create_encryption_key()?;
        let db_path = paths.database_file();
        let db = match Database::open(&db_path, &db_key) {
            Ok(db) => db,
            Err(e) => {
                tracing::warn!(
                    "Failed to open database ({}). Removing and recreating.",
                    e
                );
                if db_path.exists() {
                    std::fs::remove_file(&db_path)?;
                }
                Database::open(&db_path, &db_key)?
            }
        };
        let db = Arc::new(Mutex::new(db));

        // 5. Event bus
        let event_bus = EventBus::new(1024);

        // 6. Start event bridge to Tauri
        let app_handle = app.handle().clone();
        let event_rx_bus = event_bus.clone();
        tauri::async_runtime::spawn(async move {
            crate::events::run_event_bridge(app_handle, event_rx_bus).await;
        });

        // 7. Policy engine
        let policy_engine = Arc::new(PolicyEngine::new(db.clone(), DefaultPolicy::Deny));

        // 8. Extension host (wrapped for Send + Sync)
        let extensions_dir = paths.extensions_dir();
        let mut extension_host = ExtensionHost::new(
            policy_engine.clone(),
            event_bus.clone(),
            db.clone(),
            extensions_dir,
        )
        .map_err(|e| format!("Failed to create extension host: {}", e))?;

        // 8b. Set up extension callbacks (LLM + channels) BEFORE activation
        //     so that activated sandboxes receive them.
        //     NOTE: LLM bridge and channel manager are created later, but we need
        //     them now. Create them early and re-use the Arcs.
        let vault = Arc::new(CredentialVault::new());
        let llm_bridge = Arc::new(LLMBridge::new(vault));

        // 8b-pre. Register LLM providers from config so the bridge can serve requests.
        {
            let rt = tauri::async_runtime::handle();
            for (name, provider_cfg) in &config.providers {
                if !provider_cfg.enabled {
                    continue;
                }
                rt.block_on(crate::commands::register_provider_from_config(
                    &llm_bridge,
                    name,
                    provider_cfg,
                ));
            }
        }

        // Parse webhook bind address from config (default: 127.0.0.1 for security)
        let webhook_bind_addr: [u8; 4] = config
            .channels
            .webhook_bind_address
            .parse::<std::net::Ipv4Addr>()
            .map(|ip| ip.octets())
            .unwrap_or_else(|_| {
                tracing::warn!(
                    address = config.channels.webhook_bind_address.as_str(),
                    "Invalid webhook_bind_address in config, falling back to 127.0.0.1"
                );
                [127, 0, 0, 1]
            });

        let channel_manager = Arc::new(ChannelManager::new_with_webhook_config(
            event_bus.clone(),
            config.channels.webhook_port,
            webhook_bind_addr,
        ));

        // 12b will be initialized later, but we need the Arc reference now
        let mcp_manager = Arc::new(McpManager::new());

        {
            let rt = tauri::async_runtime::handle();
            let tokio_handle = rt.block_on(async { tokio::runtime::Handle::current() });
            let llm_cb = Arc::new(crate::callbacks::AppLlmCallback::new(
                llm_bridge.clone(),
                tokio_handle.clone(),
            ));
            let ch_cb = Arc::new(crate::callbacks::AppChannelCallback::new(
                channel_manager.clone(),
                tokio_handle.clone(),
            ));
            let mcp_cb = Arc::new(crate::callbacks::AppMcpCallback::new(
                mcp_manager.clone(),
                tokio_handle,
            ));
            extension_host.set_llm_callback(llm_cb);
            extension_host.set_channel_callback(ch_cb);
            extension_host.set_mcp_callback(mcp_cb);
        }

        // 8c. Load instance metas from DB, discover extensions, and auto-activate instances.
        // Also auto-register any MCP servers declared in extension manifests.
        let ext_mcp_servers_to_register: Vec<(String, Vec<omni_extensions::manifest::McpServerDeclaration>)>;
        {
            let rt = tauri::async_runtime::handle();

            // Load instance metas from database
            let instance_rows = {
                let db_guard = db.lock().unwrap();
                db_guard.list_extension_instances().unwrap_or_default()
            };
            let metas: Vec<omni_extensions::host::ExtensionInstanceMeta> = instance_rows
                .into_iter()
                .map(|row| omni_extensions::host::ExtensionInstanceMeta {
                    instance_name: row.instance_name,
                    extension_id: row.extension_id,
                    display_name: row.display_name,
                    enabled: row.enabled,
                    created_at: chrono::NaiveDateTime::parse_from_str(
                        &row.created_at,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .map(|dt| dt.and_utc())
                    .unwrap_or_else(|_| chrono::Utc::now()),
                })
                .collect();
            if !metas.is_empty() {
                tracing::info!("{} extension instance(s) loaded from database", metas.len());
            }
            extension_host.load_instance_metas_sync(metas);

            // Discover and register extensions (also creates ::default instances if needed)
            let mut mcp_to_reg = Vec::new();
            match rt.block_on(extension_host.discover_and_register()) {
                Ok(ids) => {
                    // Activate all enabled instances for discovered extensions
                    for ext_id in &ids {
                        let ext_instances = rt.block_on(extension_host.list_instances(Some(ext_id)));
                        for (iid, meta) in &ext_instances {
                            if !meta.enabled {
                                continue;
                            }
                            if let Err(e) = rt.block_on(extension_host.activate(iid)) {
                                tracing::warn!(instance = %iid, "Failed to auto-activate instance: {}", e);
                                continue;
                            }
                        }
                        let servers = rt.block_on(extension_host.get_mcp_servers(ext_id));
                        if !servers.is_empty() {
                            mcp_to_reg.push((ext_id.clone(), servers));
                        }
                    }
                    if !ids.is_empty() {
                        tracing::info!("{} extension(s) discovered and registered on startup", ids.len());
                    }
                }
                Err(e) => {
                    tracing::warn!("Extension discovery failed: {}", e);
                }
            }
            ext_mcp_servers_to_register = mcp_to_reg;
        }

        let extension_host = Arc::new(Mutex::new(SendSyncWrapper(extension_host)));

        // 9. (LLM bridge created earlier at step 8b for extension callbacks)

        // 10. Tool registry + Guardian
        let tool_registry = Arc::new(ExtensionToolRegistry::new());
        let guardian = Arc::new(
            Guardian::new(
                &config.guardian,
                event_bus.clone(),
                Box::new(tool_registry.as_ref().clone()),
                None,
            )
            .map_err(|e| format!("Failed to initialize Guardian: {}", e))?,
        );

        // 11. Channel manager factory registration (manager created earlier at step 8b)
        {
            let cm = channel_manager.clone();
            let db_for_channels = db.clone();
            let config_for_channels = config.clone();
            tauri::async_runtime::spawn(async move {
                // Register all 22 channel type factories
                cm.register_factory(Arc::new(DiscordChannelFactory)).await;
                cm.register_factory(Arc::new(TelegramChannelFactory)).await;
                cm.register_factory(Arc::new(WhatsAppWebChannelFactory)).await;
                cm.register_factory(Arc::new(SynologyChatChannelFactory)).await;
                cm.register_factory(Arc::new(ZaloChannelFactory)).await;
                cm.register_factory(Arc::new(BlueBubblesChannelFactory)).await;
                cm.register_factory(Arc::new(SlackChannelFactory)).await;
                cm.register_factory(Arc::new(MattermostChannelFactory)).await;
                cm.register_factory(Arc::new(LineChannelFactory)).await;
                cm.register_factory(Arc::new(TeamsChannelFactory)).await;
                cm.register_factory(Arc::new(GoogleChatChannelFactory)).await;
                cm.register_factory(Arc::new(FeishuChannelFactory)).await;
                cm.register_factory(Arc::new(IrcChannelFactory)).await;
                cm.register_factory(Arc::new(TwitchChannelFactory)).await;
                cm.register_factory(Arc::new(MatrixChannelFactory)).await;
                cm.register_factory(Arc::new(NostrChannelFactory)).await;
                cm.register_factory(Arc::new(NextcloudTalkChannelFactory)).await;
                cm.register_factory(Arc::new(IMessageChannelFactory)).await;
                cm.register_factory(Arc::new(SignalChannelFactory)).await;
                cm.register_factory(Arc::new(UrbitChannelFactory)).await;
                cm.register_factory(Arc::new(WebChatChannelFactory)).await;
                cm.register_factory(Arc::new(TwitterChannelFactory)).await;
                tracing::info!("All 22 channel type factories registered");

                // Load saved channel instances from database
                let instances = {
                    let db_guard = db_for_channels.lock().unwrap();
                    db_guard.list_channel_instances().unwrap_or_default()
                };
                for inst in &instances {
                    if let Err(e) = cm.create_instance(&inst.channel_type, &inst.instance_id).await {
                        tracing::warn!(
                            "Failed to restore channel instance {}:{} -- {}",
                            inst.channel_type, inst.instance_id, e
                        );
                    }
                }

                // Also create instances for any types configured in config file
                for (key, _inst_cfg) in &config_for_channels.channels.instances {
                    let cid = omni_channels::ChannelInstanceId::parse(key);
                    if cm.get_channel(&cid.key()).await.is_err() {
                        if let Err(e) = cm.create_instance(&cid.channel_type, &cid.instance_id).await {
                            tracing::warn!(
                                "Failed to create configured instance {} -- {}",
                                key, e
                            );
                        }
                    }
                }

                let count = cm.list_channels().await.len();
                tracing::info!("{} channel instances active", count);
            });
        }

        // 12. Marketplace client
        let marketplace = Arc::new(MarketplaceClient::new(&config.marketplace.api_url));

        // 12b. MCP server manager -- auto-start config servers + extension MCP servers
        // (mcp_manager Arc was created earlier at step 8b for extension callback injection)
        {
            let mcp = mcp_manager.clone();
            let mcp_servers = config.mcp.servers.clone();
            let mcp_event_bus = event_bus.clone();
            let ext_mcp = ext_mcp_servers_to_register;
            tauri::async_runtime::spawn(async move {
                // Register config-declared MCP servers
                for entry in mcp_servers {
                    if !entry.auto_start {
                        continue;
                    }
                    let name = entry.name.clone();
                    let server_config: McpServerConfig = entry.into();
                    match mcp.add_server(server_config).await {
                        Ok(()) => {
                            let tool_count = mcp
                                .list_servers()
                                .await
                                .iter()
                                .find(|s| s.name == name)
                                .map(|s| s.tool_count)
                                .unwrap_or(0);
                            mcp_event_bus.emit(omni_core::events::OmniEvent::McpServerConnected {
                                server_name: name,
                                tool_count,
                            });
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to auto-start MCP server '{}': {}",
                                name,
                                e
                            );
                        }
                    }
                }

                // Register extension-declared MCP servers
                for (ext_id, servers) in ext_mcp {
                    for decl in servers {
                        let server_name = format!("{}:{}", ext_id, decl.name);
                        let config = McpServerConfig {
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
                                mcp_event_bus.emit(omni_core::events::OmniEvent::McpServerConnected {
                                    server_name: server_name.clone(),
                                    tool_count,
                                });
                                tracing::info!(
                                    extension = %ext_id,
                                    server = %server_name,
                                    "Auto-registered extension MCP server on startup"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    extension = %ext_id,
                                    server = %decl.name,
                                    "Failed to auto-register extension MCP server: {}",
                                    e
                                );
                            }
                        }
                    }
                }
            });
        }

        // 13. Binding registry -- load from DB + config
        let binding_registry = Arc::new(BindingRegistry::new());
        {
            let db_guard = db.lock().unwrap();
            let db_bindings = db_guard.list_bindings().unwrap_or_default();
            let mut bindings: Vec<omni_channels::bindings::ChannelBinding> = db_bindings
                .into_iter()
                .map(|r| omni_channels::bindings::ChannelBinding {
                    id: r.id,
                    channel_instance: r.channel_instance,
                    extension_id: r.extension_id,
                    peer_filter: r.peer_filter,
                    group_filter: r.group_filter,
                    priority: r.priority,
                    enabled: r.enabled,
                })
                .collect();

            // Also load bindings from config (use deterministic IDs to avoid duplicates)
            for cfg_binding in &config.channels.bindings {
                let id = format!(
                    "cfg:{}:{}",
                    cfg_binding.channel_instance, cfg_binding.extension_id
                );
                if !bindings.iter().any(|b| b.id == id) {
                    bindings.push(omni_channels::bindings::ChannelBinding {
                        id,
                        channel_instance: cfg_binding.channel_instance.clone(),
                        extension_id: cfg_binding.extension_id.clone(),
                        peer_filter: cfg_binding.peer_filter.clone(),
                        group_filter: cfg_binding.group_filter.clone(),
                        priority: cfg_binding.priority,
                        enabled: true,
                    });
                }
            }

            binding_registry.load(bindings);
            tracing::info!(
                "{} channel bindings loaded",
                binding_registry.list().len()
            );
        }

        // 14. Flowchart registry for visual extensions
        let flowcharts_dir = paths.extensions_dir().join("flowcharts");
        std::fs::create_dir_all(&flowcharts_dir)?;

        let mut flowchart_engine = FlowchartEngine::new(
            policy_engine.clone(),
            db.clone(),
        );
        {
            let rt = tauri::async_runtime::handle();
            let tokio_handle = rt.block_on(async { tokio::runtime::Handle::current() });
            let llm_cb = Arc::new(crate::callbacks::AppLlmCallback::new(
                llm_bridge.clone(),
                tokio_handle.clone(),
            ));
            let ch_cb = Arc::new(crate::callbacks::AppChannelCallback::new(
                channel_manager.clone(),
                tokio_handle.clone(),
            ));
            let native_registry = Arc::new(NativeToolRegistry::new_with_db(
                policy_engine.clone(),
                db.clone(),
            ));
            let nt_cb = Arc::new(crate::callbacks::AppNativeToolCallback::new(
                native_registry.clone(),
                tokio_handle.clone(),
            ));
            flowchart_engine.set_llm_callback(llm_cb);
            flowchart_engine.set_channel_callback(ch_cb);
            flowchart_engine.set_native_tool_callback(nt_cb);
            // Agent callback for AgentRequest nodes -- lightweight agent loop
            let agent_cb = Arc::new(crate::callbacks::AppAgentCallback::new(
                llm_bridge.clone(),
                native_registry,
                mcp_manager.clone(),
                guardian.clone(),
                tokio_handle,
            ));
            flowchart_engine.set_agent_callback(agent_cb);
            // Guardian scanner for flowchart engine -- scans at every external boundary
            let guardian_cb = Arc::new(crate::callbacks::AppGuardianCallback::new(guardian.clone()));
            flowchart_engine.set_guardian_callback(guardian_cb);
            // EventBus for audit events (node execution, Guardian blocks, permission denials)
            flowchart_engine.set_event_bus(event_bus.clone());
        }
        let flowchart_engine = Arc::new(flowchart_engine);
        let flowchart_registry = Arc::new(FlowchartRegistry::new(
            flowcharts_dir,
            flowchart_engine.clone(),
        ));
        // Set sub-flow callback now that registry exists (OnceLock on engine allows this)
        {
            let rt = tauri::async_runtime::handle();
            let tokio_handle = rt.block_on(async { tokio::runtime::Handle::current() });
            let fc_cb = Arc::new(crate::callbacks::AppFlowchartCallback::new(
                flowchart_registry.clone(),
                tokio_handle,
            ));
            flowchart_engine.set_flowchart_callback(fc_cb);
        }
        {
            let rt = tauri::async_runtime::handle();
            match rt.block_on(flowchart_registry.discover()) {
                Ok(ids) => {
                    if !ids.is_empty() {
                        tracing::info!("{} flowchart(s) discovered on startup", ids.len());
                    }
                }
                Err(e) => tracing::warn!("Flowchart discovery failed: {}", e),
            }
        }

        // 15. Auto-trigger service for event/schedule-driven flowcharts
        {
            let guardian_cb_for_triggers = Arc::new(
                crate::callbacks::AppGuardianCallback::new(guardian.clone()),
            );
            let trigger_service = omni_extensions::flowchart::AutoTriggerService::new(
                flowchart_registry.clone(),
            ).with_guardian(guardian_cb_for_triggers);
            let event_bus_for_triggers = event_bus.clone();
            tauri::async_runtime::spawn(async move {
                trigger_service.start(&event_bus_for_triggers).await;
            });
        }

        tracing::info!("Omni UI initialized successfully");

        Ok(Self {
            db,
            event_bus,
            config: Arc::new(RwLock::new(config)),
            paths,
            policy_engine,
            extension_host,
            guardian,
            llm_bridge,
            tool_registry,
            channel_manager,
            binding_registry,
            marketplace,
            mcp_manager,
            flowchart_registry,
            pending_prompts: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}
