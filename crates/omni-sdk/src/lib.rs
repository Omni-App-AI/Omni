//! Omni Extension SDK
//!
//! Use this crate to build extensions for the Omni AI agent platform.
//! This SDK compiles to `wasm32-wasi` and provides ergonomic wrappers
//! around the Omni host functions.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use omni_sdk::prelude::*;
//!
//! struct MyExtension;
//!
//! impl Extension for MyExtension {
//!     fn handle_tool(
//!         &mut self,
//!         ctx: &Context,
//!         tool_name: &str,
//!         params: serde_json::Value,
//!     ) -> ToolResult {
//!         match tool_name {
//!             "hello" => {
//!                 let name = params["name"].as_str().unwrap_or("world");
//!                 Ok(serde_json::json!({ "greeting": format!("Hello, {}!", name) }))
//!             }
//!             _ => Err(SdkError::UnknownTool(tool_name.to_string())),
//!         }
//!     }
//! }
//!
//! omni_main!(MyExtension);
//! ```

pub mod ffi;

use serde::{Deserialize, Serialize};

/// Result type for tool invocations.
pub type ToolResult = std::result::Result<serde_json::Value, SdkError>;

/// Errors that can occur in the SDK.
#[derive(Debug)]
pub enum SdkError {
    /// The requested tool was not found.
    UnknownTool(String),
    /// Serialization/deserialization error.
    Serde(String),
    /// Permission was denied by the host.
    PermissionDenied(String),
    /// An HTTP request failed.
    HttpError(String),
    /// A storage operation failed.
    StorageError(String),
    /// A filesystem operation failed.
    FsError(String),
    /// A process execution failed.
    ProcessError(String),
    /// An LLM inference request failed.
    LlmError(String),
    /// A channel messaging operation failed.
    ChannelError(String),
    /// An MCP tool invocation failed.
    McpError(String),
    /// The requested capability is not available (no callback configured).
    NotAvailable(String),
    /// A generic error.
    Other(String),
}

