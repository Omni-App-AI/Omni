use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{RwLock, Semaphore};

use omni_core::database::Database;
use omni_core::events::{EventBus, OmniEvent};
use omni_permissions::policy::PolicyEngine;

use crate::error::{ExtensionError, Result};
use crate::manifest::ExtensionManifest;
use crate::sandbox::{
    ChannelCallback, ExtensionInstance, LlmCallback, McpCallback, ResourceLimits, SandboxConfig,
    WasmSandbox,
};

/// Instance ID separator -- two colons distinguish from single-colon channel IDs.
const INSTANCE_SEPARATOR: &str = "::";

/// Source from which to install an extension.
pub enum ExtensionSource {
    Path(PathBuf),
}

/// Tracks an installed extension (the base package).
pub struct InstalledExtension {
    pub manifest: ExtensionManifest,
    pub install_path: PathBuf,
    pub installed_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub enabled: bool,
}

/// Metadata for a single instance of an extension.
#[derive(Debug, Clone)]
pub struct ExtensionInstanceMeta {
    pub instance_name: String,
    pub extension_id: String,
    pub display_name: Option<String>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<Utc>,
}

/// Manages extension discovery, installation, activation, and lifecycle.
pub struct ExtensionHost {
    registry: RwLock<HashMap<String, InstalledExtension>>,
    /// Instance metadata, keyed by full instance_id (e.g., "com.example.ext::my-bot").
    instance_metas: RwLock<HashMap<String, ExtensionInstanceMeta>>,
    /// Active WASM sandboxes, keyed by instance_id.
    instances: RwLock<HashMap<String, ExtensionInstance>>,
    sandbox: WasmSandbox,
    policy_engine: Arc<PolicyEngine>,
    event_bus: EventBus,
    db: Arc<Mutex<Database>>,
    extensions_dir: PathBuf,
    llm_callback: Option<Arc<dyn LlmCallback>>,
    channel_callback: Option<Arc<dyn ChannelCallback>>,
    mcp_callback: Option<Arc<dyn McpCallback>>,
    /// Per-instance concurrency semaphores, keyed by instance_id.
    concurrency_semaphores: RwLock<HashMap<String, Arc<Semaphore>>>,
}

/// Parse a full instance_id into (extension_id, instance_name).
/// If no `::` separator is found, assumes `::default`.
pub fn parse_instance_id(instance_id: &str) -> (&str, &str) {
    match instance_id.split_once(INSTANCE_SEPARATOR) {
        Some((ext, name)) => (ext, name),
        None => (instance_id, "default"),
    }
}

/// Build a full instance_id from extension_id and instance_name.
pub fn format_instance_id(extension_id: &str, instance_name: &str) -> String {
    format!("{}{}{}", extension_id, INSTANCE_SEPARATOR, instance_name)
}

/// Resolve an ID that might be either a bare extension_id or a full instance_id.
/// Returns the full instance_id (appending `::default` if needed).
pub fn resolve_instance_id(id: &str) -> String {
    if id.contains(INSTANCE_SEPARATOR) {
        id.to_string()
    } else {
        format_instance_id(id, "default")
    }
}

impl ExtensionHost {
    pub fn new(
        policy_engine: Arc<PolicyEngine>,
        event_bus: EventBus,
        db: Arc<Mutex<Database>>,
        extensions_dir: PathBuf,
    ) -> Result<Self> {
        let sandbox = WasmSandbox::new(&SandboxConfig::default())?;

        Ok(Self {
            registry: RwLock::new(HashMap::new()),
            instance_metas: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            sandbox,
            policy_engine,
            event_bus,
            db,
            extensions_dir,
            llm_callback: None,
            channel_callback: None,
            mcp_callback: None,
            concurrency_semaphores: RwLock::new(HashMap::new()),
        })
    }

    /// Set the LLM inference callback. Extensions calling `llm_request` will use this.
    pub fn set_llm_callback(&mut self, callback: Arc<dyn LlmCallback>) {
        self.llm_callback = Some(callback);
    }

