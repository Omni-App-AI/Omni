//! Raw FFI imports from the Omni host.
//!
//! These functions are provided by the Omni WASM sandbox runtime
//! via the `omni` import module. They are only available when
//! compiled to `wasm32-wasi`.

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "omni")]
extern "C" {
    /// Log a message to the host.
    /// level: 0=error, 1=warn, 2=info, 3=debug
    pub fn log(level: u32, msg_ptr: u32, msg_len: u32);

    /// Read a value from persistent storage.
    /// Returns packed (ptr << 32 | len) on success, -1 on failure.
    pub fn storage_get(key_ptr: u32, key_len: u32) -> i64;

    /// Write a value to persistent storage.
    /// Returns 0 on success, -1 on failure.
    pub fn storage_set(key_ptr: u32, key_len: u32, value_ptr: u32, value_len: u32) -> i32;

    /// Make an HTTP request.
    /// Returns packed (ptr << 32 | len) of JSON response on success.
    /// Returns -1 on permission denied, -2 on needs prompt, -3 on error.
    /// Response JSON format: {"status": u16, "body": "base64...", "body_len": usize}
    pub fn http_request(
        url_ptr: u32,
        url_len: u32,
        method_ptr: u32,
        method_len: u32,
        body_ptr: u32,
        body_len: u32,
    ) -> i64;

    /// Read a file from the host filesystem.
    /// Returns packed (ptr << 32 | len) of file contents on success.
    /// Returns -1 on permission denied, -2 on needs prompt, -3 on error.
    pub fn fs_read(path_ptr: u32, path_len: u32) -> i64;

    /// Write data to a file on the host filesystem.
    /// Returns 0 on success, -1 on permission denied, -2 on needs prompt, -3 on error.
    pub fn fs_write(path_ptr: u32, path_len: u32, data_ptr: u32, data_len: u32) -> i32;

    /// Spawn a process on the host system.
    /// Args are newline-separated in the args buffer.
    /// Returns packed (ptr << 32 | len) of JSON result on success.
    /// JSON format: {"exit_code": i32, "stdout": "...", "stderr": "..."}
    /// Returns -1 on permission denied, -2 on needs prompt, -3 on error.
    pub fn process_spawn(cmd_ptr: u32, cmd_len: u32, args_ptr: u32, args_len: u32) -> i64;

    /// Request LLM inference from the host.
    /// Returns packed (ptr << 32 | len) of response text on success.
    /// max_tokens: 0 means use provider default.
    /// Returns -1 on permission denied, -2 on needs prompt, -3 on error, -4 on no callback.
    pub fn llm_request(prompt_ptr: u32, prompt_len: u32, max_tokens: u32) -> i64;

    /// Send a message through a connected channel plugin.
    /// Returns packed (ptr << 32 | len) of JSON result on success.
    /// Returns -1 on permission denied, -2 on needs prompt, -3 on error, -4 on no callback.
    pub fn channel_send(
        channel_ptr: u32,
        channel_len: u32,
        recipient_ptr: u32,
        recipient_len: u32,
        text_ptr: u32,
        text_len: u32,
    ) -> i64;

    /// Read an extension configuration value.
    /// Returns packed (ptr << 32 | len) on success, -1 if not found.
    pub fn config_get(key_ptr: u32, key_len: u32) -> i64;

    /// Invoke a tool on an MCP server.
    /// Returns packed (ptr << 32 | len) of JSON result on success.
    /// Returns -1 on permission denied, -2 on needs prompt, -3 on error, -4 on no callback.
    pub fn mcp_call(
        server_ptr: u32,
        server_len: u32,
        tool_ptr: u32,
        tool_len: u32,
        params_ptr: u32,
        params_len: u32,
    ) -> i64;
}