impl std::fmt::Display for SdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTool(name) => write!(f, "Unknown tool: {name}"),
            Self::Serde(msg) => write!(f, "Serialization error: {msg}"),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {msg}"),
            Self::HttpError(msg) => write!(f, "HTTP error: {msg}"),
            Self::StorageError(msg) => write!(f, "Storage error: {msg}"),
            Self::FsError(msg) => write!(f, "Filesystem error: {msg}"),
            Self::ProcessError(msg) => write!(f, "Process error: {msg}"),
            Self::LlmError(msg) => write!(f, "LLM error: {msg}"),
            Self::ChannelError(msg) => write!(f, "Channel error: {msg}"),
            Self::McpError(msg) => write!(f, "MCP error: {msg}"),
            Self::NotAvailable(msg) => write!(f, "Not available: {msg}"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

/// Log levels matching the host function convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

// --- Helper for reading packed ptr/len from host ---

#[cfg(target_arch = "wasm32")]
fn decode_host_result(packed: i64) -> Option<(u32, u32)> {
    if packed < 0 {
        return None;
    }
    let ptr = (packed >> 32) as u32;
    let len = (packed & 0xFFFF_FFFF) as u32;
    Some((ptr, len))
}

#[cfg(target_arch = "wasm32")]
fn read_host_bytes(ptr: u32, len: u32) -> Vec<u8> {
    if len == 0 {
        return Vec::new();
    }
    unsafe {
        let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        slice.to_vec()
    }
}

#[cfg(target_arch = "wasm32")]
fn read_host_string(ptr: u32, len: u32) -> String {
    String::from_utf8_lossy(&read_host_bytes(ptr, len)).to_string()
}

/// Check a host function return code for permission errors.
#[cfg(target_arch = "wasm32")]
fn check_permission_i64(result: i64) -> Result<(), SdkError> {
    match result {
        -1 => Err(SdkError::PermissionDenied("Permission denied by policy".to_string())),
        -2 => Err(SdkError::PermissionDenied("Requires user approval".to_string())),
        _ => Ok(()),
    }
}

#[cfg(target_arch = "wasm32")]
fn check_permission_i32(result: i32) -> Result<(), SdkError> {
    match result {
        -1 => Err(SdkError::PermissionDenied("Permission denied by policy".to_string())),
        -2 => Err(SdkError::PermissionDenied("Requires user approval".to_string())),
        _ => Ok(()),
    }
}

/// The context object provided to extension handlers.
/// All host interactions go through this object.
pub struct Context {
    extension_id: String,
}

impl Context {
    /// Create a new context (called internally by the runtime).
    pub fn new(extension_id: &str) -> Self {
        Self {
            extension_id: extension_id.to_string(),
        }
    }

    /// Get the extension ID.
    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    /// Get an HTTP client for making requests (subject to permission checks).
    pub fn http(&self) -> HttpClient {
        HttpClient
    }

    /// Access persistent key-value storage.
    pub fn storage(&self) -> StorageClient {
        StorageClient
    }

    /// Access the host filesystem (subject to permission checks).
    pub fn fs(&self) -> FsClient {
        FsClient
    }

    /// Execute processes on the host system (subject to permission checks).
    pub fn process(&self) -> ProcessClient {
        ProcessClient
    }

    /// Access LLM inference (requires `ai.inference` permission).
    pub fn llm(&self) -> LlmClient {
        LlmClient
    }

    /// Send messages through connected channels (requires `channel.send` permission).
    pub fn channels(&self) -> ChannelClient {
        ChannelClient
    }

    /// Read extension configuration values set by the user.
    pub fn config(&self) -> ConfigClient {
        ConfigClient
    }

    /// Invoke tools on MCP servers (requires `mcp.server` permission).
    pub fn mcp(&self) -> McpClient {
        McpClient
    }

    /// Log a message at the given level (always allowed).
    pub fn log(&self, level: LogLevel, message: &str) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            ffi::log(level as u32, message.as_ptr() as u32, message.len() as u32);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (level, message);
        }
    }

    /// Log an error message.
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    /// Log a warning message.
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    /// Log an info message.
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    /// Log a debug message.
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }
}

/// HTTP client scoped to the extension's permissions.
pub struct HttpClient;

impl HttpClient {
    /// Send a GET request.
    pub fn get(&self, url: &str) -> Result<HttpResponse, SdkError> {
        self.request("GET", url, &[])
    }

    /// Create a POST request builder.
    pub fn post(&self, url: &str) -> RequestBuilder {
        RequestBuilder::new("POST", url)
    }

    /// Create a PUT request builder.
    pub fn put(&self, url: &str) -> RequestBuilder {
        RequestBuilder::new("PUT", url)
    }

    /// Send a DELETE request.
    pub fn delete(&self, url: &str) -> Result<HttpResponse, SdkError> {
        self.request("DELETE", url, &[])
    }

    fn request(&self, method: &str, url: &str, body: &[u8]) -> Result<HttpResponse, SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result = unsafe {
                ffi::http_request(
                    url.as_ptr() as u32,
                    url.len() as u32,
                    method.as_ptr() as u32,
                    method.len() as u32,
                    body.as_ptr() as u32,
                    body.len() as u32,
                )
            };

            // Check for permission errors
            check_permission_i64(result)?;

            if result == -3 {
                return Err(SdkError::HttpError("Request failed".to_string()));
            }

            // Decode the packed response
            let (ptr, len) = decode_host_result(result)
                .ok_or_else(|| SdkError::HttpError("Invalid host response".to_string()))?;
            let json_str = read_host_string(ptr, len);

            // Parse the JSON response: {"status": N, "body": "base64...", "body_len": N}
            let parsed: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|e| SdkError::HttpError(format!("Failed to parse response: {e}")))?;

            let status = parsed["status"].as_u64().unwrap_or(0) as u16;
            let body_b64 = parsed["body"].as_str().unwrap_or("");

            // Decode base64 body -- use a simple decoder since we can't pull in base64 crate easily in WASM
            let body_bytes = base64_decode(body_b64);

            Ok(HttpResponse {
                status,
                body: body_bytes,
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (method, url, body);
            Err(SdkError::Other(
                "HTTP requests are only available in the WASM runtime".to_string(),
            ))
        }
    }
}