    /// Set the channel send callback. Extensions calling `channel_send` will use this.
    pub fn set_channel_callback(&mut self, callback: Arc<dyn ChannelCallback>) {
        self.channel_callback = Some(callback);
    }

    /// Set the MCP tool invocation callback. Extensions calling `mcp_call` will use this.
    pub fn set_mcp_callback(&mut self, callback: Arc<dyn McpCallback>) {
        self.mcp_callback = Some(callback);
    }

    /// Load instance metas from the database (called during startup).
    pub fn load_instance_metas_sync(&self, metas: Vec<ExtensionInstanceMeta>) {
        let mut map = self.instance_metas.blocking_write();
        for meta in metas {
            let instance_id = format_instance_id(&meta.extension_id, &meta.instance_name);
            map.insert(instance_id, meta);
        }
    }

    fn bundled_dir(&self) -> PathBuf {
        self.extensions_dir.join("bundled")
    }

    fn user_dir(&self) -> PathBuf {
        self.extensions_dir.join("user")
    }

    // ── Instance CRUD ───────────────────────────────────────────────

    /// Create a new named instance of an installed extension.
    /// Returns the full instance_id (e.g., "com.example.ext::support-bot").
    pub async fn create_instance(
        &self,
        extension_id: &str,
        instance_name: &str,
        display_name: Option<String>,
    ) -> Result<String> {
        // Validate extension exists
        let registry = self.registry.read().await;
        if !registry.contains_key(extension_id) {
            return Err(ExtensionError::NotFound(extension_id.to_string()));
        }
        drop(registry);

        let instance_id = format_instance_id(extension_id, instance_name);

        // Check for duplicates
        let metas = self.instance_metas.read().await;
        if metas.contains_key(&instance_id) {
            return Err(ExtensionError::Other(format!(
                "Instance '{}' already exists for extension '{}'",
                instance_name, extension_id
            )));
        }
        drop(metas);

        let meta = ExtensionInstanceMeta {
            instance_name: instance_name.to_string(),
            extension_id: extension_id.to_string(),
            display_name: display_name.clone(),
            enabled: true,
            created_at: Utc::now(),
        };

        // Persist to database
        let db = self.db.clone();
        let iid = instance_id.clone();
        let eid = extension_id.to_string();
        let iname = instance_name.to_string();
        let dname = display_name.clone();
        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| {
                omni_core::error::OmniError::Other(format!("Database mutex poisoned: {}", e))
            })?;
            db.create_extension_instance(&iid, &eid, &iname, dname.as_deref())
        })
        .await
        .unwrap_or(Err(omni_core::error::OmniError::Other(
            "spawn_blocking failed".to_string(),
        )))
        .map_err(ExtensionError::Core)?;

        self.instance_metas
            .write()
            .await
            .insert(instance_id.clone(), meta);

        self.event_bus.emit(OmniEvent::ExtensionInstanceCreated {
            instance_id: instance_id.clone(),
            extension_id: extension_id.to_string(),
            instance_name: instance_name.to_string(),
        });

        tracing::info!(
            instance = %instance_id,
            extension = extension_id,
            "Extension instance created"
        );

        Ok(instance_id)
    }

    /// Delete an extension instance. Deactivates it first if active.
    pub async fn delete_instance(&self, instance_id: &str) -> Result<()> {
        let instance_id = resolve_instance_id(instance_id);

        // Deactivate if running
        if self.instances.read().await.contains_key(&instance_id) {
            if let Err(e) = self.deactivate(&instance_id).await {
                tracing::warn!(
                    instance = %instance_id,
                    error = %e,
                    "Failed to deactivate instance during delete, continuing"
                );
            }
        }

        let meta = self.instance_metas.write().await.remove(&instance_id);
        let extension_id = meta
            .as_ref()
            .map(|m| m.extension_id.clone())
            .unwrap_or_default();

        // Delete from database
        let db = self.db.clone();
        let iid = instance_id.clone();
        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| {
                omni_core::error::OmniError::Other(format!("Database mutex poisoned: {}", e))
            })?;
            db.delete_extension_instance(&iid)?;
            // Also clean up instance-scoped state
            db.delete_extension_state(&iid)?;
            Ok::<_, omni_core::error::OmniError>(())
        })
        .await
        .unwrap_or(Err(omni_core::error::OmniError::Other(
            "spawn_blocking failed".to_string(),
        )))
        .map_err(ExtensionError::Core)?;

        self.event_bus.emit(OmniEvent::ExtensionInstanceDeleted {
            instance_id: instance_id.clone(),
            extension_id,
        });

        tracing::info!(instance = %instance_id, "Extension instance deleted");
        Ok(())
    }

    /// List all instance metas, optionally filtered by extension_id.
    pub async fn list_instances(
        &self,
        extension_id: Option<&str>,
    ) -> Vec<(String, ExtensionInstanceMeta)> {
        let metas = self.instance_metas.read().await;
        let instances = self.instances.read().await;

        metas
            .iter()
            .filter(|(_, meta)| {
                extension_id.map_or(true, |eid| meta.extension_id == eid)
            })
            .map(|(iid, meta)| {
                let _ = instances.contains_key(iid); // just to suppress unused warning
                (iid.clone(), meta.clone())
            })
            .collect()
    }

    /// Update the display name or enabled state of an instance.
    pub async fn update_instance(
        &self,
        instance_id: &str,
        display_name: Option<String>,
    ) -> Result<()> {
        let instance_id = resolve_instance_id(instance_id);

        let mut metas = self.instance_metas.write().await;
        let meta = metas
            .get_mut(&instance_id)
            .ok_or_else(|| ExtensionError::NotFound(instance_id.clone()))?;

        meta.display_name = display_name.clone();

        // Persist
        let db = self.db.clone();
        let iid = instance_id.clone();
        let dname = display_name;
        let enabled = meta.enabled;
        drop(metas);

        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| {
                omni_core::error::OmniError::Other(format!("Database mutex poisoned: {}", e))
            })?;
            db.update_extension_instance(&iid, dname.as_deref(), enabled)
        })
        .await
        .unwrap_or(Err(omni_core::error::OmniError::Other(
            "spawn_blocking failed".to_string(),
        )))
        .map_err(ExtensionError::Core)?;

        Ok(())
    }

    // ── Discovery & Install ─────────────────────────────────────────

    /// Discover and register extensions found in the bundled and user directories.
    /// Auto-creates a `::default` instance for any extension that has no instances.
    /// Returns the list of extension IDs that were newly registered or updated.
    pub async fn discover_and_register(&self) -> Result<Vec<String>> {
        let mut registered = Vec::new();

        for dir in &[self.bundled_dir(), self.user_dir()] {
            if !dir.exists() {
                continue;
            }
            let entries = std::fs::read_dir(dir).map_err(ExtensionError::Io)?;
            for entry in entries {
                let entry = entry.map_err(ExtensionError::Io)?;
                let ext_path = entry.path();
                let manifest_path = ext_path.join("omni-extension.toml");
                if !manifest_path.exists() {
                    continue;
                }
                match ExtensionManifest::load(&manifest_path) {
                    Ok(manifest) => {
                        let ext_id = manifest.extension.id.clone();

                        // Check if already registered -- update if newer version on disk
                        {
                            let registry = self.registry.read().await;
                            if let Some(existing) = registry.get(&ext_id) {
                                let existing_ver =
                                    semver::Version::parse(&existing.manifest.extension.version)
                                        .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
                                let new_ver =
                                    semver::Version::parse(&manifest.extension.version)
                                        .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
                                if new_ver <= existing_ver {
                                    continue;
                                }
                                tracing::info!(
                                    extension = %ext_id,
                                    old_version = %existing_ver,
                                    new_version = %new_ver,
                                    "Updating discovered extension to newer version"
                                );
                            }
                        }

                        // Validate WASM entrypoint exists
                        let wasm_path = ext_path.join(&manifest.runtime.entrypoint);
                        if !wasm_path.exists() {
                            tracing::warn!(
                                extension = %ext_id,
                                "Skipping discovered extension: missing entrypoint {}",
                                manifest.runtime.entrypoint
                            );
                            continue;
                        }

                        let installed = InstalledExtension {
                            manifest,
                            install_path: ext_path,
                            installed_at: Utc::now(),
                            updated_at: Utc::now(),
                            enabled: true,
                        };

                        self.registry
                            .write()
                            .await
                            .insert(ext_id.clone(), installed);

                        // Ensure at least a ::default instance exists
                        self.ensure_default_instance(&ext_id).await;

                        registered.push(ext_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %manifest_path.display(),
                            error = %e,
                            "Failed to load extension manifest"
                        );
                    }
                }
            }
        }

        Ok(registered)
    }

    /// Install an extension from a source.
    pub async fn install(&self, source: &ExtensionSource) -> Result<String> {
        match source {
            ExtensionSource::Path(path) => self.install_from_path(path).await,
        }
    }

    async fn install_from_path(&self, source_path: &std::path::Path) -> Result<String> {
        // 1. Load and validate manifest
        let manifest_path = source_path.join("omni-extension.toml");
        let manifest = ExtensionManifest::load(&manifest_path)?;

        // 2. Validate WASM module exists and has valid magic bytes
        let wasm_path = source_path.join(&manifest.runtime.entrypoint);
        if !wasm_path.exists() {
            return Err(ExtensionError::MissingEntrypoint(
                manifest.runtime.entrypoint.clone(),
            ));
        }
        let wasm_header = std::fs::read(&wasm_path)
            .map_err(ExtensionError::Io)?;
        if wasm_header.len() < 4 || &wasm_header[..4] != b"\0asm" {
            return Err(ExtensionError::Wasm(
                "Invalid WASM module: missing magic bytes".to_string(),
            ));
        }

        // 3. Copy to user extensions directory
        let install_dir = self.user_dir().join(&manifest.extension.id);
        if install_dir.exists() {
            std::fs::remove_dir_all(&install_dir).map_err(ExtensionError::Io)?;
        }
        copy_dir_recursive(source_path, &install_dir)?;

        // 4. Register
        let extension_id = manifest.extension.id.clone();
        let installed = InstalledExtension {
            manifest,
            install_path: install_dir,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            enabled: true,
        };

        self.registry
            .write()
            .await
            .insert(extension_id.clone(), installed);

        // 5. Ensure default instance exists
        self.ensure_default_instance(&extension_id).await;

        // 6. Emit event
        self.event_bus.emit(OmniEvent::ExtensionInstalled {
            extension_id: extension_id.clone(),
        });

        Ok(extension_id)
    }

    /// Ensure a `::default` instance meta exists for the given extension.
    async fn ensure_default_instance(&self, extension_id: &str) {
        let default_id = format_instance_id(extension_id, "default");
        let metas = self.instance_metas.read().await;

        // Check if any instance exists for this extension
        let has_instances = metas
            .values()
            .any(|m| m.extension_id == extension_id);

        if has_instances {
            return;
        }
        drop(metas);

        let meta = ExtensionInstanceMeta {
            instance_name: "default".to_string(),
            extension_id: extension_id.to_string(),
            display_name: None,
            enabled: true,
            created_at: Utc::now(),
        };

        self.instance_metas
            .write()
            .await
            .insert(default_id.clone(), meta);

        // Best-effort persist to DB
        let db = self.db.clone();
        let did = default_id;
        let eid = extension_id.to_string();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(db) = db.lock() {
                let _ = db.create_extension_instance(&did, &eid, "default", None);
            }
        })
        .await;
    }

    // ── Activation / Deactivation ───────────────────────────────────

    /// Activate an extension instance (load its WASM and create sandbox).
    ///
    /// Accepts either a full instance_id ("ext::name") or a bare extension_id
    /// (auto-resolved to "ext::default" for backward compatibility).
    pub async fn activate(&self, id: &str) -> Result<()> {
        let instance_id = resolve_instance_id(id);
        let (ext_id, _inst_name) = parse_instance_id(&instance_id);

        // Check instance meta exists and is enabled
        {
            let metas = self.instance_metas.read().await;
            if let Some(meta) = metas.get(&instance_id) {
                if !meta.enabled {
                    return Err(ExtensionError::Disabled(instance_id));
                }
            }
            // If no meta exists but the extension exists, that's fine -- we'll create
            // the default instance on the fly below.
        }

        let registry = self.registry.write().await;
        let installed = registry
            .get(ext_id)
            .ok_or_else(|| ExtensionError::NotFound(ext_id.to_string()))?;

        if !installed.enabled {
            return Err(ExtensionError::Disabled(ext_id.to_string()));
        }

        let wasm_path = installed
            .install_path
            .join(&installed.manifest.runtime.entrypoint);
        let wasm_bytes = std::fs::read(&wasm_path).map_err(ExtensionError::Io)?;

        let max_memory_bytes =
            (installed.manifest.runtime.max_memory_mb as usize) * 1024 * 1024;

        // Storage uses instance_id for isolation; permissions use extension_id
        let state = WasmSandbox::create_state_with_callbacks(
            ext_id,
            &instance_id,
            Arc::clone(&self.policy_engine),
            Arc::clone(&self.db),
            max_memory_bytes,
            self.llm_callback.clone(),
            self.channel_callback.clone(),
            self.mcp_callback.clone(),
        );

        let limits = ResourceLimits {
            max_fuel: installed.manifest.runtime.max_cpu_ms_per_call * 1_000_000,
            max_memory_bytes,
        };

        let max_concurrent = installed.manifest.runtime.max_concurrent_calls;
        let instance = self.sandbox.instantiate(&wasm_bytes, state, &limits)?;

        // Drop the registry write lock before acquiring instances write lock
        drop(registry);

        self.instances
            .write()
            .await
            .insert(instance_id.clone(), instance);

        self.concurrency_semaphores
            .write()
            .await
            .insert(
                instance_id.clone(),
                Arc::new(Semaphore::new(max_concurrent as usize)),
            );

        self.event_bus.emit(OmniEvent::ExtensionActivated {
            extension_id: instance_id.clone(),
        });
        tracing::info!(instance = %instance_id, "Extension instance activated");
        Ok(())
    }

    /// Invoke a tool on an active extension instance.
    pub async fn invoke_tool(
        &self,
        id: &str,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> std::result::Result<serde_json::Value, ExtensionError> {
        let instance_id = resolve_instance_id(id);
        let (ext_id, _) = parse_instance_id(&instance_id);

        // Emit invocation event
        self.event_bus.emit(OmniEvent::ExtensionInvoked {
            extension_id: instance_id.clone(),
            tool_name: tool_name.to_string(),
            params: params.clone(),
        });

        // Read timeout from manifest
        let timeout = {
            let registry = self.registry.read().await;
            registry
                .get(ext_id)
                .map(|inst| Duration::from_millis(inst.manifest.runtime.max_cpu_ms_per_call))
                .unwrap_or_else(|| Duration::from_millis(5000))
        };

        // Acquire concurrency semaphore permit
        let _semaphore = self
            .concurrency_semaphores
            .read()
            .await
            .get(&instance_id)
            .cloned();
        let _permit = match &_semaphore {
            Some(sem) => Some(
                sem.acquire()
                    .await
                    .map_err(|_| ExtensionError::Other("Concurrency semaphore closed".to_string()))?,
            ),
            None => None,
        };

        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(&instance_id)
            .ok_or_else(|| ExtensionError::NotActive(instance_id.clone()))?;

        let params_json = serde_json::to_string(params)
            .map_err(|e| ExtensionError::Other(e.to_string()))?;

        match WasmSandbox::call_tool(instance, tool_name, &params_json, timeout) {
            Ok(result_json) => {
                let result: serde_json::Value = serde_json::from_str(&result_json)
                    .map_err(|e| ExtensionError::Other(e.to_string()))?;

                self.event_bus.emit(OmniEvent::ExtensionResult {
                    extension_id: instance_id.clone(),
                    tool_name: tool_name.to_string(),
                    result: result.clone(),
                });

                Ok(result)
            }
            Err(e) => {
                self.event_bus.emit(OmniEvent::ExtensionError {
                    extension_id: instance_id.clone(),
                    error: e.to_string(),
                });
                Err(e)
            }
        }
    }

    /// Deactivate an extension instance (stop its sandbox but keep it installed).
    pub async fn deactivate(&self, id: &str) -> Result<()> {
        let instance_id = resolve_instance_id(id);
        self.instances.write().await.remove(&instance_id);
        self.concurrency_semaphores.write().await.remove(&instance_id);
        self.event_bus.emit(OmniEvent::ExtensionDeactivated {
            extension_id: instance_id.clone(),
        });
        tracing::info!(instance = %instance_id, "Extension instance deactivated");
        Ok(())
    }

    /// Uninstall an extension completely -- deletes ALL instances and files.
    pub async fn uninstall(&self, extension_id: &str) -> Result<()> {
        // Deactivate and delete all instances for this extension
        let instance_ids: Vec<String> = self
            .instance_metas
            .read()
            .await
            .iter()
            .filter(|(_, meta)| meta.extension_id == extension_id)
            .map(|(iid, _)| iid.clone())
            .collect();

        for iid in &instance_ids {
            if let Err(e) = self.delete_instance(iid).await {
                tracing::warn!(
                    instance = %iid,
                    error = %e,
                    "Failed to delete instance during uninstall, continuing"
                );
            }
        }

        // Also deactivate the bare extension_id (backward compat: may have been
        // activated with just the extension_id before migration)
        let _ = self.deactivate(extension_id).await;

        // Revoke all permissions
        self.policy_engine
            .revoke_all(extension_id)
            .await
            .map_err(ExtensionError::Core)?;

        // Remove files and registry entry
        let mut registry = self.registry.write().await;
        if let Some(installed) = registry.remove(extension_id) {
            if installed.install_path.exists() {
                std::fs::remove_dir_all(&installed.install_path)
                    .map_err(ExtensionError::Io)?;
            }
        }

        // Clean up extension instances from database
        let db = self.db.clone();
        let ext_id = extension_id.to_string();
        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| {
                omni_core::error::OmniError::Other(format!("Database mutex poisoned: {}", e))
            })?;
            db.delete_extension_instances_for(&ext_id)?;
            // Also clean up any remaining state for the bare extension_id
            db.delete_extension_state(&ext_id)?;
            Ok::<_, omni_core::error::OmniError>(())
        })
        .await
        .unwrap_or(Err(omni_core::error::OmniError::Other(
            "spawn_blocking failed".to_string(),
        )))
        .map_err(ExtensionError::Core)?;

        self.event_bus.emit(OmniEvent::ExtensionRemoved {
            extension_id: extension_id.to_string(),
        });

        Ok(())
    }

    // ── Query Methods ───────────────────────────────────────────────

    /// Get tool definitions from all active instances.
    /// Returns (instance_id, ToolDefinition) pairs.
    pub async fn get_all_tools(
        &self,
    ) -> Vec<(String, crate::manifest::ToolDefinition)> {
        let registry = self.registry.read().await;
        let instances = self.instances.read().await;
        let metas = self.instance_metas.read().await;

        let mut tools = Vec::new();
        for instance_id in instances.keys() {
            let ext_id = metas
                .get(instance_id)
                .map(|m| m.extension_id.as_str())
                .unwrap_or_else(|| parse_instance_id(instance_id).0);

            if let Some(installed) = registry.get(ext_id) {
                for tool in &installed.manifest.tools {
                    tools.push((instance_id.clone(), tool.clone()));
                }
            }
        }
        tools
    }

    /// Get the MCP server declarations for an installed extension.
    /// Accepts either extension_id or instance_id (resolves to extension).
    pub async fn get_mcp_servers(&self, id: &str) -> Vec<crate::manifest::McpServerDeclaration> {
        let ext_id = parse_instance_id(id).0;
        self.registry
            .read()
            .await
            .get(ext_id)
            .map(|inst| inst.manifest.mcp_servers.clone())
            .unwrap_or_default()
    }

    /// Check if an instance is currently active.
    pub async fn is_active(&self, id: &str) -> bool {
        let instance_id = resolve_instance_id(id);
        self.instances.read().await.contains_key(&instance_id)
    }

    /// List all installed extension IDs (base packages, not instances).
    pub async fn list_installed(&self) -> Vec<String> {
        self.registry
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    /// List all installed extensions with full details from their manifests.
    pub async fn list_installed_details(&self) -> Vec<ExtensionDetails> {
        let registry = self.registry.read().await;
        let instances = self.instances.read().await;
        let metas = self.instance_metas.read().await;

        registry
            .iter()
            .map(|(id, installed)| {
                let manifest = &installed.manifest;

                // Count instances for this extension
                let instance_count = metas
                    .values()
                    .filter(|m| m.extension_id == *id)
                    .count();

                // Check if any instance is active
                let any_active = metas
                    .iter()
                    .any(|(iid, m)| m.extension_id == *id && instances.contains_key(iid));

                ExtensionDetails {
                    id: id.clone(),
                    name: manifest.extension.name.clone(),
                    version: manifest.extension.version.clone(),
                    author: manifest.extension.author.clone(),
                    description: manifest.extension.description.clone(),
                    enabled: installed.enabled,
                    active: any_active,
                    tools: manifest.tools.iter().map(|t| t.name.clone()).collect(),
                    permissions: manifest
                        .permissions
                        .iter()
                        .map(|p| p.capability.clone())
                        .collect(),
                    instance_count,
                }
            })
            .collect()
    }

    /// Toggle the enabled state of an instance.
    /// If disabling, also deactivates. If enabling, also auto-activates.
    pub async fn set_instance_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let instance_id = resolve_instance_id(id);

        {
            let mut metas = self.instance_metas.write().await;
            let meta = metas
                .get_mut(&instance_id)
                .ok_or_else(|| ExtensionError::NotFound(instance_id.clone()))?;
            meta.enabled = enabled;

            // Persist
            let db = self.db.clone();
            let iid = instance_id.clone();
            let dname = meta.display_name.clone();
            drop(metas);
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(db) = db.lock() {
                    let _ = db.update_extension_instance(&iid, dname.as_deref(), enabled);
                }
            })
            .await;
        }

        if !enabled {
            if let Err(e) = self.deactivate(&instance_id).await {
                tracing::warn!(
                    instance = %instance_id,
                    error = %e,
                    "Failed to deactivate instance while disabling"
                );
            }
        } else {
            if let Err(e) = self.activate(&instance_id).await {
                tracing::warn!(
                    instance = %instance_id,
                    error = %e,
                    "Failed to auto-activate instance while enabling"
                );
            }
        }

        Ok(())
    }

    /// Toggle the enabled state of an installed extension (base package).
    /// Affects ALL instances: disabling deactivates all, enabling activates all.
    pub async fn set_enabled(&self, extension_id: &str, enabled: bool) -> Result<()> {
        {
            let mut registry = self.registry.write().await;
            let installed = registry
                .get_mut(extension_id)
                .ok_or_else(|| ExtensionError::NotFound(extension_id.to_string()))?;
            installed.enabled = enabled;
        }

        // Get all instance_ids for this extension
        let instance_ids: Vec<String> = self
            .instance_metas
            .read()
            .await
            .iter()
            .filter(|(_, meta)| meta.extension_id == extension_id)
            .map(|(iid, _)| iid.clone())
            .collect();

        if !enabled {
            for iid in &instance_ids {
                if let Err(e) = self.deactivate(iid).await {
                    tracing::warn!(
                        instance = %iid,
                        error = %e,
                        "Failed to deactivate instance while disabling extension"
                    );
                }
            }
        } else {
            for iid in &instance_ids {
                if let Err(e) = self.activate(iid).await {
                    tracing::warn!(
                        instance = %iid,
                        error = %e,
                        "Failed to auto-activate instance while enabling extension"
                    );
                }
            }
        }

        Ok(())
    }
}

