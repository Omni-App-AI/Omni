# Omni Extension Examples

This directory contains example extensions for the Omni AI agent platform.
Each example demonstrates different SDK features and serves as a reference for building your own extensions.

## Prerequisites

1. **Rust toolchain** — install from [rustup.rs](https://rustup.rs)
2. **WASM target** — add the compilation target:
   ```bash
   rustup target add wasm32-wasip1
   ```
3. **(Optional) wasm-opt** — for optimizing binary size, install [binaryen](https://github.com/WebAssembly/binaryen):
   ```bash
   # macOS
   brew install binaryen
   # Windows (scoop)
   scoop install binaryen
   # Linux
   apt install binaryen
   ```

## Examples

| Example | Description | SDK Features Demonstrated |
|---------|-------------|--------------------------|
| [word-tools](word-tools/) | Text analysis & manipulation tools | Multiple tools, storage, HTTP requests, config, logging |
| [template](template/) | Minimal starter template | Basic extension structure |

## Quick Start

### 1. Build an example

**Linux / macOS:**
```bash
./build.sh word-tools
```

**Windows (PowerShell):**
```powershell
.\build.ps1 word-tools
```

**Manual (any OS):**
```bash
cd word-tools
cargo build --release --target wasm32-wasip1
cp target/wasm32-wasip1/release/omni_word_tools.wasm omni_word_tools.wasm
```

### 2. Install the extension

Copy the extension directory to Omni's extensions folder:
```bash
# The extension directory needs: omni-extension.toml + the .wasm file
cp -r word-tools ~/.omni/extensions/user/com.omni.examples.word-tools
```

Or use the CLI:
```bash
omni extension install ./word-tools
```

### 3. The extension auto-activates

Once installed, Omni discovers the extension on startup (or immediately if installed via the CLI/UI).
The extension's tools become available to the AI agent.

## Creating Your Own Extension

### Start from the template

```bash
cp -r template my-extension
cd my-extension
```

Edit these files:
1. **`omni-extension.toml`** — Set your extension ID, name, tools, and permissions
2. **`Cargo.toml`** — Set your package name
3. **`src/lib.rs`** — Implement your tool logic

### Extension directory structure

```
my-extension/
├── .cargo/
│   └── config.toml          # Sets default target to wasm32-wasi
├── src/
│   └── lib.rs                # Extension source code
├── Cargo.toml                # Rust dependencies
├── omni-extension.toml       # Extension manifest (required)
└── my_extension.wasm         # Compiled binary (after build)
```

### The manifest (`omni-extension.toml`)

Every extension needs a manifest. Here's the full schema:

```toml
# ── Required metadata ──
[extension]
id = "com.yourorg.extension-name"   # Reverse-domain format, min 5 chars
name = "My Extension"               # Display name
version = "1.0.0"                   # Semantic version (major.minor.patch)
author = "Your Name"                # Author name or email
description = "What it does"        # One-line description

# Optional metadata
license = "MIT"
homepage = "https://github.com/..."
repository = "https://github.com/..."
icon = "icon.png"                    # Relative path to icon file
categories = ["utilities"]
min_omni_version = "0.5.0"

# ── Runtime configuration ──
[runtime]
type = "wasm"                        # Only "wasm" is supported
entrypoint = "my_extension.wasm"     # Path to the compiled WASM binary
max_memory_mb = 64                   # Memory limit (default: 64)
max_cpu_ms_per_call = 5000           # Timeout per tool call in ms (default: 5000)
max_concurrent_calls = 4             # Max parallel invocations (default: 4)

# ── User configuration fields (optional) ──
[config.fields.api_key]
type = "string"
label = "API Key"
help = "Your API key for the service"
sensitive = true                     # Hidden in the UI
required = true

[config.fields.mode]
type = "enum"
label = "Operating Mode"
options = ["fast", "accurate"]
default = "fast"

# ── Permissions ──
[[permissions]]
capability = "network.http"
scope = { domains = ["api.example.com"], methods = ["GET", "POST"] }
reason = "Fetch data from the Example API."
required = true                      # true = extension won't work without it

# ── Tool definitions ──
[[tools]]
name = "my_tool"
description = "Does something useful"
[tools.parameters]
type = "object"
required = ["input"]
[tools.parameters.properties.input]
type = "string"
description = "The input data"
[tools.parameters.properties.count]
type = "integer"
minimum = 1
maximum = 100
description = "How many results to return"
```

### The Rust code

```rust
use omni_sdk::prelude::*;

#[derive(Default)]
struct MyExtension;

impl Extension for MyExtension {
    fn handle_tool(
        &mut self,
        ctx: &Context,
        tool_name: &str,
        params: serde_json::Value,
    ) -> ToolResult {
        match tool_name {
            "my_tool" => {
                let input = params["input"]
                    .as_str()
                    .ok_or_else(|| SdkError::Other("Missing 'input'".into()))?;

                ctx.info(&format!("Processing: {input}"));

                // Use storage to persist data
                let count: u32 = ctx.storage()
                    .get("call_count")?
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                ctx.storage().set("call_count", &(count + 1).to_string())?;

                Ok(serde_json::json!({
                    "result": format!("Processed: {input}"),
                    "total_calls": count + 1
                }))
            }
            _ => Err(SdkError::UnknownTool(tool_name.to_string())),
        }
    }
}

omni_sdk::omni_main!(MyExtension);
```

### Key rules

1. **Your struct must implement `Default`** — the `omni_main!` macro calls `Default::default()` on first invocation
2. **Return JSON from every tool** — success returns `Ok(serde_json::Value)`, errors return `Err(SdkError)`
3. **Handle unknown tools** — always include a `_ => Err(SdkError::UnknownTool(...))` arm
4. **Crate type must be `cdylib`** — set `crate-type = ["cdylib"]` in `Cargo.toml`
5. **Extension ID must be reverse-domain** — e.g. `com.yourorg.my-extension`

## Available Capabilities

| Capability | SDK Client | Description |
|-----------|-----------|-------------|
| `network.http` | `ctx.http()` | Make HTTP GET/POST/PUT/DELETE requests |
| `filesystem.read` | `ctx.fs()` | Read files from the host system |
| `filesystem.write` | `ctx.fs()` | Write files to the host system |
| `process.spawn` | `ctx.process()` | Execute commands on the host |
| `ai.inference` | `ctx.llm()` | Request LLM completions |
| `channel.send` | `ctx.channels()` | Send messages through connected channels |
| *(always allowed)* | `ctx.storage()` | Persistent key-value storage |
| *(always allowed)* | `ctx.config()` | Read user-set configuration values |
| *(always allowed)* | `ctx.info()` etc. | Log messages at various levels |

## Extension Lifecycle

```
Install → Register → Activate → (Tool Calls) → Deactivate → Uninstall
                         ↑                            |
                         └────── Enable/Disable ──────┘
```

1. **Install** — Extension directory copied to `~/.omni/extensions/user/`, manifest validated
2. **Register** — Manifest loaded into memory, WASM entrypoint validated
3. **Activate** — WASM loaded into wasmtime sandbox, tools become available
4. **Tool Calls** — AI agent invokes tools; each call runs in the sandbox with resource limits
5. **Deactivate** — Sandbox stopped, tools no longer available (extension stays installed)
6. **Uninstall** — Extension removed from disk and all registrations

## Troubleshooting

### "target not found: wasm32-wasip1"
```bash
rustup target add wasm32-wasip1
```

### Extension not discovered
- Verify the `omni-extension.toml` file exists in the extension root
- Check the extension ID format (must contain a dot, min 5 chars)
- Ensure version is valid SemVer (e.g., `1.0.0`, not `1.0`)

### Permission denied errors
- Declare all needed capabilities in `[[permissions]]` in the manifest
- The user must approve permissions when the extension is first activated

### WASM binary too large
- Use `opt-level = "z"` and `lto = true` in `[profile.release]`
- Run `wasm-opt -Oz` on the output binary
- Minimize dependencies — every crate increases binary size