/// Simple base64 decoder (standard alphabet, with padding).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn base64_decode(input: &str) -> Vec<u8> {
    const DECODE_TABLE: [u8; 128] = {
        let mut table = [255u8; 128];
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[alphabet[i] as usize] = i as u8;
            i += 1;
        }
        table
    };

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=' && b != b'\n' && b != b'\r').collect();
    let mut output = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = DECODE_TABLE.get(bytes[i] as usize).copied().unwrap_or(0) as u32;
        let b = DECODE_TABLE.get(bytes[i + 1] as usize).copied().unwrap_or(0) as u32;
        let c = DECODE_TABLE.get(bytes[i + 2] as usize).copied().unwrap_or(0) as u32;
        let d = DECODE_TABLE.get(bytes[i + 3] as usize).copied().unwrap_or(0) as u32;
        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        output.push((triple >> 16) as u8);
        output.push((triple >> 8) as u8);
        output.push(triple as u8);
        i += 4;
    }
    // Handle remaining bytes
    let remaining = bytes.len() - i;
    if remaining == 2 {
        let a = DECODE_TABLE.get(bytes[i] as usize).copied().unwrap_or(0) as u32;
        let b = DECODE_TABLE.get(bytes[i + 1] as usize).copied().unwrap_or(0) as u32;
        let triple = (a << 18) | (b << 12);
        output.push((triple >> 16) as u8);
    } else if remaining == 3 {
        let a = DECODE_TABLE.get(bytes[i] as usize).copied().unwrap_or(0) as u32;
        let b = DECODE_TABLE.get(bytes[i + 1] as usize).copied().unwrap_or(0) as u32;
        let c = DECODE_TABLE.get(bytes[i + 2] as usize).copied().unwrap_or(0) as u32;
        let triple = (a << 18) | (b << 12) | (c << 6);
        output.push((triple >> 16) as u8);
        output.push((triple >> 8) as u8);
    }
    output
}

/// An HTTP response from the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Get the response body as a string.
    pub fn text(&self) -> Result<String, SdkError> {
        String::from_utf8(self.body.clone()).map_err(|e| SdkError::Other(e.to_string()))
    }

    /// Parse the response body as JSON.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, SdkError> {
        serde_json::from_slice(&self.body).map_err(|e| SdkError::Serde(e.to_string()))
    }
}

/// Builder for HTTP requests with headers and body.
pub struct RequestBuilder {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    fn new(method: &str, url: &str) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Add a header to the request.
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }

    /// Set the request body as JSON.
    pub fn json<T: Serialize>(mut self, body: &T) -> Result<Self, SdkError> {
        self.body =
            Some(serde_json::to_vec(body).map_err(|e| SdkError::Serde(e.to_string()))?);
        self.headers.push((
            "Content-Type".to_string(),
            "application/json".to_string(),
        ));
        Ok(self)
    }

    /// Set the request body as raw bytes.
    pub fn body(mut self, data: Vec<u8>) -> Self {
        self.body = Some(data);
        self
    }

    /// Send the request.
    pub fn send(self) -> Result<HttpResponse, SdkError> {
        let client = HttpClient;
        client.request(&self.method, &self.url, self.body.as_deref().unwrap_or(&[]))
    }
}

/// Client for persistent key-value storage.
pub struct StorageClient;

impl StorageClient {
    /// Get a value from storage.
    pub fn get(&self, key: &str) -> Result<Option<String>, SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result =
                unsafe { ffi::storage_get(key.as_ptr() as u32, key.len() as u32) };

            if result == -1 {
                return Ok(None);
            }

            let ptr = (result >> 32) as u32;
            let len = (result & 0xFFFF_FFFF) as u32;

            if len == 0 {
                return Ok(Some(String::new()));
            }

