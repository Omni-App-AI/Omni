# Omni

Created by **Colin Vaughn** | Licensed under **GPL**

Omni is an open, extensible AI agent that lives on your desktop - designed as a security-focused competitor to OpenClaw. It connects to the LLM providers you already use, talks to people on your behalf across 25+ messaging platforms, and runs sandboxed WASM extensions that give it new skills without trusting arbitrary native code.

This is a Rust-heavy monorepo. The core engine, safety layer, extension runtime, and messaging backbone are all Rust crates. The desktop app is built with Tauri v2 and React. There's also a Next.js marketplace/website, a Supabase backend, and a handful of Node.js sidecar processes for things that don't have good Rust libraries yet.

## Repository Layout

```
omni/
├── crates/                  # Rust workspace - the core of everything
│   ├── omni-core/           # Config, database (SQLCipher), events, logging, paths
│   ├── omni-llm/            # LLM providers, agent loop, tool system, MCP client
│   ├── omni-guardian/        # Content safety - heuristics, signatures, policies, ML
│   ├── omni-extensions/      # WASM plugin runtime (wasmtime), manifest parsing, sandbox
│   ├── omni-channels/        # 25+ messaging platform integrations
│   ├── omni-permissions/     # Capability-based permission model, scoping, audit log
│   ├── omni-cli/             # CLI binary (`omni chat`, `omni config`, `omni status`)
│   ├── omni-sdk/             # Lightweight types shared with WASM extensions
│   └── omni-platform/        # Platform abstraction layer (in progress)
│
├── ui/                      # Tauri v2 desktop application
│   ├── src/                 # React 19 + TypeScript frontend
│   │   ├── components/      # Chat, channels, flowchart editor, settings, marketplace
│   │   ├── stores/          # Zustand state (chat, channels, extensions, providers, etc.)
│   │   └── lib/             # Shared utilities
│   └── src-tauri/           # Tauri Rust backend - bridges all crates into the app
│
├── website/                 # Next.js 15 marketplace and marketing site
│   └── src/
│       └── app/             # Auth, marketplace, dashboard, community, profiles, API
│
├── sidecar/                 # Node.js bridge processes
│   ├── browser/             # Puppeteer-based browser automation (stealth mode)
│   └── whatsapp/            # WhatsApp Web bridge via Baileys
│
├── supabase/                # Backend infrastructure
│   ├── migrations/          # 24 SQL migrations (profiles, extensions, reviews, etc.)
│   └── functions/           # Edge functions (security scanning, Stripe checkout)
│
├── extension-examples/      # Example extensions + SDK documentation
│   ├── word-tools/          # Full-featured example (storage, HTTP, config, logging)
│   ├── template/            # Minimal starter for new extensions
│   ├── README.md            # Extension authoring guide
│   ├── SDK-REFERENCE.md     # Full SDK reference
│   ├── build.sh             # Linux/macOS build script
│   └── build.ps1            # Windows build script
│
└── .github/workflows/       # CI/CD
    └── release.yml          # Cross-platform build + signed release pipeline
```

## Crate Architecture

The dependency graph flows downward. Higher-level crates depend on lower-level ones, never the reverse.

```
                  omni-cli
                     │
        ┌────────────┼─────────────┐
        │            │             │
    omni-llm    omni-extensions   │
        │            │             │
        ├────────────┤             │
        │            │             │
  omni-guardian  omni-permissions  │
        │            │             │
        └────────────┼─────────────┘
                     │
                 omni-core
                     │
                 omni-sdk
```

### omni-core

The foundation. Handles configuration (TOML-based, with per-agent overrides), an encrypted SQLite database (via SQLCipher) for conversations and extension data, a filesystem watcher for hot-reload, structured logging, OS keyring integration for secrets, and platform-specific path resolution. Everything else depends on this.

### omni-llm

Where the actual AI agent lives. This crate contains:

- **Providers** - OpenAI, Anthropic, Google (Gemini), Ollama, AWS Bedrock, and a generic OpenAI-compatible endpoint for self-hosted models. There's also a WebSocket-based OpenAI realtime provider.
- **Agent loop** - The core turn-based execution loop that handles tool calls, streaming responses, context management, and multi-turn conversations.
- **Tool system** - 20+ built-in tools: filesystem operations, git integration, code search, browser automation, command execution, REPL sessions, cron scheduling, memory (persistent notes), a debugger, test runner, desktop app interaction (Windows UIAutomation), notifications, image processing, clipboard access, sub-agents, and more.
- **MCP client** - Full Model Context Protocol support, so Omni can connect to external MCP servers and use their tools natively.
- **Credential rotation** - Key management with automatic rotation and OS keyring storage.
- **System prompts** - A pretty involved prompt construction system that adapts to the active tools, extensions, and agent configuration.
- **Guardian bridge** - Hooks into omni-guardian to scan both inbound and outbound content.

### omni-guardian

Content safety and trust. This isn't a toy filter; it's a layered scanning pipeline:

- **Heuristic scanner** - Pattern-matching rules for known dangerous content patterns.
- **Signature scanner** - Hash-based detection for known malicious payloads.
- **Policy engine** - Configurable rules that define what the agent is and isn't allowed to do, with severity levels and override options.
- **ML classifier** (optional, behind a feature flag) - ONNX Runtime-based model inference using HuggingFace tokenizers. Lets you run a local classifier for content moderation without any external API calls.
- **Benchmarks** - Criterion-based benchmarks for the scanning pipeline, because safety checks sit in the hot path of every agent turn.

### omni-extensions

The plugin runtime. Extensions are compiled to WebAssembly and run inside a wasmtime sandbox with strict resource limits (memory caps, CPU timeouts, concurrency limits). The system handles:

- **Manifest validation** - TOML-based manifests with SemVer versioning, capability declarations, and JSON Schema-validated tool parameter definitions.
- **Host functions** - The bridge between WASM guest code and host capabilities (HTTP, filesystem, LLM inference, storage, channels).
- **Sandbox enforcement** - Every capability must be declared in the manifest and approved by the user. The sandbox validates each call against the granted permissions before executing it.
- **Flowchart engine** - A visual programming system where users can wire together tools and extensions into automated workflows. The engine handles expression evaluation, trigger conditions, branching, and execution state.

### omni-channels

Messaging platform integrations. Each channel follows a common trait interface, so adding a new platform means implementing one Rust trait. Currently supported:

Discord, Telegram, Slack, WhatsApp (via sidecar bridge), Microsoft Teams, Matrix, IRC, Nostr, Twitter/X, Twitch, BlueBubbles (iMessage bridge), Signal, LINE, Feishu/Lark, Google Chat, Mattermost, Nextcloud Talk, Synology Chat, Urbit, Zalo, Webchat (self-hosted widget), and a generic webhook server.

The channel manager handles lifecycle, reconnection, message routing, and bridging between the inbound messages and the agent's conversation loop.

### omni-permissions

Capability-based security. Extensions and tools declare what they need (network access to specific domains, filesystem paths, process spawning), and omni-permissions enforces those boundaries at runtime. Includes an audit trail so you can review what was accessed and when.

### omni-cli

A terminal interface to the agent. Subcommands include:

- `omni chat` - Start an interactive conversation with the agent in your terminal.
- `omni config` - View and modify agent configuration.
- `omni status` - Check agent status and connected services.

### omni-sdk

A small Rust crate that defines the types and traits extensions use. Keeps WASM binary sizes down by not pulling in the full omni-core dependency tree. Extension authors depend on this crate only.

## Desktop Application

The UI is a Tauri v2 app. The frontend is React 19 with TypeScript, built by Vite, styled with Tailwind CSS v4. State management uses Zustand.

Main UI areas:

- **Chat interface** - Markdown rendering, syntax highlighting, streaming responses, diff views.
- **Flowchart editor** - Visual workflow builder using React Flow (xyflow). Drag-and-drop node composition with live execution.
- **Channel management** - Connect, configure, and monitor messaging platforms.
- **Extension browser** - Install, configure, and manage extensions. Per-extension permission controls.
- **Guardian dashboard** - View safety scan results, configure policies, review flagged content.
- **Settings** - Provider configuration, API key management, agent behavior tuning, appearance.
- **Marketplace** - Browse and install community extensions directly from the app.
- **Auto-updater** - Built-in update system with signed releases (Tauri updater plugin).

The Tauri backend (`ui/src-tauri/`) links against all the core Rust crates and exposes their functionality to the frontend through Tauri commands and events.

## Website and Marketplace

A Next.js 15 application (`website/`) that serves as both the marketing site and the extension marketplace. Built with:

- **Supabase** for auth (GitHub + Google OAuth), database, and file storage.
- **Stripe** for extension monetization.
- **Tailwind CSS v4** for styling.
- **Zod** for runtime validation.

The site includes: extension listings with reviews and download tracking, a publisher dashboard, user profiles, a community forum, blog, and API endpoints for the desktop app to check for updates and fetch marketplace data.

### Supabase Backend

The `supabase/` directory contains the full schema (24 migrations) and 8 edge functions:

- **Schema** covers: user profiles, extensions and versions, reviews, download counters, scan results, API keys, row-level security, forum threads, app releases (for the auto-updater), donations, blog posts, anti-bot protections, and moderation tools.
- **Edge functions** handle: extension security scanning (heuristic, signature, AI, and sandbox analysis), Stripe checkout sessions, and scheduled rescans.

## Sidecar Processes

Some integrations rely on Node.js libraries that don't have Rust equivalents:

- **Browser bridge** (`sidecar/browser/`) - Puppeteer with stealth plugins, Readability for article extraction, Turndown for HTML-to-Markdown conversion. The Rust tool in omni-llm spawns this process and communicates with it over stdio.
- **WhatsApp bridge** (`sidecar/whatsapp/`) - Uses the Baileys library to interface with WhatsApp Web. Handles QR code authentication and message relay.

## Getting Started

### Prerequisites

- **Rust** (stable) - [rustup.rs](https://rustup.rs)
- **Node.js** 20+ and npm
- **WASM target** (for building extensions): `rustup target add wasm32-wasip1`

### Build the Desktop App

```bash
cd ui
npm install
npm run tauri dev
```

This starts the Vite dev server and launches the Tauri window. Hot-reload works for the frontend; Rust changes require a rebuild.

### Build the Rust Crates

```bash
# Build the entire workspace
cargo build

# Run the CLI
cargo run -p omni-cli -- chat

# Run tests
cargo test
```

### Run the Website Locally

```bash
cd website
npm install
cp .env.local.example .env.local   # Fill in your Supabase + Stripe keys
npm run dev
```

### Build an Extension

```powershell
# Windows
cd extension-examples
.\build.ps1 word-tools
```

```bash
# Linux / macOS
cd extension-examples
./build.sh word-tools
```

See [extension-examples/README.md](extension-examples/README.md) for the full extension authoring guide and [extension-examples/SDK-REFERENCE.md](extension-examples/SDK-REFERENCE.md) for the SDK API reference.

## Release Pipeline

Releases are automated through GitHub Actions (`.github/workflows/release.yml`). Pushing a tag that matches `v*` triggers the pipeline:

1. **Cross-platform build** - Compiles the Tauri app for Windows (NSIS installer), macOS (DMG, both ARM64 and x86_64), and Linux (AppImage).
2. **Code signing** - All release artifacts are signed with a Tauri signing key.
3. **GitHub Release** - Artifacts are uploaded to a GitHub Release. Tags containing `-beta`, `-alpha`, or `-rc` are marked as prereleases.
4. **Release registration** - The pipeline registers the new version in Supabase so the desktop app's auto-updater can discover it.

## Tech Stack at a Glance

| Layer              | Technology                                                        |
| ------------------ | ----------------------------------------------------------------- |
| Core engine        | Rust, Tokio, SQLCipher, wasmtime                                  |
| Desktop app        | Tauri v2, React 19, Vite, Zustand, Tailwind CSS v4                |
| Website            | Next.js 15, Supabase, Stripe, Tailwind CSS v4                     |
| LLM providers      | OpenAI, Anthropic, Google, Ollama, Bedrock, custom                |
| Extension runtime  | WebAssembly (WASI), sandboxed via wasmtime                        |
| Messaging          | 25+ platforms via native Rust integrations + Node.js bridges      |
| Content safety     | Heuristic + signature scanning, policy engine, optional ONNX ML   |
| Browser automation | Puppeteer (stealth mode) via Node.js sidecar                      |
| CI/CD              | GitHub Actions, cross-platform Tauri builds                       |
| Database           | Supabase (Postgres) for the marketplace, SQLCipher for local data |

## Documentation and Community

Head over to [omniapp.org](https://omniapp.org) for full documentation, downloads, donation options, and everything else around the project.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for the full text.
