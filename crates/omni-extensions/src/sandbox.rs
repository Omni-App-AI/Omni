use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;

use omni_core::database::Database;
use omni_permissions::capability::Capability;
use omni_permissions::policy::{PermissionDecision, PolicyEngine};

use crate::error::{ExtensionError, Result};
use crate::storage::{DatabaseStorage, ExtensionStorage};

/// Reports streaming text chunks during LLM inference.
/// Passed to `LlmCallback::request_with_progress` to receive incremental updates.
pub trait LlmProgressReporter: Send + Sync {
    /// Called with each text delta as it arrives from the LLM.
    fn on_chunk(&self, text: &str);
}

/// Callback for LLM inference requests from extensions.
/// Implemented by the runtime layer (omni-llm) and passed into the sandbox.
pub trait LlmCallback: Send + Sync {
    /// Send a prompt to the configured LLM and return the response text.
    /// This is a blocking call -- the extension will wait for the response.
    fn request(&self, prompt: &str, max_tokens: Option<u32>) -> std::result::Result<String, String>;

    /// Send a prompt with streaming progress reporting.
    /// Each text delta is reported via the `progress` callback as it arrives.
    /// Returns the complete response text.
    /// Default implementation calls `request()` and reports the full text as one chunk.
    fn request_with_progress(
        &self,
        prompt: &str,
        max_tokens: Option<u32>,
        progress: &dyn LlmProgressReporter,
    ) -> std::result::Result<String, String> {
        let result = self.request(prompt, max_tokens)?;
        progress.on_chunk(&result);
        Ok(result)
    }
}

/// Callback for sending messages through channel plugins from extensions.
/// Implemented by the runtime layer and passed into the sandbox.
pub trait ChannelCallback: Send + Sync {
    /// Send a message through a connected channel.
    /// Returns JSON result string on success.
    fn send_message(
        &self,
        channel_id: &str,
        recipient: &str,
        text: &str,
    ) -> std::result::Result<String, String>;
}

/// Callback for executing native tools from flowchart nodes.
/// Implemented by the runtime layer to bridge to NativeToolRegistry.
/// The callback is synchronous (blocking) -- the flowchart engine calls it
/// from an async context using spawn_blocking or scoped threads.
pub trait NativeToolCallback: Send + Sync {
    /// Execute a native tool by name with JSON-serialized parameters.
    /// Returns JSON-serialized result on success.
    fn execute(
        &self,
        tool_name: &str,
        params_json: &str,
    ) -> std::result::Result<String, String>;

    /// Get the list of available tool names and their JSON schemas.
    /// Returns JSON array of `{ "name": "...", "description": "...", "parameters": {...} }`.
    fn list_tools(&self) -> std::result::Result<String, String>;
}

/// Callback for invoking another flowchart from a SubFlow node.
/// Implemented by the runtime layer to bridge to the FlowchartRegistry.
/// The callback is synchronous (blocking) -- the flowchart engine calls it
/// from an async context using spawn_blocking.
pub trait FlowchartCallback: Send + Sync {
    /// Invoke a tool on another flowchart. Returns JSON-serialized result.
    /// `depth` tracks recursion level to prevent infinite sub-flow chains.
    fn invoke(
        &self,
        flowchart_id: &str,
        tool_name: &str,
        params_json: &str,
        depth: u32,
    ) -> std::result::Result<String, String>;
}

/// Callback for Guardian anti-injection scanning from within the flowchart engine.
/// Implemented by the runtime layer to bridge to the Guardian scanner.
/// This enables scan-on-use: every time the flow engine invokes an external
/// service (LLM, channel, native tool, sub-flow, HTTP), the content is scanned.
pub trait GuardianCallback: Send + Sync {
    /// Scan input content (prompts, params, messages) before it leaves the flow engine.
    /// Returns `Ok(())` if clean, `Err(reason)` if blocked.
    fn scan_input(&self, content: &str) -> std::result::Result<(), String>;

    /// Scan output/result content coming back into the flow engine.
    /// `source_id` identifies the origin (flowchart ID, "omni.native", etc.).
    /// Returns `Ok(())` if clean, `Err(reason)` if blocked.
    fn scan_output(&self, source_id: &str, content: &str) -> std::result::Result<(), String>;
}

/// Callback for invoking the full agent loop (multi-turn LLM with tool use).
/// Used by the AgentRequest flowchart node to delegate complex reasoning to the agent.
pub trait AgentCallback: Send + Sync {
    /// Run the agent loop with the given user message.
    /// The agent will use its full tool set and multi-turn reasoning.
    /// `system_prompt` optionally overrides the agent's system prompt.
    /// `max_iterations` limits the number of LLM round-trips.
    /// Returns the agent's final text response.
    fn run(
        &self,
        user_message: &str,
        system_prompt: Option<&str>,
        max_iterations: Option<u32>,
    ) -> std::result::Result<String, String>;
}