            // Read the data from the pointer the host wrote
            let slice =
                unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
            let value = String::from_utf8(slice.to_vec())
                .map_err(|e| SdkError::StorageError(e.to_string()))?;
            Ok(Some(value))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            Err(SdkError::Other(
                "Storage is only available in the WASM runtime".to_string(),
            ))
        }
    }

    /// Set a value in storage.
    pub fn set(&self, key: &str, value: &str) -> Result<(), SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result = unsafe {
                ffi::storage_set(
                    key.as_ptr() as u32,
                    key.len() as u32,
                    value.as_ptr() as u32,
                    value.len() as u32,
                )
            };

            if result == 0 {
                Ok(())
            } else {
                Err(SdkError::StorageError("Failed to set value".to_string()))
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (key, value);
            Err(SdkError::Other(
                "Storage is only available in the WASM runtime".to_string(),
            ))
        }
    }

    /// Delete a value from storage.
    pub fn delete(&self, key: &str) -> Result<(), SdkError> {
        // Delete is implemented as setting to empty -- the host can handle cleanup
        self.set(key, "")
    }
}

/// Client for host filesystem operations. Requires `filesystem.read`/`filesystem.write` permission.
pub struct FsClient;

impl FsClient {
    /// Read a file from the host filesystem.
    /// Requires `filesystem.read` permission.
    pub fn read(&self, path: &str) -> Result<Vec<u8>, SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result = unsafe {
                ffi::fs_read(path.as_ptr() as u32, path.len() as u32)
            };

            check_permission_i64(result)?;

            if result == -3 {
                return Err(SdkError::FsError("Failed to read file".to_string()));
            }

            let (ptr, len) = decode_host_result(result)
                .ok_or_else(|| SdkError::FsError("Invalid host response".to_string()))?;
            Ok(read_host_bytes(ptr, len))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = path;
            Err(SdkError::Other(
                "Filesystem operations are only available in the WASM runtime".to_string(),
            ))
        }
    }

    /// Read a file as a UTF-8 string.
    /// Requires `filesystem.read` permission.
    pub fn read_string(&self, path: &str) -> Result<String, SdkError> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|e| SdkError::FsError(e.to_string()))
    }

    /// Write data to a file on the host filesystem.
    /// Requires `filesystem.write` permission.
    pub fn write(&self, path: &str, data: &[u8]) -> Result<(), SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result = unsafe {
                ffi::fs_write(
                    path.as_ptr() as u32,
                    path.len() as u32,
                    data.as_ptr() as u32,
                    data.len() as u32,
                )
            };

            check_permission_i32(result)?;

            if result == -3 {
                return Err(SdkError::FsError("Failed to write file".to_string()));
            }

            Ok(())
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (path, data);
            Err(SdkError::Other(
                "Filesystem operations are only available in the WASM runtime".to_string(),
            ))
        }
    }

    /// Write a string to a file on the host filesystem.
    /// Requires `filesystem.write` permission.
    pub fn write_string(&self, path: &str, content: &str) -> Result<(), SdkError> {
        self.write(path, content.as_bytes())
    }
}

/// Output from a process execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Client for executing processes on the host system. Requires `process.spawn` permission.
pub struct ProcessClient;

impl ProcessClient {
    /// Execute a command with arguments and wait for completion.
    /// Requires `process.spawn` permission.
    /// Arguments are passed as a slice of strings.
    pub fn exec(&self, command: &str, args: &[&str]) -> Result<ProcessOutput, SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            // Encode args as newline-separated string
            let args_str = args.join("\n");

            let result = unsafe {
                ffi::process_spawn(
                    command.as_ptr() as u32,
                    command.len() as u32,
                    args_str.as_ptr() as u32,
                    args_str.len() as u32,
                )
            };

            check_permission_i64(result)?;

            if result == -3 {
                return Err(SdkError::ProcessError("Failed to execute process".to_string()));
            }

            let (ptr, len) = decode_host_result(result)
                .ok_or_else(|| SdkError::ProcessError("Invalid host response".to_string()))?;
            let json_str = read_host_string(ptr, len);