/// Detailed info about an installed extension, derived from its manifest.
pub struct ExtensionDetails {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub active: bool,
    pub tools: Vec<String>,
    pub permissions: Vec<String>,
    pub instance_count: usize,
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(ExtensionError::Io)?;
    for entry in std::fs::read_dir(src).map_err(ExtensionError::Io)? {
        let entry = entry.map_err(ExtensionError::Io)?;
        let ty = entry.file_type().map_err(ExtensionError::Io)?;
        // Reject symlinks to prevent directory traversal / escape attacks
        if ty.is_symlink() {
            return Err(ExtensionError::Other(format!(
                "Symlink found during extension install: {}. Symlinks are not allowed in extension packages.",
                entry.path().display()
            )));
        }
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path).map_err(ExtensionError::Io)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instance_id_with_separator() {
        let (ext, name) = parse_instance_id("com.example.ext::support-bot");
        assert_eq!(ext, "com.example.ext");
        assert_eq!(name, "support-bot");
    }

    #[test]
    fn test_parse_instance_id_default() {
        let (ext, name) = parse_instance_id("com.example.ext::default");
        assert_eq!(ext, "com.example.ext");
        assert_eq!(name, "default");
    }

    #[test]
    fn test_parse_instance_id_bare_extension() {
        let (ext, name) = parse_instance_id("com.example.ext");
        assert_eq!(ext, "com.example.ext");
        assert_eq!(name, "default");
    }