/// Callback for invoking MCP tools from extensions.
/// Implemented by the runtime layer to bridge to the MCP manager.
pub trait McpCallback: Send + Sync {
    /// Execute an MCP tool by server name and tool name.
    /// `params_json` is the JSON-serialized tool arguments.
    /// Returns the JSON-serialized tool result on success.
    fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        params_json: &str,
    ) -> std::result::Result<String, String>;
}

/// Configuration for sandbox engine creation.
#[derive(Default)]
pub struct SandboxConfig {
    pub async_support: bool,
}

/// The WASM sandbox runtime. Holds the engine and pre-configured linker.
pub struct WasmSandbox {
    engine: Engine,
}

/// State shared between host and WASM guest.
pub struct SandboxState {
    pub extension_id: String,
    /// Full instance ID (e.g., "com.example.ext::my-instance").
    /// Used for storage namespace isolation and event attribution.
    pub instance_id: String,
    pub wasi: WasiP1Ctx,
    pub policy_engine: Arc<PolicyEngine>,
    pub storage: Arc<dyn ExtensionStorage>,
    pub store_limits: StoreLimits,
    /// Bump allocator offset -- tracks where host-written data starts in WASM memory.
    /// Grows downward from the end of memory to avoid clobbering guest heap.
    pub host_alloc_offset: AtomicU32,
    /// Optional LLM inference callback
    pub llm_callback: Option<Arc<dyn LlmCallback>>,
    /// Optional channel send callback
    pub channel_callback: Option<Arc<dyn ChannelCallback>>,
    /// Optional MCP tool invocation callback
    pub mcp_callback: Option<Arc<dyn McpCallback>>,
}

/// Resource limits for a WASM instance.
pub struct ResourceLimits {
    pub max_fuel: u64,
    pub max_memory_bytes: usize,
}

/// A running WASM extension instance.
pub struct ExtensionInstance {
    pub store: Store<SandboxState>,
    pub instance: Instance,
}

// --- Memory helpers ---

fn read_bytes_from_memory(
    memory: &Memory,
    store: &impl AsContext,
    ptr: u32,
    len: u32,
) -> Vec<u8> {
    let data = memory.data(store);
    let start = ptr as usize;
    let end = start + len as usize;
    if end > data.len() {
        return Vec::new();
    }
    data[start..end].to_vec()
}

fn read_string_from_memory(
    memory: &Memory,
    store: &impl AsContext,
    ptr: u32,
    len: u32,
) -> String {
    let bytes = read_bytes_from_memory(memory, store, ptr, len);
    String::from_utf8_lossy(&bytes).to_string()
}

/// Host-side bump allocator. Allocates from the top of WASM linear memory
/// growing downward. Each call returns a pointer to a contiguous region.
///
/// `current_offset` is the current allocator position (0 means uninitialized,
/// will start from `mem_size`). Returns `(ptr, new_offset)` on success.
fn host_alloc(
    memory: &Memory,
    caller: &mut impl AsContextMut,
    data: &[u8],
    current_offset: u32,
    mem_size: u32,
) -> std::result::Result<(u32, u32), anyhow::Error> {
    let len = data.len() as u32;
    // Align to 8 bytes for safety
    let aligned_len = (len + 7) & !7;
    let base = if current_offset == 0 { mem_size } else { current_offset };
    if base < aligned_len + 4096 {
        return Err(anyhow::anyhow!(
            "Host allocator exhausted: need {} bytes, only {} available above guest heap",
            aligned_len,
            base.saturating_sub(4096)
        ));
    }
    let ptr = base - aligned_len;
    memory
        .write(caller, ptr as usize, data)
        .map_err(|e| anyhow::anyhow!("Failed to write to WASM memory: {e}"))?;
    Ok((ptr, ptr))
}


fn encode_ptr_len(ptr: u32, len: u32) -> i64 {
    ((ptr as i64) << 32) | (len as i64)
}

fn decode_ptr_len(packed: i64) -> (u32, u32) {
    let ptr = (packed >> 32) as u32;
    let len = (packed & 0xFFFF_FFFF) as u32;
    (ptr, len)
}

impl WasmSandbox {
    pub fn new(_config: &SandboxConfig) -> Result<Self> {
        let mut engine_config = Config::new();
        engine_config
            .wasm_memory64(false)
            .wasm_threads(false)
            .consume_fuel(true)
            .epoch_interruption(true);

        let engine =
            Engine::new(&engine_config).map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        Ok(Self { engine })
    }

    /// Instantiate an extension from its WASM module bytes.
    pub fn instantiate(
        &self,
        wasm_bytes: &[u8],
        state: SandboxState,
        limits: &ResourceLimits,
    ) -> Result<ExtensionInstance> {
        let module =
            Module::new(&self.engine, wasm_bytes).map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        let mut linker: Linker<SandboxState> = Linker::new(&self.engine);

        // Add WASI preview1 support
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |state: &mut SandboxState| {
            &mut state.wasi
        })
        .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // Bind Omni host functions
        Self::bind_host_functions(&mut linker)?;