            let output: ProcessOutput = serde_json::from_str(&json_str)
                .map_err(|e| SdkError::ProcessError(format!("Failed to parse output: {e}")))?;
            Ok(output)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (command, args);
            Err(SdkError::Other(
                "Process execution is only available in the WASM runtime".to_string(),
            ))
        }
    }
}

/// Client for LLM inference. Requires `ai.inference` permission.
pub struct LlmClient;

impl LlmClient {
    /// Send a prompt to the configured LLM and return the response text.
    /// `max_tokens` of 0 means use the provider's default.
    pub fn request(&self, prompt: &str, max_tokens: u32) -> Result<String, SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result = unsafe {
                ffi::llm_request(prompt.as_ptr() as u32, prompt.len() as u32, max_tokens)
            };

            check_permission_i64(result)?;

            if result == -4 {
                return Err(SdkError::NotAvailable(
                    "No LLM provider configured".to_string(),
                ));
            }
            if result == -3 {
                return Err(SdkError::LlmError("Request failed".to_string()));
            }

            let (ptr, len) = decode_host_result(result)
                .ok_or_else(|| SdkError::LlmError("Invalid host response".to_string()))?;
            Ok(read_host_string(ptr, len))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (prompt, max_tokens);
            Err(SdkError::Other(
                "LLM requests are only available in the WASM runtime".to_string(),
            ))
        }
    }
}

/// Client for sending messages through connected channels.
/// Requires `channel.send` permission.
pub struct ChannelClient;

impl ChannelClient {
    /// Send a text message through a connected channel.
    ///
    /// `channel_id` is the compound key (e.g. `"discord:production"`).
    /// `recipient` is the channel-specific recipient identifier.
    pub fn send(
        &self,
        channel_id: &str,
        recipient: &str,
        text: &str,
    ) -> Result<String, SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result = unsafe {
                ffi::channel_send(
                    channel_id.as_ptr() as u32,
                    channel_id.len() as u32,
                    recipient.as_ptr() as u32,
                    recipient.len() as u32,
                    text.as_ptr() as u32,
                    text.len() as u32,
                )
            };

            check_permission_i64(result)?;

            if result == -4 {
                return Err(SdkError::NotAvailable(
                    "No channel handler configured".to_string(),
                ));
            }
            if result == -3 {
                return Err(SdkError::ChannelError("Send failed".to_string()));
            }

            let (ptr, len) = decode_host_result(result)
                .ok_or_else(|| SdkError::ChannelError("Invalid host response".to_string()))?;
            Ok(read_host_string(ptr, len))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (channel_id, recipient, text);
            Err(SdkError::Other(
                "Channel operations are only available in the WASM runtime".to_string(),
            ))
        }
    }
}

/// Client for reading extension configuration values.
/// Config values are set by the user through the UI and stored with a `_config.` prefix.
pub struct ConfigClient;

impl ConfigClient {
    /// Get a configuration value by key.
    pub fn get(&self, key: &str) -> Result<Option<String>, SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let result = unsafe { ffi::config_get(key.as_ptr() as u32, key.len() as u32) };

            if result == -1 {
                return Ok(None);
            }

            let (ptr, len) = decode_host_result(result)
                .ok_or_else(|| SdkError::Other("Invalid host response".to_string()))?;

            if len == 0 {
                return Ok(Some(String::new()));
            }

            let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
            let value = String::from_utf8(slice.to_vec())
                .map_err(|e| SdkError::Other(e.to_string()))?;
            Ok(Some(value))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            Err(SdkError::Other(
                "Config is only available in the WASM runtime".to_string(),
            ))
        }
    }

    /// Get a configuration value, returning a default if not set.
    pub fn get_or(&self, key: &str, default: &str) -> Result<String, SdkError> {
        Ok(self.get(key)?.unwrap_or_else(|| default.to_string()))
    }
}

/// Client for invoking MCP server tools. Requires `mcp.server` permission.
pub struct McpClient;