    #[test]
    fn test_parse_instance_id_empty_name() {
        // Edge case: trailing :: with empty name
        let (ext, name) = parse_instance_id("com.example.ext::");
        assert_eq!(ext, "com.example.ext");
        assert_eq!(name, "");
    }

    #[test]
    fn test_parse_instance_id_multiple_separators() {
        // Only splits on first ::
        let (ext, name) = parse_instance_id("com.example::nested::deep");
        assert_eq!(ext, "com.example");
        assert_eq!(name, "nested::deep");
    }

    #[test]
    fn test_format_instance_id() {
        let id = format_instance_id("com.example.ext", "support-bot");
        assert_eq!(id, "com.example.ext::support-bot");
    }

    #[test]
    fn test_format_instance_id_default() {
        let id = format_instance_id("com.example.ext", "default");
        assert_eq!(id, "com.example.ext::default");
    }

    #[test]
    fn test_resolve_instance_id_bare() {
        let resolved = resolve_instance_id("com.example.ext");
        assert_eq!(resolved, "com.example.ext::default");
    }

    #[test]
    fn test_resolve_instance_id_already_full() {
        let resolved = resolve_instance_id("com.example.ext::custom");
        assert_eq!(resolved, "com.example.ext::custom");
    }

    #[test]
    fn test_resolve_instance_id_default() {
        let resolved = resolve_instance_id("com.example.ext::default");
        assert_eq!(resolved, "com.example.ext::default");
    }

    #[test]
    fn test_format_parse_roundtrip() {
        let ext_id = "com.example.world-tools";
        let inst_name = "support-bot";
        let instance_id = format_instance_id(ext_id, inst_name);
        let (parsed_ext, parsed_name) = parse_instance_id(&instance_id);
        assert_eq!(parsed_ext, ext_id);
        assert_eq!(parsed_name, inst_name);
    }

    #[test]
    fn test_resolve_then_parse_roundtrip() {
        let bare = "com.example.ext";
        let resolved = resolve_instance_id(bare);
        let (ext, name) = parse_instance_id(&resolved);
        assert_eq!(ext, bare);
        assert_eq!(name, "default");
    }
}