        let mut store = Store::new(&self.engine, state);
        store
            .set_fuel(limits.max_fuel)
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;
        store.limiter(|state| &mut state.store_limits);
        // Set a generous default epoch deadline; call_tool will override with a tighter one
        store.set_epoch_deadline(u64::MAX);

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        Ok(ExtensionInstance { store, instance })
    }

    fn bind_host_functions(linker: &mut Linker<SandboxState>) -> Result<()> {
        // --- log: always allowed ---
        linker
            .func_wrap(
                "omni",
                "log",
                |mut caller: Caller<'_, SandboxState>, level: u32, msg_ptr: u32, msg_len: u32| {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return,
                    };
                    let msg = read_string_from_memory(&memory, &caller, msg_ptr, msg_len);
                    let ext_id = &caller.data().extension_id;
                    match level {
                        0 => tracing::error!(extension = %ext_id, "{}", msg),
                        1 => tracing::warn!(extension = %ext_id, "{}", msg),
                        2 => tracing::info!(extension = %ext_id, "{}", msg),
                        _ => tracing::debug!(extension = %ext_id, "{}", msg),
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- storage_get ---
        linker
            .func_wrap(
                "omni",
                "storage_get",
                |mut caller: Caller<'_, SandboxState>, key_ptr: u32, key_len: u32| -> i64 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -1,
                    };
                    let key = read_string_from_memory(&memory, &caller, key_ptr, key_len);
                    let storage = caller.data().storage.clone();
                    match storage.get(&key) {
                        Ok(Some(value)) => {
                            let bytes = value.as_bytes();
                            let offset = caller.data().host_alloc_offset.load(Ordering::Relaxed);
                            let mem_size = memory.data_size(&caller) as u32;
                            match host_alloc(&memory, &mut caller, bytes, offset, mem_size) {
                                Ok((ptr, new_offset)) => {
                                    caller.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);
                                    encode_ptr_len(ptr, bytes.len() as u32)
                                }
                                Err(_) => -1,
                            }
                        }
                        _ => -1,
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- storage_set ---
        linker
            .func_wrap(
                "omni",
                "storage_set",
                |mut caller: Caller<'_, SandboxState>,
                 key_ptr: u32,
                 key_len: u32,
                 value_ptr: u32,
                 value_len: u32|
                 -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -1,
                    };
                    let key = read_string_from_memory(&memory, &caller, key_ptr, key_len);
                    let value = read_string_from_memory(&memory, &caller, value_ptr, value_len);
                    let storage = caller.data().storage.clone();
                    match storage.set(&key, &value) {
                        Ok(()) => 0,
                        Err(_) => -1,
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- http_request (permission-gated) ---
        // Returns packed (ptr << 32 | len) of JSON response on success,
        // -1 on permission denied, -2 on needs prompt, -3 on error.
        linker
            .func_wrap(
                "omni",
                "http_request",
                |mut caller: Caller<'_, SandboxState>,
                 url_ptr: u32,
                 url_len: u32,
                 method_ptr: u32,
                 method_len: u32,
                 body_ptr: u32,
                 body_len: u32|
                 -> i64 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -3,
                    };
                    let url_str = read_string_from_memory(&memory, &caller, url_ptr, url_len);
                    let method =
                        read_string_from_memory(&memory, &caller, method_ptr, method_len);
                    let body = if body_len > 0 {
                        read_bytes_from_memory(&memory, &caller, body_ptr, body_len)
                    } else {
                        Vec::new()
                    };

                    // Permission check
                    let state = caller.data();
                    let capability = Capability::NetworkHttp(None);
                    let decision =
                        state.policy_engine.check_sync(&state.extension_id, &capability);

                    match decision {
                        PermissionDecision::Allow => {}
                        PermissionDecision::Deny { .. } => return -1,
                        PermissionDecision::Prompt { .. } => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "Permission requires user prompt but extension runs in sync context; denying"
                            );
                            return -2;
                        }
                    }

                    // Execute the HTTP request using reqwest::blocking
                    let client = match reqwest::blocking::Client::builder()
                        .timeout(Duration::from_secs(30))
                        .build()
                    {
                        Ok(c) => c,
                        Err(e) => {
                            let ext_id = &caller.data().extension_id;
                            tracing::error!(extension = %ext_id, "Failed to build HTTP client: {}", e);
                            return -3;
                        }
                    };

                    let request = match method.to_uppercase().as_str() {
                        "GET" => client.get(&url_str),
                        "POST" => client.post(&url_str).body(body),
                        "PUT" => client.put(&url_str).body(body),
                        "DELETE" => client.delete(&url_str),
                        "PATCH" => client.patch(&url_str).body(body),
                        "HEAD" => client.head(&url_str),
                        _ => {
                            tracing::warn!("Unsupported HTTP method: {}", method);
                            return -3;
                        }
                    };

                    match request.send() {
                        Ok(response) => {
                            let status = response.status().as_u16();
                            let response_body = response
                                .bytes()
                                .map(|b| b.to_vec())
                                .unwrap_or_default();

                            // Cap response size to 5MB to prevent memory exhaustion
                            let capped_body = if response_body.len() > 5 * 1024 * 1024 {
                                response_body[..5 * 1024 * 1024].to_vec()
                            } else {
                                response_body
                            };

                            // Encode as JSON: {"status": N, "body": "base64..."}
                            let body_b64 = base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                &capped_body,
                            );
                            let response_json = serde_json::json!({
                                "status": status,
                                "body": body_b64,
                                "body_len": capped_body.len(),
                            });
                            let json_bytes = response_json.to_string().into_bytes();

                            let offset = caller.data().host_alloc_offset.load(Ordering::Relaxed);
                            let mem_size = memory.data_size(&caller) as u32;
                            match host_alloc(&memory, &mut caller, &json_bytes, offset, mem_size) {
                                Ok((ptr, new_offset)) => {
                                    caller.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);
                                    encode_ptr_len(ptr, json_bytes.len() as u32)
                                }
                                Err(e) => {
                                    tracing::error!("Failed to write HTTP response to WASM memory: {}", e);
                                    -3
                                }
                            }
                        }
                        Err(e) => {
                            let ext_id = &caller.data().extension_id;
                            tracing::error!(extension = %ext_id, "HTTP request failed: {}", e);
                            -3
                        }
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- fs_read (permission-gated) ---
        // Returns packed (ptr << 32 | len) of file contents on success,
        // -1 on permission denied, -2 on needs prompt, -3 on error.
        linker
            .func_wrap(
                "omni",
                "fs_read",
                |mut caller: Caller<'_, SandboxState>, path_ptr: u32, path_len: u32| -> i64 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -3,
                    };
                    let path_str = read_string_from_memory(&memory, &caller, path_ptr, path_len);

                    // Permission check
                    let state = caller.data();
                    let capability = Capability::FilesystemRead(None);
                    let decision =
                        state.policy_engine.check_sync(&state.extension_id, &capability);

                    match decision {
                        PermissionDecision::Allow => {}
                        PermissionDecision::Deny { .. } => return -1,
                        PermissionDecision::Prompt { .. } => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "Permission requires user prompt but extension runs in sync context; denying"
                            );
                            return -2;
                        }
                    }

                    let path = Path::new(&path_str);

                    // Read the file (cap at 10MB)
                    match std::fs::metadata(path) {
                        Ok(meta) => {
                            if meta.len() > 10 * 1024 * 1024 {
                                tracing::warn!("File too large for fs_read: {} bytes", meta.len());
                                return -3;
                            }
                        }
                        Err(e) => {
                            tracing::error!("fs_read metadata failed for '{}': {}", path_str, e);
                            return -3;
                        }
                    }

                    match std::fs::read(path) {
                        Ok(data) => {
                            let offset = caller.data().host_alloc_offset.load(Ordering::Relaxed);
                            let mem_size = memory.data_size(&caller) as u32;
                            match host_alloc(&memory, &mut caller, &data, offset, mem_size) {
                                Ok((ptr, new_offset)) => {
                                    caller.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);
                                    encode_ptr_len(ptr, data.len() as u32)
                                }
                                Err(e) => {
                                    tracing::error!("Failed to write file data to WASM memory: {}", e);
                                    -3
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("fs_read failed for '{}': {}", path_str, e);
                            -3
                        }
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- fs_write (permission-gated) ---
        // Returns 0 on success, -1 on permission denied, -2 on needs prompt, -3 on error.
        linker
            .func_wrap(
                "omni",
                "fs_write",
                |mut caller: Caller<'_, SandboxState>,
                 path_ptr: u32,
                 path_len: u32,
                 data_ptr: u32,
                 data_len: u32|
                 -> i32 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -3,
                    };
                    let path_str = read_string_from_memory(&memory, &caller, path_ptr, path_len);
                    let data = read_bytes_from_memory(&memory, &caller, data_ptr, data_len);

                    // Permission check
                    let state = caller.data();
                    let capability = Capability::FilesystemWrite(None);
                    let decision =
                        state.policy_engine.check_sync(&state.extension_id, &capability);

                    match decision {
                        PermissionDecision::Allow => {}
                        PermissionDecision::Deny { .. } => return -1,
                        PermissionDecision::Prompt { .. } => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "Permission requires user prompt but extension runs in sync context; denying"
                            );
                            return -2;
                        }
                    }

                    let path = Path::new(&path_str);

                    // Create parent directories if needed
                    if let Some(parent) = path.parent() {
                        if !parent.exists() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                tracing::error!("fs_write create_dir_all failed: {}", e);
                                return -3;
                            }
                        }
                    }

                    match std::fs::write(path, &data) {
                        Ok(()) => 0,
                        Err(e) => {
                            tracing::error!("fs_write failed for '{}': {}", path_str, e);
                            -3
                        }
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- process_spawn (permission-gated) ---
        // Executes a command and returns packed (ptr << 32 | len) of JSON result,
        // -1 on permission denied, -2 on needs prompt, -3 on error.
        // Input: cmd and args are newline-separated in the args buffer.
        linker
            .func_wrap(
                "omni",
                "process_spawn",
                |mut caller: Caller<'_, SandboxState>,
                 cmd_ptr: u32,
                 cmd_len: u32,
                 args_ptr: u32,
                 args_len: u32|
                 -> i64 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -3,
                    };
                    let cmd = read_string_from_memory(&memory, &caller, cmd_ptr, cmd_len);
                    let args_str = read_string_from_memory(&memory, &caller, args_ptr, args_len);
                    let args: Vec<&str> = if args_str.is_empty() {
                        Vec::new()
                    } else {
                        args_str.split('\n').collect()
                    };

                    // Permission check
                    let state = caller.data();
                    let capability = Capability::ProcessSpawn(None);
                    let decision =
                        state.policy_engine.check_sync(&state.extension_id, &capability);

                    match decision {
                        PermissionDecision::Allow => {}
                        PermissionDecision::Deny { .. } => return -1,
                        PermissionDecision::Prompt { .. } => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "Permission requires user prompt but extension runs in sync context; denying"
                            );
                            return -2;
                        }
                    }

                    // Execute the command
                    let output = std::process::Command::new(&cmd)
                        .args(&args)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .output();

                    match output {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let exit_code = output.status.code().unwrap_or(-1);

                            // Truncate output to 50KB each
                            let max_output = 50 * 1024;
                            let stdout_capped = if stdout.len() > max_output {
                                &stdout[..max_output]
                            } else {
                                &stdout
                            };
                            let stderr_capped = if stderr.len() > max_output {
                                &stderr[..max_output]
                            } else {
                                &stderr
                            };

                            let result_json = serde_json::json!({
                                "exit_code": exit_code,
                                "stdout": stdout_capped,
                                "stderr": stderr_capped,
                            });
                            let json_bytes = result_json.to_string().into_bytes();

                            let offset = caller.data().host_alloc_offset.load(Ordering::Relaxed);
                            let mem_size = memory.data_size(&caller) as u32;
                            match host_alloc(&memory, &mut caller, &json_bytes, offset, mem_size) {
                                Ok((ptr, new_offset)) => {
                                    caller.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);
                                    encode_ptr_len(ptr, json_bytes.len() as u32)
                                }
                                Err(e) => {
                                    tracing::error!("Failed to write process output to WASM memory: {}", e);
                                    -3
                                }
                            }
                        }
                        Err(e) => {
                            let ext_id = &caller.data().extension_id;
                            tracing::error!(extension = %ext_id, "process_spawn failed: {}", e);
                            -3
                        }
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- llm_request (permission-gated) ---
        // Sends a prompt to the configured LLM via the callback.
        // Returns packed (ptr << 32 | len) of response text on success,
        // -1 on permission denied, -2 on needs prompt, -3 on error, -4 on no callback.
        linker
            .func_wrap(
                "omni",
                "llm_request",
                |mut caller: Caller<'_, SandboxState>,
                 prompt_ptr: u32,
                 prompt_len: u32,
                 max_tokens: u32|
                 -> i64 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -3,
                    };
                    let prompt = read_string_from_memory(&memory, &caller, prompt_ptr, prompt_len);

                    // Permission check
                    let state = caller.data();
                    let capability = Capability::AiInference(None);
                    let decision =
                        state.policy_engine.check_sync(&state.extension_id, &capability);

                    match decision {
                        PermissionDecision::Allow => {}
                        PermissionDecision::Deny { .. } => return -1,
                        PermissionDecision::Prompt { .. } => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "Permission requires user prompt but extension runs in sync context; denying"
                            );
                            return -2;
                        }
                    }

                    // Get the callback
                    let callback = match &state.llm_callback {
                        Some(cb) => cb.clone(),
                        None => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "llm_request called but no LLM callback configured"
                            );
                            return -4;
                        }
                    };

                    let max_tok = if max_tokens > 0 { Some(max_tokens) } else { None };

                    match callback.request(&prompt, max_tok) {
                        Ok(response) => {
                            let bytes = response.as_bytes();
                            let offset = caller.data().host_alloc_offset.load(Ordering::Relaxed);
                            let mem_size = memory.data_size(&caller) as u32;
                            match host_alloc(&memory, &mut caller, bytes, offset, mem_size) {
                                Ok((ptr, new_offset)) => {
                                    caller.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);
                                    encode_ptr_len(ptr, bytes.len() as u32)
                                }
                                Err(e) => {
                                    tracing::error!("Failed to write LLM response to WASM memory: {}", e);
                                    -3
                                }
                            }
                        }
                        Err(e) => {
                            let ext_id = &caller.data().extension_id;
                            tracing::error!(extension = %ext_id, "LLM request failed: {}", e);
                            -3
                        }
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- channel_send (permission-gated) ---
        // Sends a message through a connected channel plugin.
        // Returns packed (ptr << 32 | len) of JSON result on success,
        // -1 on permission denied, -2 on needs prompt, -3 on error, -4 on no callback.
        linker
            .func_wrap(
                "omni",
                "channel_send",
                |mut caller: Caller<'_, SandboxState>,
                 channel_ptr: u32,
                 channel_len: u32,
                 recipient_ptr: u32,
                 recipient_len: u32,
                 text_ptr: u32,
                 text_len: u32|
                 -> i64 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -3,
                    };
                    let channel_id =
                        read_string_from_memory(&memory, &caller, channel_ptr, channel_len);
                    let recipient =
                        read_string_from_memory(&memory, &caller, recipient_ptr, recipient_len);
                    let text = read_string_from_memory(&memory, &caller, text_ptr, text_len);

                    // Permission check
                    let state = caller.data();
                    let capability = Capability::ChannelSend(None);
                    let decision =
                        state.policy_engine.check_sync(&state.extension_id, &capability);

                    match decision {
                        PermissionDecision::Allow => {}
                        PermissionDecision::Deny { .. } => return -1,
                        PermissionDecision::Prompt { .. } => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "Permission requires user prompt but extension runs in sync context; denying"
                            );
                            return -2;
                        }
                    }

                    // Get the callback
                    let callback = match &state.channel_callback {
                        Some(cb) => cb.clone(),
                        None => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "channel_send called but no channel callback configured"
                            );
                            return -4;
                        }
                    };

                    match callback.send_message(&channel_id, &recipient, &text) {
                        Ok(result_json) => {
                            let bytes = result_json.as_bytes();
                            let offset = caller.data().host_alloc_offset.load(Ordering::Relaxed);
                            let mem_size = memory.data_size(&caller) as u32;
                            match host_alloc(&memory, &mut caller, bytes, offset, mem_size) {
                                Ok((ptr, new_offset)) => {
                                    caller.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);
                                    encode_ptr_len(ptr, bytes.len() as u32)
                                }
                                Err(e) => {
                                    tracing::error!("Failed to write channel result to WASM memory: {}", e);
                                    -3
                                }
                            }
                        }
                        Err(e) => {
                            let ext_id = &caller.data().extension_id;
                            tracing::error!(extension = %ext_id, "channel_send failed: {}", e);
                            -3
                        }
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- config_get ---
        // Returns extension config values from storage (stored with "_config." prefix).
        // Returns packed (ptr << 32 | len) on success, -1 if not found.
        linker
            .func_wrap(
                "omni",
                "config_get",
                |mut caller: Caller<'_, SandboxState>, key_ptr: u32, key_len: u32| -> i64 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -1,
                    };
                    let key = read_string_from_memory(&memory, &caller, key_ptr, key_len);
                    let config_key = format!("_config.{}", key);

                    let storage = caller.data().storage.clone();
                    match storage.get(&config_key) {
                        Ok(Some(value)) => {
                            let bytes = value.as_bytes();
                            let offset = caller.data().host_alloc_offset.load(Ordering::Relaxed);
                            let mem_size = memory.data_size(&caller) as u32;
                            match host_alloc(&memory, &mut caller, bytes, offset, mem_size) {
                                Ok((ptr, new_offset)) => {
                                    caller.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);
                                    encode_ptr_len(ptr, bytes.len() as u32)
                                }
                                Err(_) => -1,
                            }
                        }
                        _ => -1,
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        // --- mcp_call (permission-gated) ---
        // Invokes a tool on an MCP server via the callback.
        // Returns packed (ptr << 32 | len) of JSON result on success,
        // -1 on permission denied, -2 on needs prompt, -3 on error, -4 on no callback.
        linker
            .func_wrap(
                "omni",
                "mcp_call",
                |mut caller: Caller<'_, SandboxState>,
                 server_ptr: u32,
                 server_len: u32,
                 tool_ptr: u32,
                 tool_len: u32,
                 params_ptr: u32,
                 params_len: u32|
                 -> i64 {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return -3,
                    };
                    let server_name =
                        read_string_from_memory(&memory, &caller, server_ptr, server_len);
                    let tool_name =
                        read_string_from_memory(&memory, &caller, tool_ptr, tool_len);
                    let params_json =
                        read_string_from_memory(&memory, &caller, params_ptr, params_len);

                    // Permission check
                    let state = caller.data();
                    let capability = Capability::McpServer(None);
                    let decision =
                        state.policy_engine.check_sync(&state.extension_id, &capability);

                    match decision {
                        PermissionDecision::Allow => {}
                        PermissionDecision::Deny { .. } => return -1,
                        PermissionDecision::Prompt { .. } => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "Permission requires user prompt but extension runs in sync context; denying"
                            );
                            return -2;
                        }
                    }

                    // Get the callback
                    let callback = match &state.mcp_callback {
                        Some(cb) => cb.clone(),
                        None => {
                            tracing::warn!(
                                extension = %state.extension_id,
                                "mcp_call called but no MCP callback configured"
                            );
                            return -4;
                        }
                    };

                    // Try extension-prefixed server name first (e.g., "com.example.ext:my-server")
                    // since extension-declared MCP servers are registered with this prefix.
                    // Fall back to the raw server name for globally-configured servers.
                    let prefixed_server = format!("{}:{}", state.extension_id, server_name);
                    let result = callback.call_tool(&prefixed_server, &tool_name, &params_json)
                        .or_else(|_| callback.call_tool(&server_name, &tool_name, &params_json));

                    match result {
                        Ok(result_json) => {
                            let bytes = result_json.as_bytes();
                            let offset = caller.data().host_alloc_offset.load(Ordering::Relaxed);
                            let mem_size = memory.data_size(&caller) as u32;
                            match host_alloc(&memory, &mut caller, bytes, offset, mem_size) {
                                Ok((ptr, new_offset)) => {
                                    caller.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);
                                    encode_ptr_len(ptr, bytes.len() as u32)
                                }
                                Err(e) => {
                                    tracing::error!("Failed to write MCP result to WASM memory: {}", e);
                                    -3
                                }
                            }
                        }
                        Err(e) => {
                            let ext_id = &caller.data().extension_id;
                            tracing::error!(extension = %ext_id, "mcp_call failed: {}", e);
                            -3
                        }
                    }
                },
            )
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;

        Ok(())
    }

    /// Call an extension's tool handler.
    pub fn call_tool(
        instance: &mut ExtensionInstance,
        tool_name: &str,
        params_json: &str,
        timeout: Duration,
    ) -> Result<String> {
        // Reset the host bump allocator before each call to reclaim memory
        instance.store.data().host_alloc_offset.store(0, Ordering::Relaxed);

        let func = instance
            .instance
            .get_typed_func::<(u32, u32, u32, u32), i64>(&mut instance.store, "handle_tool")
            .map_err(|_| ExtensionError::MissingEntrypoint("handle_tool".to_string()))?;

        let memory = instance
            .instance
            .get_memory(&mut instance.store, "memory")
            .ok_or(ExtensionError::NoMemory)?;

        // Write tool_name and params_json into WASM memory using host_alloc
        // to avoid fixed-offset collisions with guest heap data.
        let name_bytes = tool_name.as_bytes();
        let params_bytes = params_json.as_bytes();
        let mem_size = memory.data_size(&instance.store) as u32;

        let offset = instance.store.data().host_alloc_offset.load(Ordering::Relaxed);
        let (name_ptr, new_offset) = host_alloc(&memory, &mut instance.store, name_bytes, offset, mem_size)
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;
        let (params_ptr, new_offset) = host_alloc(&memory, &mut instance.store, params_bytes, new_offset, mem_size)
            .map_err(|e| ExtensionError::Wasm(e.to_string()))?;
        instance.store.data().host_alloc_offset.store(new_offset, Ordering::Relaxed);

        // Set a deadline using epoch interruption.
        // Use an AtomicBool so the timer thread exits early when the call completes.
        let engine = instance.store.engine().clone();
        let deadline = std::time::Instant::now() + timeout;
        let engine_clone = engine.clone();
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        let _timer = std::thread::spawn(move || {
            while std::time::Instant::now() < deadline {
                if done_clone.load(Ordering::Relaxed) {
                    return; // Call completed, exit early
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            engine_clone.increment_epoch();
        });

        // Set epoch deadline: allow 1 tick before trapping
        instance.store.set_epoch_deadline(1);
        instance.store.epoch_deadline_trap();

        let result = func
            .call(
                &mut instance.store,
                (
                    name_ptr,
                    name_bytes.len() as u32,
                    params_ptr,
                    params_bytes.len() as u32,
                ),
            )
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("epoch")
                    || msg.contains("interrupt")
                    || msg.contains("fuel")
                {
                    ExtensionError::Timeout
                } else {
                    ExtensionError::Wasm(msg)
                }
            });

        // Signal the timer thread to exit early
        done.store(true, Ordering::Relaxed);

        let result = result?;

        if result < 0 {
            return Err(ExtensionError::Wasm(format!(
                "Tool handler returned error code: {result}"
            )));
        }

        let (result_ptr, result_len) = decode_ptr_len(result);
        let result_bytes =
            read_bytes_from_memory(&memory, &instance.store, result_ptr, result_len);
        String::from_utf8(result_bytes).map_err(|_| ExtensionError::InvalidUtf8)
    }

    /// Create a SandboxState with default configuration.
    pub fn create_state(
        extension_id: &str,
        policy_engine: Arc<PolicyEngine>,
        db: Arc<Mutex<Database>>,
        max_memory_bytes: usize,
    ) -> SandboxState {
        Self::create_state_with_callbacks(extension_id, extension_id, policy_engine, db, max_memory_bytes, None, None, None)
    }

    /// Create a SandboxState with optional LLM, channel, and MCP callbacks.
    /// `instance_id` is used for storage namespace isolation (e.g., "ext::my-instance").
    /// `extension_id` is used for permission lookups (the base extension ID).
    pub fn create_state_with_callbacks(
        extension_id: &str,
        instance_id: &str,
        policy_engine: Arc<PolicyEngine>,
        db: Arc<Mutex<Database>>,
        max_memory_bytes: usize,
        llm_callback: Option<Arc<dyn LlmCallback>>,
        channel_callback: Option<Arc<dyn ChannelCallback>>,
        mcp_callback: Option<Arc<dyn McpCallback>>,
    ) -> SandboxState {
        let wasi = WasiCtxBuilder::new()
            .inherit_stderr()
            .build_p1();

        // Storage uses instance_id for namespace isolation
        let storage = Arc::new(DatabaseStorage::new(db, instance_id));

        SandboxState {
            extension_id: extension_id.to_string(),
            instance_id: instance_id.to_string(),
            wasi,
            policy_engine,
            storage,
            store_limits: StoreLimitsBuilder::new()
                .memory_size(max_memory_bytes)
                .build(),
            host_alloc_offset: AtomicU32::new(0),
            llm_callback,
            channel_callback,
            mcp_callback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let sandbox = WasmSandbox::new(&SandboxConfig::default()).unwrap();
        // Engine should be created successfully -- just verify it exists
        let _ = &sandbox.engine;
    }

    #[test]
    fn test_encode_decode_ptr_len() {
        let ptr = 1024u32;
        let len = 256u32;
        let packed = encode_ptr_len(ptr, len);
        let (decoded_ptr, decoded_len) = decode_ptr_len(packed);
        assert_eq!(decoded_ptr, ptr);
        assert_eq!(decoded_len, len);
    }

    #[test]
    fn test_instantiate_minimal_wasm() {
        let sandbox = WasmSandbox::new(&SandboxConfig::default()).unwrap();

        // Minimal WASM module with memory export
        let wasm = wat::parse_str(
            r#"
            (module
                (memory (export "memory") 1)
            )
            "#,
        )
        .unwrap();

        let db = Arc::new(Mutex::new(
            omni_core::database::Database::open(
                &std::env::temp_dir().join("test_sandbox.db"),
                "test-key",
            )
            .unwrap(),
        ));

        let policy = Arc::new(PolicyEngine::new(
            db.clone(),
            omni_permissions::policy::DefaultPolicy::Deny,
        ));

        let state = WasmSandbox::create_state("test-ext", policy, db, 1024 * 1024);
        let limits = ResourceLimits {
            max_fuel: 1_000_000,
            max_memory_bytes: 1024 * 1024,
        };

        let _instance = sandbox.instantiate(&wasm, state, &limits).unwrap();
    }

    #[test]
    fn test_fuel_exhaustion() {
        let sandbox = WasmSandbox::new(&SandboxConfig::default()).unwrap();

        // WASM module with an infinite loop
        let wasm = wat::parse_str(
            r#"
            (module
                (memory (export "memory") 1)
                (func (export "infinite_loop")
                    (loop $l
                        (br $l)
                    )
                )
            )
            "#,
        )
        .unwrap();

        let db = Arc::new(Mutex::new(
            omni_core::database::Database::open(
                &std::env::temp_dir().join("test_fuel.db"),
                "test-key",
            )
            .unwrap(),
        ));

        let policy = Arc::new(PolicyEngine::new(
            db.clone(),
            omni_permissions::policy::DefaultPolicy::Deny,
        ));

        let state = WasmSandbox::create_state("test-ext", policy, db, 1024 * 1024);
        let limits = ResourceLimits {
            max_fuel: 100, // Very low fuel
            max_memory_bytes: 1024 * 1024,
        };

        let mut instance = sandbox.instantiate(&wasm, state, &limits).unwrap();

        let func = instance
            .instance
            .get_typed_func::<(), ()>(&mut instance.store, "infinite_loop")
            .unwrap();

        let result = func.call(&mut instance.store, ());
        assert!(
            result.is_err(),
            "Should fail due to fuel exhaustion"
        );
    }
}