impl McpClient {
    /// Call a tool on an MCP server.
    ///
    /// `server_name` is the name of the MCP server to invoke.
    /// `tool_name` is the specific tool on that server.
    /// `params` is a JSON value of tool arguments.
    ///
    /// Returns the parsed JSON response from the MCP tool.
    pub fn call(
        &self,
        server_name: &str,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        #[cfg(target_arch = "wasm32")]
        {
            let params_json = serde_json::to_string(params)
                .map_err(|e| SdkError::Serde(e.to_string()))?;

            let result = unsafe {
                ffi::mcp_call(
                    server_name.as_ptr() as u32,
                    server_name.len() as u32,
                    tool_name.as_ptr() as u32,
                    tool_name.len() as u32,
                    params_json.as_ptr() as u32,
                    params_json.len() as u32,
                )
            };

            check_permission_i64(result)?;

            if result == -4 {
                return Err(SdkError::NotAvailable(
                    "No MCP manager configured".to_string(),
                ));
            }
            if result == -3 {
                return Err(SdkError::McpError("Tool call failed".to_string()));
            }

            let (ptr, len) = decode_host_result(result)
                .ok_or_else(|| SdkError::McpError("Invalid host response".to_string()))?;
            let json_str = read_host_string(ptr, len);

            serde_json::from_str(&json_str)
                .map_err(|e| SdkError::McpError(format!("Failed to parse response: {e}")))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (server_name, tool_name, params);
            Err(SdkError::Other(
                "MCP operations are only available in the WASM runtime".to_string(),
            ))
        }
    }
}

/// Trait that all extensions must implement.
pub trait Extension {
    /// Handle a tool invocation.
    ///
    /// `tool_name` is the name of the tool being called (matching what's
    /// declared in `omni-extension.toml`). `params` contains the JSON
    /// parameters passed by the caller.
    fn handle_tool(
        &mut self,
        ctx: &Context,
        tool_name: &str,
        params: serde_json::Value,
    ) -> ToolResult;
}

