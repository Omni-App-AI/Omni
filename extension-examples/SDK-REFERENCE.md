# Omni SDK Quick Reference

## Imports

```rust
use omni_sdk::prelude::*;
// Brings in: Extension, Context, ToolResult, SdkError,
//   HttpClient, HttpResponse, RequestBuilder,
//   StorageClient, FsClient, ProcessClient, ProcessOutput,
//   LlmClient, ChannelClient, ConfigClient, LogLevel,
//   serde::{Serialize, Deserialize}, serde_json
```

## Extension Trait

```rust
#[derive(Default)]
struct MyExt;

impl Extension for MyExt {
    fn handle_tool(
        &mut self,
        ctx: &Context,
        tool_name: &str,
        params: serde_json::Value,
    ) -> ToolResult {
        // Return Ok(json) or Err(SdkError)
    }
}

omni_sdk::omni_main!(MyExt);
```

## Context Methods

| Method | Returns | Permission |
|--------|---------|------------|
| `ctx.extension_id()` | `&str` | None |
| `ctx.http()` | `HttpClient` | `network.http` |
| `ctx.storage()` | `StorageClient` | None |
| `ctx.fs()` | `FsClient` | `filesystem.read/write` |
| `ctx.process()` | `ProcessClient` | `process.spawn` |
| `ctx.llm()` | `LlmClient` | `ai.inference` |
| `ctx.channels()` | `ChannelClient` | `channel.send` |
| `ctx.config()` | `ConfigClient` | None |
| `ctx.error(msg)` | `()` | None |
| `ctx.warn(msg)` | `()` | None |
| `ctx.info(msg)` | `()` | None |
| `ctx.debug(msg)` | `()` | None |

## HTTP Client

```rust
// GET
let resp = ctx.http().get("https://api.example.com/data")?;

// POST with JSON body
let data = serde_json::json!({"key": "value"});
let resp = ctx.http().post("https://api.example.com/data")
    .json(&data)?
    .send()?;

// PUT with raw body
let resp = ctx.http().put("https://api.example.com/data")
    .body(b"raw bytes".to_vec())
    .send()?;

// DELETE
let resp = ctx.http().delete("https://api.example.com/data/123")?;

// With custom headers
let resp = ctx.http().post("https://api.example.com/data")
    .header("Authorization", "Bearer token123")
    .header("X-Custom", "value")
    .json(&data)?
    .send()?;

// Response handling
let status: u16 = resp.status;
let text: String = resp.text()?;
let parsed: MyStruct = resp.json()?;
let raw: Vec<u8> = resp.body;
```

## Storage Client

```rust
// Store a value (persists across sessions)
ctx.storage().set("key", "value")?;

// Retrieve a value
let val: Option<String> = ctx.storage().get("key")?;

// Delete a value
ctx.storage().delete("key")?;

// Common pattern: store structured data as JSON
let data = serde_json::json!({"count": 42});
ctx.storage().set("stats", &serde_json::to_string(&data).unwrap())?;

let stats: serde_json::Value = ctx.storage().get("stats")?
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default();
```

## Filesystem Client

```rust
// Read a file as bytes
let bytes: Vec<u8> = ctx.fs().read("/path/to/file.bin")?;

// Read a file as a UTF-8 string
let text: String = ctx.fs().read_string("/path/to/file.txt")?;

// Write bytes to a file
ctx.fs().write("/path/to/output.bin", &bytes)?;

// Write a string to a file
ctx.fs().write_string("/path/to/output.txt", "Hello, world!")?;
```

## Process Client

```rust
let output: ProcessOutput = ctx.process().exec("ls", &["-la", "/tmp"])?;

println!("Exit code: {}", output.exit_code);
println!("Stdout: {}", output.stdout);
println!("Stderr: {}", output.stderr);
```

## LLM Client

```rust
// Send a prompt (max_tokens = 0 means provider default)
let response: String = ctx.llm().request("Summarize this text: ...", 0)?;

// With a specific max token count
let response: String = ctx.llm().request("Translate to French: Hello", 100)?;
```

## Channel Client

```rust
// Send a message through a connected channel
let result: String = ctx.channels().send(
    "discord:production",   // Channel compound key
    "user-id-123",          // Recipient identifier
    "Hello from extension!" // Message text
)?;
```

## Config Client

```rust
// Get a configuration value (set by user in UI)
let api_key: Option<String> = ctx.config().get("api_key")?;

// Get with a default value
let mode: String = ctx.config().get_or("mode", "fast")?;
```

## Error Types

```rust
SdkError::UnknownTool(name)       // Tool not recognized
SdkError::Serde(msg)              // JSON serialization error
SdkError::PermissionDenied(msg)   // Permission not granted
SdkError::HttpError(msg)          // HTTP request failed
SdkError::StorageError(msg)       // Storage operation failed
SdkError::FsError(msg)            // Filesystem operation failed
SdkError::ProcessError(msg)       // Process execution failed
SdkError::LlmError(msg)          // LLM request failed
SdkError::ChannelError(msg)       // Channel send failed
SdkError::NotAvailable(msg)       // Capability not configured
SdkError::Other(msg)              // Generic error
```

## Common Patterns

### Parse required parameters

```rust
let text = params["text"].as_str()
    .ok_or_else(|| SdkError::Other("Missing 'text' parameter".into()))?;

let count = params["count"].as_u64().unwrap_or(10) as usize;

let flag = params["flag"].as_bool().unwrap_or(false);
```

### Parse typed JSON into structs

```rust
#[derive(Deserialize)]
struct MyParams {
    text: String,
    #[serde(default = "default_count")]
    count: usize,
}

fn default_count() -> usize { 10 }

// In handle_tool:
let p: MyParams = serde_json::from_value(params)
    .map_err(|e| SdkError::Serde(e.to_string()))?;
```

### Return structured JSON

```rust
Ok(serde_json::json!({
    "result": "success",
    "data": {
        "items": ["a", "b", "c"],
        "count": 3
    }
}))
```

### Persistent counter

```rust
let count: u32 = ctx.storage().get("counter")?
    .and_then(|s| s.parse().ok())
    .unwrap_or(0);
let new_count = count + 1;
ctx.storage().set("counter", &new_count.to_string())?;
```

## Manifest Capabilities Reference

| Capability | Scope Fields | Description |
|-----------|-------------|-------------|
| `network.http` | `domains`, `methods` | HTTP requests |
| `network.websocket` | `domains` | WebSocket connections |
| `filesystem.read` | `paths` | Read files |
| `filesystem.write` | `paths` | Write files |
| `process.spawn` | `executables`, `allowed_args` | Execute commands |
| `ai.inference` | `max_tokens`, `rate_limit` | LLM completions |
| `channel.send` | `channels`, `rate_limit` | Send messages |
| `system.notifications` | — | System notifications |
| `system.scheduling` | — | Cron scheduling |
| `browser.scrape` | `domains`, `max_pages` | Web scraping |
| `clipboard.read` | — | Read clipboard |
| `clipboard.write` | — | Write clipboard |
| `storage.persistent` | `max_bytes` | Persistent storage |

## Build Commands

```bash
# One-time setup
rustup target add wasm32-wasip1

# Build (debug)
cargo build --target wasm32-wasip1

# Build (release, optimized)
cargo build --release --target wasm32-wasip1

# Using the build script
./build.sh my-extension          # Linux/macOS
.\build.ps1 my-extension         # Windows
```