/// Macro that generates the WASM `handle_tool` export function.
///
/// Usage:
/// ```rust,ignore
/// struct MyExt;
/// impl omni_sdk::Extension for MyExt {
///     fn handle_tool(&mut self, ctx: &omni_sdk::Context, tool_name: &str, params: serde_json::Value) -> omni_sdk::ToolResult {
///         Ok(serde_json::json!({}))
///     }
/// }
/// omni_sdk::omni_main!(MyExt);
/// ```
#[macro_export]
macro_rules! omni_main {
    ($ext_type:ty) => {
        static mut EXTENSION: Option<$ext_type> = None;

        #[no_mangle]
        pub extern "C" fn handle_tool(
            name_ptr: u32,
            name_len: u32,
            params_ptr: u32,
            params_len: u32,
        ) -> i64 {
            // Read tool name from memory
            let tool_name = unsafe {
                let slice =
                    std::slice::from_raw_parts(name_ptr as *const u8, name_len as usize);
                match std::str::from_utf8(slice) {
                    Ok(s) => s,
                    Err(_) => return -1,
                }
            };

            // Read params from memory
            let params_str = unsafe {
                let slice = std::slice::from_raw_parts(
                    params_ptr as *const u8,
                    params_len as usize,
                );
                match std::str::from_utf8(slice) {
                    Ok(s) => s,
                    Err(_) => return -1,
                }
            };

            let params: serde_json::Value = match serde_json::from_str(params_str) {
                Ok(v) => v,
                Err(_) => return -1,
            };

            // Initialize extension on first call
            let ext = unsafe {
                if EXTENSION.is_none() {
                    EXTENSION = Some(<$ext_type>::default());
                }
                EXTENSION.as_mut().unwrap()
            };

            let ctx = $crate::Context::new("");

            match ext.handle_tool(&ctx, tool_name, params) {
                Ok(result) => {
                    let json = match serde_json::to_string(&result) {
                        Ok(s) => s,
                        Err(_) => return -1,
                    };
                    let bytes = json.as_bytes();
                    let ptr = bytes.as_ptr() as u32;
                    let len = bytes.len() as u32;
                    // Leak the string so the host can read it
                    std::mem::forget(json);
                    ((ptr as i64) << 32) | (len as i64)
                }
                Err(e) => {
                    let error_json =
                        serde_json::json!({ "error": e.to_string() }).to_string();
                    let bytes = error_json.as_bytes();
                    let ptr = bytes.as_ptr() as u32;
                    let len = bytes.len() as u32;
                    std::mem::forget(error_json);
                    ((ptr as i64) << 32) | (len as i64)
                }
            }
        }
    };
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::{
        ChannelClient, ConfigClient, Context, Extension, FsClient, HttpClient, HttpResponse,
        LlmClient, LogLevel, McpClient, ProcessClient, ProcessOutput, RequestBuilder, SdkError,
        StorageClient, ToolResult,
    };
    pub use serde::{Deserialize, Serialize};
    pub use serde_json;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_values() {
        assert_eq!(LogLevel::Error as u32, 0);
        assert_eq!(LogLevel::Warn as u32, 1);
        assert_eq!(LogLevel::Info as u32, 2);
        assert_eq!(LogLevel::Debug as u32, 3);
    }

    #[test]
    fn test_request_builder() {
        let builder = RequestBuilder::new("POST", "https://example.com")
            .header("Authorization", "Bearer token123")
            .body(b"hello".to_vec());

        assert_eq!(builder.method, "POST");
        assert_eq!(builder.url, "https://example.com");
        assert_eq!(builder.headers.len(), 1);
        assert_eq!(builder.headers[0].0, "Authorization");
        assert_eq!(builder.body, Some(b"hello".to_vec()));
    }

    #[test]
    fn test_request_builder_json() {
        let data = serde_json::json!({"key": "value"});
        let builder = RequestBuilder::new("POST", "https://example.com")
            .json(&data)
            .unwrap();

        assert_eq!(builder.headers.len(), 1);
        assert_eq!(builder.headers[0].0, "Content-Type");
        assert_eq!(builder.headers[0].1, "application/json");
        assert!(builder.body.is_some());
    }

    #[test]
    fn test_sdk_error_display() {
        assert_eq!(
            SdkError::UnknownTool("foo".to_string()).to_string(),
            "Unknown tool: foo"
        );
        assert_eq!(
            SdkError::PermissionDenied("denied".to_string()).to_string(),
            "Permission denied: denied"
        );
        assert_eq!(
            SdkError::ProcessError("failed".to_string()).to_string(),
            "Process error: failed"
        );
    }

    #[test]
    fn test_http_response_text() {
        let resp = HttpResponse {
            status: 200,
            body: b"hello world".to_vec(),
        };
        assert_eq!(resp.text().unwrap(), "hello world");
    }

    #[test]
    fn test_http_response_json() {
        let resp = HttpResponse {
            status: 200,
            body: br#"{"key":"value"}"#.to_vec(),
        };
        let parsed: serde_json::Value = resp.json().unwrap();
        assert_eq!(parsed["key"], "value");
    }

    #[test]
    fn test_context_creation() {
        let ctx = Context::new("com.example.test");
        assert_eq!(ctx.extension_id(), "com.example.test");
    }

    #[test]
    fn test_base64_decode() {
        // "Hello, World!" => "SGVsbG8sIFdvcmxkIQ=="
        let decoded = base64_decode("SGVsbG8sIFdvcmxkIQ==");
        assert_eq!(decoded, b"Hello, World!");

        // Empty
        assert_eq!(base64_decode(""), b"");

        // Single byte "A" => "QQ=="
        let decoded = base64_decode("QQ==");
        assert_eq!(decoded, b"A");

        // Two bytes "AB" => "QUI="
        let decoded = base64_decode("QUI=");
        assert_eq!(decoded, b"AB");
    }

    #[test]
    fn test_process_output_deserialize() {
        let json = r#"{"exit_code": 0, "stdout": "hello\n", "stderr": ""}"#;
        let output: ProcessOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, "hello\n");
        assert_eq!(output.stderr, "");
    }
}
