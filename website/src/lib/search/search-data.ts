export interface SearchEntry {
  id: number;
  pageSlug: string;
  pageTitle: string;
  category: "Getting Started" | "Core Concepts" | "Developers" | "Resources";
  section: string;
  href: string;
  content: string;
  keywords: string;
}

export const searchEntries: SearchEntry[] = [
  // ─── Overview (docs/) ───────────────────────────────────────────────
  {
    id: 0,
    pageSlug: "docs",
    pageTitle: "Documentation",
    category: "Getting Started",
    section: "Documentation",
    href: "/docs",
    content:
      "Complete Omni documentation covering AI agent setup, configuration, LLM providers, messaging channels, WASM SDK, extension publishing, security permissions, and system architecture.",
    keywords:
      "docs documentation overview home getting started guide reference",
  },

  // ─── Getting Started ────────────────────────────────────────────────
  {
    id: 1,
    pageSlug: "getting-started",
    pageTitle: "Getting Started",
    category: "Getting Started",
    section: "What is Omni?",
    href: "/docs/getting-started#introduction",
    content:
      "Omni is a privacy-first AI agent that runs locally on your desktop. It connects to LLM providers like OpenAI, Anthropic, Gemini, and Ollama, and provides capabilities through a WASM extension system with a 4-layer antivirus pipeline.",
    keywords:
      "what is omni privacy local desktop agent WASM sandbox introduction",
  },
  {
    id: 2,
    pageSlug: "getting-started",
    pageTitle: "Getting Started",
    category: "Getting Started",
    section: "Installation",
    href: "/docs/getting-started#installation",
    content:
      "Download and install Omni on Windows, macOS, or Linux. System requirements: 4GB RAM, 500MB disk, internet for cloud APIs. Available as MSI, DMG, and DEB packages.",
    keywords:
      "install download setup windows macos linux msi dmg deb system requirements curl",
  },
  {
    id: 3,
    pageSlug: "getting-started",
    pageTitle: "Getting Started",
    category: "Getting Started",
    section: "First Launch",
    href: "/docs/getting-started#first-launch",
    content:
      "On first launch, connect an LLM provider (OpenAI, Anthropic, Gemini, Ollama, Bedrock, or Custom) in Settings > Providers, then start chatting with the agent.",
    keywords:
      "first launch setup provider api key settings start chat",
  },
  {
    id: 4,
    pageSlug: "getting-started",
    pageTitle: "Getting Started",
    category: "Getting Started",
    section: "Connecting Channels",
    href: "/docs/getting-started#channels",
    content:
      "Connect Omni to 21 messaging platforms like Discord, Telegram, Slack, and WhatsApp. Channels let the agent communicate through external services.",
    keywords:
      "channels connect discord telegram slack whatsapp messaging platform setup",
  },
  {
    id: 5,
    pageSlug: "getting-started",
    pageTitle: "Getting Started",
    category: "Getting Started",
    section: "Installing Extensions",
    href: "/docs/getting-started#extensions",
    content:
      "Browse the marketplace, install extensions, review permissions, and approve. Extensions add new tools and capabilities to the agent. CLI commands: omni ext install, omni ext list, omni ext uninstall.",
    keywords:
      "extensions install marketplace browse permissions omni ext install list uninstall",
  },
  {
    id: 6,
    pageSlug: "getting-started",
    pageTitle: "Getting Started",
    category: "Getting Started",
    section: "Basic Usage",
    href: "/docs/getting-started#usage",
    content:
      "Chat naturally with the agent. Channel routing sends messages to bound extensions. Permission prompts let you allow once, always, or deny. Multiple providers with exponential backoff provide failover.",
    keywords:
      "usage chat permission prompt allow deny channel routing failover",
  },

  // ─── Configuration ──────────────────────────────────────────────────
  {
    id: 7,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "File Location",
    href: "/docs/configuration#file-location",
    content:
      "Omni reads its configuration from omni.toml. Created automatically on first launch. Linux: ~/.config/omni/omni.toml, macOS: ~/Library/Application Support/Omni/omni.toml, Windows: %APPDATA%\\Omni\\omni.toml.",
    keywords:
      "omni.toml config file location path linux macos windows appdata",
  },
  {
    id: 8,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "[general]",
    href: "/docs/configuration#general",
    content:
      "Top-level application settings: data_dir (storage path), telemetry (disabled by default), log_level (trace/debug/info/warn/error), max_history (message history limit, default 1000).",
    keywords:
      "general data_dir telemetry log_level max_history trace debug info warn error",
  },
  {
    id: 9,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "[providers]",
    href: "/docs/configuration#providers",
    content:
      "Configure LLM providers with unique keys. Fields: provider_type (required), default_model, endpoint, max_tokens, temperature, enabled. API keys stored in OS keychain.",
    keywords:
      "providers provider_type default_model endpoint max_tokens temperature enabled keychain api key",
  },
  {
    id: 10,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "[agent]",
    href: "/docs/configuration#agent",
    content:
      "Agent behavior configuration: system_prompt (custom system prompt), max_iterations (tool-use loop limit, default 25), timeout_secs (overall timeout, default 120).",
    keywords:
      "agent system_prompt max_iterations timeout_secs loop limit timeout behavior",
  },
  {
    id: 11,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "[guardian]",
    href: "/docs/configuration#guardian",
    content:
      "Guardian anti-injection pipeline config: enabled (default true), sensitivity (strict/balanced/permissive), custom_signatures (path to extra regex file), allow_override (allow suspicious results).",
    keywords:
      "guardian anti-injection pipeline sensitivity strict balanced permissive custom_signatures signatures",
  },
  {
    id: 12,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "[permissions]",
    href: "/docs/configuration#permissions",
    content:
      "Permission system defaults: default_policy (deny or prompt), trust_verified (auto-approve verified extensions), audit_enabled (log all permission decisions).",
    keywords:
      "permissions default_policy deny prompt trust_verified audit_enabled audit policy",
  },
  {
    id: 13,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "[ui]",
    href: "/docs/configuration#ui",
    content:
      "15 appearance settings: theme (light/dark/system), accent_color, font_family (Inter/JetBrains Mono/Fira Code), font_size, line_height, ui_density, sidebar_width, message_style (bubbles/flat/compact), code_theme, show_timestamps, border_radius, reduce_animations, high_contrast.",
    keywords:
      "ui appearance theme dark light font accent color sidebar message style timestamps animations accessibility",
  },
  {
    id: 14,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "[channels]",
    href: "/docs/configuration#channels",
    content:
      "Channel instances keyed by compound ID (type:instance_id) and bindings for message routing. Instance fields: channel_type, display_name, auto_connect. Bindings route messages to extensions with glob-pattern peer/group filters.",
    keywords:
      "channels instances bindings channel_type display_name auto_connect peer_filter group_filter glob routing",
  },
  {
    id: 15,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "[marketplace]",
    href: "/docs/configuration#marketplace",
    content:
      "Marketplace API settings: api_url (default: https://omniapp.org/api/v1/marketplace). Override for self-hosted marketplace instances.",
    keywords: "marketplace api_url self-hosted api endpoint",
  },
  {
    id: 16,
    pageSlug: "configuration",
    pageTitle: "Configuration",
    category: "Getting Started",
    section: "Full Example",
    href: "/docs/configuration#full-example",
    content:
      "Complete omni.toml configuration file showing all sections with typical values. Copy and customize for your setup.",
    keywords: "full example complete config toml template sample",
  },

  // ─── LLM Providers ─────────────────────────────────────────────────
  {
    id: 17,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "Provider Overview",
    href: "/docs/providers#overview",
    content:
      "Provider-agnostic LLM bridge abstraction layer. Handles streaming, token counting, and automatic failover. All 6 providers support tool calling (function calling).",
    keywords:
      "provider overview llm bridge abstraction streaming token counting failover tool calling function calling",
  },
  {
    id: 18,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "OpenAI",
    href: "/docs/providers#openai",
    content:
      "Connect to OpenAI GPT-4o and GPT-4. API key from platform.openai.com. Token counting via cl100k_base tokenizer (tiktoken-rs). Key stored securely in OS keychain.",
    keywords:
      "openai gpt-4o gpt-4 api key platform cl100k_base tiktoken keychain chatgpt",
  },
  {
    id: 19,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "Anthropic",
    href: "/docs/providers#anthropic",
    content:
      "Connect to Anthropic Claude models. API key from console.anthropic.com. Uses SSE streaming. Token counting via cl100k_base estimation.",
    keywords:
      "anthropic claude claude-opus api key console sse streaming",
  },
  {
    id: 20,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "Google Gemini",
    href: "/docs/providers#gemini",
    content:
      "Connect to Google Gemini Pro and Ultra models. API key from aistudio.google.com. Uses Gemini API v1 beta.",
    keywords:
      "google gemini gemini-pro gemini-ultra aistudio api key",
  },
  {
    id: 21,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "Ollama (Local)",
    href: "/docs/providers#ollama",
    content:
      "Run models locally with Ollama. No API key needed. Default port 11434. Supports llama3.1, mistral, phi3, and any Ollama model. Fully offline, privacy-first.",
    keywords:
      "ollama local llama3 mistral phi3 offline privacy localhost 11434 self-hosted",
  },
  {
    id: 22,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "AWS Bedrock",
    href: "/docs/providers#bedrock",
    content:
      "Access models through AWS Bedrock (Claude, Titan, Llama). Uses AWS SigV4 signing authentication. InvokeModelWithResponseStream API.",
    keywords:
      "aws bedrock amazon sigv4 signing titan llama cloud enterprise",
  },
  {
    id: 23,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "Custom HTTP",
    href: "/docs/providers#custom",
    content:
      "Connect any OpenAI-compatible API endpoint. Must support /chat/completions with SSE streaming. Works with LM Studio, vLLM, text-generation-inference, and other compatible servers.",
    keywords:
      "custom http openai compatible chat completions sse lm studio vllm tgi endpoint",
  },
  {
    id: 24,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "Provider Rotation & Fallback",
    href: "/docs/providers#rotation",
    content:
      "Multi-provider failover with exponential backoff: 5s, 15s, 60s, 300s delays. Providers tried in config order. Failed providers temporarily marked unavailable and retried after cooldown.",
    keywords:
      "rotation failover fallback exponential backoff retry priority multi-provider redundancy",
  },
  {
    id: 25,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "Streaming",
    href: "/docs/providers#streaming",
    content:
      "All providers use SSE streaming for real-time token delivery. Tokio async streams with byte buffer accumulation. Chunk types: TextDelta, ToolCallDelta, Usage, Done.",
    keywords:
      "streaming sse server-sent events tokio async textdelta toolcalldelta real-time tokens",
  },
  {
    id: 26,
    pageSlug: "providers",
    pageTitle: "LLM Providers",
    category: "Getting Started",
    section: "Token Counting",
    href: "/docs/providers#tokens",
    content:
      "Per-provider token counting: OpenAI/Anthropic use tiktoken cl100k_base tokenizer. Gemini/Ollama/Bedrock/Custom use character estimation (chars / 4).",
    keywords:
      "token counting tiktoken cl100k_base tokenizer characters estimation usage",
  },

  // ─── Channels ───────────────────────────────────────────────────────
  {
    id: 27,
    pageSlug: "channels",
    pageTitle: "Channels",
    category: "Getting Started",
    section: "Channels Overview",
    href: "/docs/channels#overview",
    content:
      "Channels are messaging platform integrations. Each runs as an independent plugin with its own connection lifecycle. Key concepts: Channel Plugin, Channel Instance, Channel Binding, Compound Key.",
    keywords:
      "channels overview plugin instance binding compound key messaging platform integration",
  },
  {
    id: 28,
    pageSlug: "channels",
    pageTitle: "Channels",
    category: "Getting Started",
    section: "Supported Channels",
    href: "/docs/channels#supported-channels",
    content:
      "21 supported channels: Discord, Telegram, WhatsApp Web, Slack, Microsoft Teams, Google Chat, Matrix, IRC, Twitch, Signal, LINE, Mattermost, Feishu, Nostr, Nextcloud Talk, Synology Chat, Twitter/X, BlueBubbles, iMessage, Zalo, WebChat. All free.",
    keywords:
      "supported channels discord telegram whatsapp slack teams google chat matrix irc twitch signal line mattermost feishu nostr nextcloud synology twitter x bluebubbles imessage zalo webchat 21",
  },
  {
    id: 29,
    pageSlug: "channels",
    pageTitle: "Channels",
    category: "Getting Started",
    section: "Connecting a Channel",
    href: "/docs/channels#connecting",
    content:
      "Connect via UI (Settings > Channels > Add Instance > credentials > Connect) or via config file. Connection lifecycle: Disconnected, Connecting, Connected, Reconnecting, Error.",
    keywords:
      "connect channel settings ui config lifecycle disconnected connecting connected reconnecting error",
  },
  {
    id: 30,
    pageSlug: "channels",
    pageTitle: "Channels",
    category: "Getting Started",
    section: "Multi-Instance Support",
    href: "/docs/channels#multi-instance",
    content:
      "Run multiple instances of the same channel type simultaneously. Compound key format: {type}:{instance_id}. Example: discord:production, discord:staging, telegram:alerts.",
    keywords:
      "multi-instance multiple compound key type instance_id production staging simultaneous",
  },
  {
    id: 31,
    pageSlug: "channels",
    pageTitle: "Channels",
    category: "Getting Started",
    section: "Message Routing & Bindings",
    href: "/docs/channels#bindings",
    content:
      "Bindings route incoming messages to extensions. Fields: channel_instance, extension_id, peer_filter (glob), group_filter (glob), priority, enabled. Resolution: Priority > Specificity > First match.",
    keywords:
      "bindings routing message peer_filter group_filter priority glob pattern resolution specificity",
  },
  {
    id: 32,
    pageSlug: "channels",
    pageTitle: "Channels",
    category: "Getting Started",
    section: "Channel Features",
    href: "/docs/channels#features",
    content:
      "6 feature types: Direct Messages, Group Messages, Media Attachments, Reactions, Read Receipts, Typing Indicators. Extensions can query available features per channel.",
    keywords:
      "features direct messages groups media attachments reactions read receipts typing indicators",
  },
  {
    id: 33,
    pageSlug: "channels",
    pageTitle: "Channels",
    category: "Getting Started",
    section: "Authentication Methods",
    href: "/docs/channels#auth-methods",
    content:
      "4 authentication types: Bot Token (Discord, Telegram, Slack), QR Code Pairing (WhatsApp Web, Signal), OAuth/API Key (Twitter, LINE), Server URL + Password (BlueBubbles, Nextcloud Talk).",
    keywords:
      "authentication auth bot token qr code pairing oauth api key server url password login",
  },
  {
    id: 34,
    pageSlug: "channels",
    pageTitle: "Channels",
    category: "Getting Started",
    section: "Channel Troubleshooting",
    href: "/docs/channels#troubleshooting",
    content:
      "Common issues: stuck on Connecting, messages not arriving, permission denied when sending, WhatsApp Web disconnects. Solutions for each.",
    keywords:
      "troubleshooting problems stuck connecting messages not arriving permission denied whatsapp disconnect fix",
  },

  // ─── Security & Permissions ─────────────────────────────────────────
  {
    id: 35,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "Security Overview",
    href: "/docs/security#overview",
    content:
      "Security as a core principle. All text scanned for prompt injection attacks. All actions gated by capability-based permissions. All extensions run in isolated WASM sandboxes with strict resource limits.",
    keywords:
      "security overview prompt injection permissions sandbox wasm isolation safety",
  },
  {
    id: 36,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "Guardian Anti-Injection Pipeline",
    href: "/docs/security#guardian",
    content:
      "4-layer weighted pipeline: Signature Scanner (30%, 79+ regex patterns), Heuristic Scanner (25%, 5 behavioral rules), ML Classifier (30%, feature-gated), Output Policy Validator (15%). Verdicts: Clean (>=80), Suspicious (50-79), Malicious (<50).",
    keywords:
      "guardian anti-injection pipeline signature scanner heuristic ml classifier output policy validator regex clean suspicious malicious",
  },
  {
    id: 37,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "Scan Points",
    href: "/docs/security#scan-points",
    content:
      "5 scan points in the agent loop: SP-1 User Input, SP-2 Prompt Assembly, SP-3 LLM Output, SP-4 Tool Calls, SP-5 Extension Output. Every piece of text entering or leaving is scanned.",
    keywords:
      "scan points sp-1 sp-2 sp-3 sp-4 sp-5 user input prompt llm output tool calls extension",
  },
  {
    id: 38,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "Sensitivity Levels",
    href: "/docs/security#sensitivity",
    content:
      "3 sensitivity levels: strict (lowest thresholds, most alerts), balanced (default), permissive (highest thresholds, fewest alerts). Configured via [guardian].sensitivity in omni.toml.",
    keywords:
      "sensitivity levels strict balanced permissive thresholds alerts detection",
  },
  {
    id: 39,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "Permission System",
    href: "/docs/security#permissions",
    content:
      "Capability-based permission system. Extensions declare permissions in manifest, users approve. Decisions: Allow, Deny, Prompt. Durations: Once, Session, Always. Default policy: deny or prompt.",
    keywords:
      "permission system capability manifest allow deny prompt once session always default policy",
  },
  {
    id: 40,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "All 26 Capabilities",
    href: "/docs/security#capabilities",
    content:
      "26 capability types: network.http, network.websocket, filesystem.read, filesystem.write, clipboard.read, clipboard.write, messaging.sms, messaging.email, messaging.chat, search.web, process.spawn, system.notifications, system.scheduling, device.camera, device.microphone, device.location, storage.persistent, browser.scrape, ai.inference, channel.send, app.automation, vcs.operations, mcp.server, code.intelligence, agent.spawn, debug.session.",
    keywords:
      "capabilities network http websocket filesystem read write clipboard messaging sms email chat search process spawn notifications scheduling camera microphone location storage browser scrape ai inference channel send app automation vcs mcp code intelligence agent spawn debug 26",
  },
  {
    id: 41,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "WASM Sandbox",
    href: "/docs/security#sandbox",
    content:
      "Wasmtime-powered isolation. Default limits: Memory 64MB, CPU 5000ms per call, Concurrency 4 simultaneous calls. Exceeded limits cause immediate termination. Semaphore-based concurrency control.",
    keywords:
      "wasm sandbox wasmtime isolation memory 64mb cpu 5000ms concurrency semaphore limits termination",
  },
  {
    id: 42,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "Audit Logging",
    href: "/docs/security#audit",
    content:
      "When audit_enabled=true, every permission decision is recorded: extension ID, capability, decision, reason, session ID, timestamp. Viewable in Settings > Permissions > Audit Log. Export to JSON/CSV.",
    keywords:
      "audit logging permission decision record export json csv settings log trail",
  },
  {
    id: 43,
    pageSlug: "security",
    pageTitle: "Security & Permissions",
    category: "Core Concepts",
    section: "Kill Switch",
    href: "/docs/security#kill-switch",
    content:
      "Instantly revokes ALL permissions for ALL extensions. Access via Settings > Permissions > Kill Switch or the kill_switch Tauri command. All extensions must re-request permissions after activation.",
    keywords:
      "kill switch revoke permissions emergency disable all extensions reset",
  },

  // ─── Native Tools ──────────────────────────────────────────────────
  {
    id: 44,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Tools Overview",
    href: "/docs/tools#overview",
    content:
      "29 native tools built into the Omni runtime. The LLM calls them during conversations. Every tool is permission-gated. Categories: 7 System, 3 Web, 3 Memory, 1 Vision, 2 Messaging, 2 Scheduling, 2 Sessions, 1 Desktop, 1 Git, 1 Testing, 1 Clipboard, 2 Code Intelligence, 1 Agent, 1 Debugging, 1 REPL. Plus MCP client for external tool servers.",
    keywords:
      "native tools overview built-in runtime llm 29 tools NativeToolRegistry permission mcp",
  },
  {
    id: 45,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "System Tools",
    href: "/docs/tools#system",
    content:
      "7 system tools: exec (shell commands), read_file (read text files), write_file (create/overwrite files), edit_file (find-and-replace), list_files (directory listing), apply_patch (unified diff), grep_search (regex search across files).",
    keywords:
      "system tools exec read_file write_file edit_file list_files apply_patch grep_search shell command file",
  },
  {
    id: 46,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Web Tools",
    href: "/docs/tools#web",
    content:
      "3 web tools: web_fetch (HTTP GET/POST/PUT/DELETE requests), web_search (web search engine queries), web_scrape (browser automation with 3 modes: extract, browser, crawl).",
    keywords:
      "web tools web_fetch web_search web_scrape http get post request browser scrape crawl",
  },
  {
    id: 47,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Memory Tools",
    href: "/docs/tools#memory",
    content:
      "3 memory tools: memory_save (store data with tags), memory_search (keyword and tag search), memory_get (retrieve by key). Persistent file-based storage for long-term agent memory.",
    keywords:
      "memory tools memory_save memory_search memory_get persistent storage tags long-term recall",
  },
  {
    id: 48,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Vision & Image Tools",
    href: "/docs/tools#vision",
    content:
      "image_analyze tool for vision analysis. Uses LLM vision capabilities to describe, analyze, and extract information from images.",
    keywords:
      "vision image analyze image_analyze picture screenshot describe ocr",
  },
  {
    id: 49,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Messaging Tools",
    href: "/docs/tools#messaging",
    content:
      "2 messaging tools: send_message (send through channel bindings, checks permissions) and list_channels (list all connected channel instances with status).",
    keywords:
      "messaging tools send_message list_channels channel bindings send message chat",
  },
  {
    id: 50,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Notifications & Scheduling",
    href: "/docs/tools#notifications-scheduling",
    content:
      "notify tool for desktop notifications. cron_schedule tool for scheduling recurring tasks with cron expressions. Both return structured JSON for higher-level components.",
    keywords:
      "notify notification cron_schedule cron scheduling timer recurring task desktop alert",
  },
  {
    id: 51,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Session Tools",
    href: "/docs/tools#sessions",
    content:
      "session_list (list past sessions with metadata) and session_history (full message history for a session). Both require database access for persistent session storage.",
    keywords:
      "session tools session_list session_history messages history database past conversations",
  },
  {
    id: 52,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Desktop Automation (app_interact)",
    href: "/docs/tools#desktop-automation",
    content:
      "app_interact tool for Windows UI Automation. 11 actions: launch, list_windows, find_element, find_elements, click, type_text, read_text, get_tree, get_subtree, screenshot, close. LOLBIN blocklist, password field protection, rate limiting.",
    keywords:
      "desktop automation app_interact ui automation windows launch click type read screenshot lolbin blocklist",
  },
  {
    id: 53,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Web Scrape Modes",
    href: "/docs/tools#web-scrape-modes",
    content:
      "3 scraping modes: extract (fast HTML parsing, scraper crate), browser (Puppeteer stealth with Readability + Turndown), crawl (BFS multi-page, max 100 pages, depth 5). Content limits: 500KB/page, 5MB total.",
    keywords:
      "web scrape modes extract browser crawl puppeteer stealth readability turndown bfs html parsing",
  },
  {
    id: 54,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "App Interact Actions",
    href: "/docs/tools#app-interact-actions",
    content:
      "11 desktop automation actions: launch (start app), list_windows (enumerate windows), find_element/find_elements (locate UI elements), click, type_text, read_text, get_tree/get_subtree (UI tree), screenshot (capture window), close.",
    keywords:
      "app interact actions launch list_windows find_element click type_text read_text get_tree screenshot close element_ref",
  },
  {
    id: 55,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "App Interact Security",
    href: "/docs/tools#app-interact-security",
    content:
      "12 defense-in-depth layers: Permission Gating, LOLBIN Blocklist (43 executables), Executable Allowlist, Password Field Hard-Block, Sensitive Name Guard, Rate Limiting (60/min), Max Concurrent Processes (3), Tree Depth Cap, Value Redaction, Semantic Actions Only, Guardian Scanning, Audit Events.",
    keywords:
      "app interact security lolbin blocklist password protection rate limiting defense depth audit redaction allowlist",
  },

  {
    id: 96,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Git Tool",
    href: "/docs/tools#version-control",
    content:
      "git tool with 10 actions: status, diff, log, commit, branch, checkout, stash, merge, show_conflict, resolve. Returns structured JSON. Automatic secret scanning before commits blocks API keys, tokens, and passwords. Conflict marker parsing into ours/theirs sections.",
    keywords:
      "git version control commit branch merge diff status checkout stash secret scanning conflict resolve vcs",
  },
  {
    id: 97,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Test Runner Tool",
    href: "/docs/tools#testing",
    content:
      "test_runner tool with 3 actions: run (execute tests), list (discover tests), coverage (run with coverage). Auto-detects frameworks: cargo test (Rust), jest/vitest/mocha (JS/TS), pytest (Python), go test (Go), dotnet test (.NET). Returns structured pass/fail counts.",
    keywords:
      "test runner testing run list coverage cargo jest vitest mocha pytest go test dotnet framework auto-detect",
  },
  {
    id: 98,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Clipboard Tool",
    href: "/docs/tools#clipboard",
    content:
      "clipboard tool with 2 actions: read (get clipboard text) and write (set clipboard text). Uses the arboard crate for cross-platform clipboard access. Maximum content size 1 MB.",
    keywords:
      "clipboard read write copy paste system clipboard arboard text",
  },
  {
    id: 99,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Code Search Tool",
    href: "/docs/tools#code-intelligence",
    content:
      "code_search tool for offline code intelligence. 4 actions: index (build symbol index), search (query symbols), symbols (list file symbols), dependencies (show imports). Supports 9 languages: Rust, TypeScript, JavaScript, Python, Go, C, C++, Java, C#. Works without a language server.",
    keywords:
      "code search code_search index symbols dependencies imports offline syntax analysis rust typescript python go java csharp",
  },
  {
    id: 100,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "LSP Tool",
    href: "/docs/tools#code-intelligence",
    content:
      "lsp tool for real-time code intelligence via Language Server Protocol. 8 actions: start, stop, goto_definition, find_references, hover, diagnostics, symbols, rename_preview. Auto-detects: rust-analyzer, typescript-language-server, pyright, gopls.",
    keywords:
      "lsp language server protocol goto definition find references hover diagnostics symbols rename rust-analyzer pyright gopls typescript",
  },
  {
    id: 101,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Agent Spawn Tool",
    href: "/docs/tools#agent-orchestration",
    content:
      "agent_spawn tool to run sub-agents in parallel. Each sub-agent gets its own conversation context and tool access (except agent_spawn to prevent recursion). Set wait=true to block or wait=false for async task IDs. Max 15 iterations per sub-agent.",
    keywords:
      "agent spawn sub-agent parallel task concurrent delegation orchestration wait async sub agent",
  },
  {
    id: 102,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Debugger Tool",
    href: "/docs/tools#debugging",
    content:
      "debugger tool implementing the Debug Adapter Protocol (DAP). 11 actions: launch, attach, set_breakpoints, continue, step_over, step_into, step_out, evaluate, variables, stack_trace, disconnect. Auto-detects adapters: codelldb (Rust), debugpy (Python), node-debug (JS), dlv-dap (Go).",
    keywords:
      "debugger debug dap breakpoint step over into out evaluate variables stack trace launch attach codelldb debugpy",
  },
  {
    id: 103,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "REPL Tool",
    href: "/docs/tools#interactive-execution",
    content:
      "repl tool for persistent interactive code execution sessions. 4 actions: execute, list, reset, close. Supports Python and Node.js interpreters. Up to 3 concurrent sessions with 30-second execution timeout. State persists between executions within a session.",
    keywords:
      "repl interactive execution python node javascript session persistent interpreter execute code run",
  },
  {
    id: 104,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "MCP Client",
    href: "/docs/tools#mcp-client",
    content:
      "Built-in MCP (Model Context Protocol) client connects to external tool servers via stdio JSON-RPC. Tools are auto-discovered and namespaced as mcp_<server>_<tool>. No HTTP server — fully local. Guardian SP-6 scans all MCP output. Configure in [mcp.servers] with auto_start.",
    keywords:
      "mcp model context protocol client server stdio json-rpc tool discovery namespaced external tools sp-6 guardian scan auto_start",
  },
  {
    id: 105,
    pageSlug: "tools",
    pageTitle: "Native Tools",
    category: "Core Concepts",
    section: "Git Secret Scanning",
    href: "/docs/tools#git-actions",
    content:
      "The git commit action scans staged content for secrets: API keys (sk_live, AKIA), tokens, passwords in config files, private keys, and connection strings. Blocked commits include details about what was detected and where.",
    keywords:
      "git secret scanning api key token password commit block prevent credential leak security",
  },

  // ─── Flowchart Builder ─────────────────────────────────────────────
  {
    id: 106,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Flowchart Overview",
    href: "/docs/flowcharts#overview",
    content:
      "Build AI workflows visually with the drag-and-drop Flowchart Builder. 19 node types across 4 categories, expression evaluator, auto-triggers, sub-flows, and full access to all 29 native tools — no code required.",
    keywords:
      "flowchart builder overview visual workflow drag drop no-code automation 19 node types",
  },
  {
    id: 107,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Node Types",
    href: "/docs/flowcharts#node-types",
    content:
      "19 node types in 4 categories. Control Flow: Start, End, Condition, Switch, Loop, Delay. Actions: LLM Call, Tool Call, HTTP Request, Channel Send, Code, Sub-Flow, Parallel. Data: Set Variable, Transform, Merge. Utility: Note, Log, Error.",
    keywords:
      "node types start end condition switch loop delay llm call tool call http request channel send code sub-flow parallel set variable transform merge note log error 19",
  },
  {
    id: 108,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Expression System",
    href: "/docs/flowcharts#expressions",
    content:
      "Expressions evaluate dynamic values using 3 types: JSONPath ($.variable.field), templates (Hello {{name}}), and conditions ($.count > 10). 18 operators including arithmetic, comparison, logical, and string operations.",
    keywords:
      "expression system jsonpath template condition variable dynamic evaluate operators arithmetic comparison logical string interpolation",
  },
  {
    id: 109,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Visual Editor",
    href: "/docs/flowcharts#editor",
    content:
      "React Flow drag-and-drop canvas for building flowcharts. Features: node palette (drag to add), visual connections, expression editor with syntax highlighting, real-time validation, undo/redo history, zoom and pan, JSON import/export.",
    keywords:
      "visual editor react flow canvas drag drop palette connections undo redo zoom pan json import export ui",
  },
  {
    id: 110,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Execution Engine",
    href: "/docs/flowcharts#engine",
    content:
      "Async Rust engine (FlowchartEngine) executes flowcharts with safety limits: 500 max node executions, 30s global timeout, 10 sub-flow depth, 100 loop iterations, 1MB variable size, 50 max variables.",
    keywords:
      "execution engine FlowchartEngine async rust safety limits timeout max nodes loop iterations sub-flow depth variables",
  },
  {
    id: 111,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Auto-Triggers",
    href: "/docs/flowcharts#triggers",
    content:
      "3 trigger types to start flowcharts automatically: Event triggers (message_received, extension_activated, etc.), Schedule triggers (cron expressions), Webhook triggers (HTTP POST to local endpoint).",
    keywords:
      "auto-triggers trigger event schedule cron webhook http post automatic start message_received extension_activated",
  },
  {
    id: 112,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Flowchart Permissions",
    href: "/docs/flowcharts#permissions",
    content:
      "7 node types require specific capabilities: Tool Call (varies by tool), HTTP Request (network.http), Channel Send (channel.send), Code (process.spawn), LLM Call (ai.inference), Sub-Flow (agent.spawn), Parallel (agent.spawn).",
    keywords:
      "flowchart permissions capabilities tool call http request channel send code llm sub-flow parallel network filesystem",
  },
  {
    id: 113,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Definition Format",
    href: "/docs/flowcharts#definition",
    content:
      "Flowcharts stored as JSON with 9 top-level keys: id, name, description, version, nodes (array), edges (array), variables, triggers, metadata. Nodes contain id, node_type, position, config, and label.",
    keywords:
      "definition format json schema nodes edges variables triggers metadata config position label flowchart file structure",
  },
  {
    id: 114,
    pageSlug: "flowcharts",
    pageTitle: "Flowchart Builder",
    category: "Core Concepts",
    section: "Flowchart Testing & Debugging",
    href: "/docs/flowcharts#testing",
    content:
      "Test flowcharts step by step: run with sample input, watch execution path highlight, inspect variables at each node, check error nodes for failures, use Log nodes for debugging, export and version control JSON definitions.",
    keywords:
      "flowchart testing debugging step sample input execution path variables inspect error log export version control",
  },

  // ─── Hook System ───────────────────────────────────────────────────
  {
    id: 56,
    pageSlug: "hooks",
    pageTitle: "Hook System",
    category: "Core Concepts",
    section: "Hooks Overview",
    href: "/docs/hooks#overview",
    content:
      "The hook system intercepts data at key points in the agent loop. Two types: Modifying hooks run sequentially and can transform or block data. Notification hooks run in parallel for observability.",
    keywords:
      "hooks overview intercept agent loop modifying notification transform block observability",
  },
  {
    id: 57,
    pageSlug: "hooks",
    pageTitle: "Hook System",
    category: "Core Concepts",
    section: "Hook Points",
    href: "/docs/hooks#hook-points",
    content:
      "7 hook points: 5 Modifying (MessageReceived, LlmInput, LlmOutput, BeforeToolCall, AfterToolCall) and 2 Notification (SessionStart, SessionEnd).",
    keywords:
      "hook points MessageReceived LlmInput LlmOutput BeforeToolCall AfterToolCall SessionStart SessionEnd 7",
  },
  {
    id: 58,
    pageSlug: "hooks",
    pageTitle: "Hook System",
    category: "Core Concepts",
    section: "Modifying Hooks",
    href: "/docs/hooks#modifying-hooks",
    content:
      "Run sequentially in priority order (lowest first). Each hook receives the previous hook's output. Returns Continue (with optional modified data) or Block (stops the pipeline). BeforeToolCall block produces HookBlocked error.",
    keywords:
      "modifying hooks sequential priority continue block pipeline HookBlocked transform data",
  },
  {
    id: 59,
    pageSlug: "hooks",
    pageTitle: "Hook System",
    category: "Core Concepts",
    section: "Notification Hooks",
    href: "/docs/hooks#notification-hooks",
    content:
      "Run in parallel via tokio::join!. Read-only context. Errors are logged but don't affect the pipeline. Use cases: analytics, external logging, webhooks, dashboard updates.",
    keywords:
      "notification hooks parallel tokio join read-only analytics logging webhooks dashboard",
  },
  {
    id: 60,
    pageSlug: "hooks",
    pageTitle: "Hook System",
    category: "Core Concepts",
    section: "Hook Context",
    href: "/docs/hooks#hook-context",
    content:
      "HookContext fields: hook_point (HookPoint enum), session_id, text, tool_call (ToolCallInfo), messages (Vec<ChatMessage>), metadata (serde_json::Value).",
    keywords:
      "hook context HookContext HookPoint session_id text tool_call ToolCallInfo ChatMessage metadata",
  },
  {
    id: 61,
    pageSlug: "hooks",
    pageTitle: "Hook System",
    category: "Core Concepts",
    section: "Hook Results",
    href: "/docs/hooks#hook-results",
    content:
      "Two possible results: Continue(HookContext) to proceed with optionally modified data, or Block { reason: String } to stop the pipeline and log the reason.",
    keywords:
      "hook results HookResult continue block reason stop pipeline",
  },
  {
    id: 62,
    pageSlug: "hooks",
    pageTitle: "Hook System",
    category: "Core Concepts",
    section: "Hook Registration",
    href: "/docs/hooks#registration",
    content:
      "Register hooks with HookRegistry shared via Arc<HookRegistry>. Methods: register_modifying(HookPoint, priority, handler) and register_notification(HookPoint, handler). Handlers implement the HookHandler trait.",
    keywords:
      "registration HookRegistry Arc register_modifying register_notification HookHandler trait priority",
  },
  {
    id: 63,
    pageSlug: "hooks",
    pageTitle: "Hook System",
    category: "Core Concepts",
    section: "Hook Examples",
    href: "/docs/hooks#examples",
    content:
      "3 Rust code examples: Block dangerous exec tool calls (rm -rf/format/del), Log all LLM requests for debugging, Filter profanity from incoming messages.",
    keywords:
      "hook examples block dangerous exec rm -rf log llm requests filter profanity code rust",
  },

  // ─── Architecture ──────────────────────────────────────────────────
  {
    id: 64,
    pageSlug: "architecture",
    pageTitle: "Architecture",
    category: "Core Concepts",
    section: "Crate Map",
    href: "/docs/architecture#crates",
    content:
      "8 Rust crates: omni-core (config, DB, event bus), omni-permissions (26 capabilities, PolicyEngine), omni-guardian (4-layer scanning), omni-extensions (WASM sandbox, Wasmtime), omni-sdk (developer SDK), omni-llm (6 providers, agent loop, 29 native tools), omni-channels (21 platforms), ui/src-tauri (Tauri v2 shell).",
    keywords:
      "crate map architecture omni-core omni-permissions omni-guardian omni-extensions omni-sdk omni-llm omni-channels tauri rust crates workspace",
  },
  {
    id: 65,
    pageSlug: "architecture",
    pageTitle: "Architecture",
    category: "Core Concepts",
    section: "Agent Loop",
    href: "/docs/architecture#agent-loop",
    content:
      "Core orchestration engine. 7-step per-turn flow: receive message, Guardian scans, hooks, assemble prompt, stream response, parse tool calls, execute tools, loop or return. Native tools checked first, then extension tools.",
    keywords:
      "agent loop orchestration flow receive message guardian scan hooks prompt stream tool calls execute native extension",
  },
  {
    id: 66,
    pageSlug: "architecture",
    pageTitle: "Architecture",
    category: "Core Concepts",
    section: "Event Bus",
    href: "/docs/architecture#event-bus",
    content:
      "Broadcast channel via tokio::sync::broadcast for lock-free multi-consumer event delivery. 9 event types including MessageReceived, PermissionPrompt, GuardianAlert, and Extension/Channel lifecycle events. Bridged to Tauri frontend.",
    keywords:
      "event bus broadcast tokio sync channel lock-free events MessageReceived PermissionPrompt GuardianAlert tauri bridge",
  },
  {
    id: 67,
    pageSlug: "architecture",
    pageTitle: "Architecture",
    category: "Core Concepts",
    section: "Database",
    href: "/docs/architecture#database",
    content:
      "SQLite with SQLCipher encryption. Accessed via Arc<Mutex<Database>> with spawn_blocking. Encryption key from OS keychain, env var, key file, or auto-generated. 8 tables: sessions, messages, permissions, audit_logs, extensions, extension_storage, channel_instances, channel_bindings.",
    keywords:
      "database sqlite sqlcipher encryption arc mutex spawn_blocking keychain tables sessions messages permissions audit extensions",
  },
  {
    id: 68,
    pageSlug: "architecture",
    pageTitle: "Architecture",
    category: "Core Concepts",
    section: "Data Flow",
    href: "/docs/architecture#data-flow",
    content:
      "End-to-end data flow: User Input > Tauri IPC > Guardian > Hook > Agent Loop > LLM Bridge > Provider > SSE Stream > Guardian > Tool Call > Permission Check > Tool Execution > Guardian > Response > Tauri Event > React UI.",
    keywords:
      "data flow end-to-end tauri ipc guardian hook agent loop llm bridge provider sse stream tool permission response react ui",
  },
  {
    id: 69,
    pageSlug: "architecture",
    pageTitle: "Architecture",
    category: "Core Concepts",
    section: "Extension Lifecycle",
    href: "/docs/architecture#extension-lifecycle",
    content:
      "7 phases: Install (extract .omni, validate manifest, reject symlinks), Register (DB + in-memory, SemVer), Enable (mark enabled, auto-activate), Activate (Wasmtime sandbox, host functions), Invoke (handle_tool, CPU timeout), Deactivate (tear down), Uninstall (delete, revoke permissions).",
    keywords:
      "extension lifecycle install register enable activate invoke deactivate uninstall .omni manifest semver wasmtime handle_tool",
  },

  // ─── SDK Reference ─────────────────────────────────────────────────
  {
    id: 70,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "SDK Overview",
    href: "/docs/sdk#overview",
    content:
      "Build extensions in Rust, compiled to WebAssembly (wasm32-wasip1), running in Wasmtime sandboxes. The omni-sdk crate provides typed clients, the omni_main! macro, and re-exports serde/serde_json.",
    keywords:
      "sdk overview rust wasm webassembly wasm32-wasip1 wasmtime omni-sdk crate typed clients",
  },
  {
    id: 71,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Quick Start",
    href: "/docs/sdk#quickstart",
    content:
      "Create a new extension: cargo new --lib, cargo add omni-sdk, set crate-type to cdylib, implement Extension trait with handle_tool, use omni_main! macro, build with cargo build --target wasm32-wasip1 --release.",
    keywords:
      "quick start quickstart cargo new lib cdylib extension trait handle_tool omni_main macro build wasm32-wasip1 release",
  },
  {
    id: 72,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Manifest Format",
    href: "/docs/sdk#manifest",
    content:
      "omni-extension.toml with sections: [extension] (id, name, version, author), [runtime] (entrypoint, limits), [[permissions]] (capability, scope, reason), [[tools]] (name, description, JSON Schema parameters), [config], [hooks].",
    keywords:
      "manifest omni-extension.toml extension id name version author runtime entrypoint permissions tools json schema config hooks",
  },
  {
    id: 73,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Extension Trait & Entry Point",
    href: "/docs/sdk#extension-trait",
    content:
      "Implement the Extension trait with handle_tool(&mut self, ctx: &Context, tool_name: &str, params: Value) -> ToolResult. Use omni_main! macro to generate the WASM entry point. Struct must implement Default.",
    keywords:
      "extension trait handle_tool context tool_name params value toolresult omni_main entry point default impl",
  },
  {
    id: 74,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Context API",
    href: "/docs/sdk#context",
    content:
      "Context provides typed clients: ctx.http() (HttpClient), ctx.fs() (FsClient), ctx.process() (ProcessClient), ctx.storage() (StorageClient), ctx.llm() (LlmClient), ctx.channels() (ChannelClient), ctx.config() (ConfigClient), ctx.log/info/warn/error/debug.",
    keywords:
      "context api ctx http fs process storage llm channels config log HttpClient FsClient ProcessClient StorageClient LlmClient ChannelClient ConfigClient",
  },
  {
    id: 75,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Host Functions",
    href: "/docs/sdk#host-functions",
    content:
      "10 host functions under the 'omni' import module: log, storage_get, storage_set, http_request (30s timeout, 5MB limit), fs_read (10MB limit), fs_write, process_spawn (50KB output), llm_request, channel_send, config_get. Return codes: -1 denied, -2 prompt, -3 failed, -4 no callback.",
    keywords:
      "host functions omni import log storage http_request fs_read fs_write process_spawn llm_request channel_send config_get return codes",
  },
  {
    id: 76,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "SDK Permissions",
    href: "/docs/sdk#permissions",
    content:
      "Deny-by-default capability system. 20 available capabilities with scopes. Scope examples: network.http (domains, methods), filesystem.read (paths, extensions, max_size), process.spawn (executables, denied_args regex).",
    keywords:
      "sdk permissions deny-by-default capability scope network filesystem process domains methods paths executables denied_args regex",
  },
  {
    id: 77,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Error Handling",
    href: "/docs/sdk#errors",
    content:
      "SdkError enum: UnknownTool, Serde, PermissionDenied, HttpError, StorageError, FsError, ProcessError, LlmError, ChannelError, NotAvailable, Other. ToolResult is Result<serde_json::Value, SdkError>.",
    keywords:
      "error handling SdkError UnknownTool PermissionDenied HttpError StorageError FsError ProcessError LlmError ChannelError ToolResult",
  },
  {
    id: 78,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Building & Testing Extensions",
    href: "/docs/sdk#building",
    content:
      "Build with rustup target add wasm32-wasip1. Debug and release builds. Optimize with wasm-opt. Test locally by placing files in the extensions folder. Sandbox limits: Memory 64MB, CPU 5000ms, Concurrency 4.",
    keywords:
      "building testing wasm32-wasip1 wasm-opt debug release extensions folder sandbox limits local test",
  },
  {
    id: 79,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Prelude",
    href: "/docs/sdk#prelude",
    content:
      "use omni_sdk::prelude::* re-exports 18 types: Context, Extension, ToolResult, SdkError, LogLevel, HttpClient, HttpResponse, RequestBuilder, FsClient, ProcessClient, ProcessOutput, StorageClient, LlmClient, ChannelClient, ConfigClient, Serialize, Deserialize, serde_json.",
    keywords:
      "prelude use omni_sdk re-exports context extension toolresult sdkerror clients serialize deserialize serde_json",
  },
  {
    id: 80,
    pageSlug: "sdk",
    pageTitle: "SDK Reference",
    category: "Developers",
    section: "Complete Example",
    href: "/docs/sdk#examples",
    content:
      "Full weather extension example: manifest with permissions and tool definitions, Rust source with config API key, storage caching, HTTP fetch from OpenWeatherMap, JSON response parsing.",
    keywords:
      "example weather extension manifest permissions tools rust config api key storage cache http fetch openweathermap json",
  },

  // ─── Publishing Guide ──────────────────────────────────────────────
  {
    id: 81,
    pageSlug: "publishing",
    pageTitle: "Publishing Guide",
    category: "Developers",
    section: "Publishing Overview",
    href: "/docs/publishing#overview",
    content:
      "5-step publishing process: Build your extension, create an API key, publish via CLI, pass security scan, go live on the marketplace.",
    keywords:
      "publishing overview publish marketplace extension process steps go live",
  },
  {
    id: 82,
    pageSlug: "publishing",
    pageTitle: "Publishing Guide",
    category: "Developers",
    section: "Prerequisites",
    href: "/docs/publishing#prerequisites",
    content:
      "Requirements: developer account, built WASM extension with valid manifest, Omni CLI (cargo install omni-cli), and an API key.",
    keywords:
      "prerequisites requirements developer account wasm manifest omni-cli cargo install api key",
  },
  {
    id: 83,
    pageSlug: "publishing",
    pageTitle: "Publishing Guide",
    category: "Developers",
    section: "Creating an API Key",
    href: "/docs/publishing#api-keys",
    content:
      "Generate API keys from Dashboard > API Keys. 72-character format omni_pk_... Store as OMNI_API_KEY env var. Keys are revocable and stored as SHA-256 hashes.",
    keywords:
      "api key create generate dashboard omni_pk OMNI_API_KEY env var sha-256 hash revoke",
  },
  {
    id: 84,
    pageSlug: "publishing",
    pageTitle: "Publishing Guide",
    category: "Developers",
    section: "Publishing via CLI",
    href: "/docs/publishing#cli-publish",
    content:
      "Run omni ext publish --api-key $OMNI_API_KEY. CLI reads manifest, uploads WASM binary, verifies SHA-256 checksum, creates marketplace entry, triggers security scan. Supports --changelog flag.",
    keywords:
      "cli publish omni ext publish api-key upload wasm checksum sha-256 changelog command",
  },
  {
    id: 85,
    pageSlug: "publishing",
    pageTitle: "Publishing Guide",
    category: "Developers",
    section: "The Security Scan",
    href: "/docs/publishing#security-scan",
    content:
      "4-layer scan pipeline (30-60 seconds): Signature Scanning (30%), Heuristic Analysis (25%), AI Code Review by Claude (30%), Sandbox Testing (15%). Each layer produces a weighted score.",
    keywords:
      "security scan pipeline signature heuristic ai code review claude sandbox testing score layer",
  },
  {
    id: 86,
    pageSlug: "publishing",
    pageTitle: "Publishing Guide",
    category: "Developers",
    section: "Scan Verdicts",
    href: "/docs/publishing#verdicts",
    content:
      "Clean (score >=80, no layer below 60) = auto-approved. Suspicious (50-79) = manual review in 1-3 business days. Malicious (<50 or critical flags) = auto-rejected.",
    keywords:
      "verdicts clean suspicious malicious auto-approved manual review rejected score threshold",
  },
  {
    id: 87,
    pageSlug: "publishing",
    pageTitle: "Publishing Guide",
    category: "Developers",
    section: "Versioning",
    href: "/docs/publishing#versioning",
    content:
      "Semantic versioning (semver). New version must be higher than currently published. Patch for bug fixes, Minor for new features, Major for breaking changes.",
    keywords:
      "versioning semver semantic version patch minor major breaking changes bump",
  },
  {
    id: 88,
    pageSlug: "publishing",
    pageTitle: "Publishing Guide",
    category: "Developers",
    section: "Best Practices",
    href: "/docs/publishing#best-practices",
    content:
      "5 best practices: minimize permissions, write clear descriptions, include source code (repository field), write meaningful changelogs, respond to user reviews.",
    keywords:
      "best practices minimize permissions description source code repository changelog reviews verified",
  },

  // ─── Building from Source ──────────────────────────────────────────
  {
    id: 89,
    pageSlug: "building",
    pageTitle: "Building from Source",
    category: "Developers",
    section: "Build Prerequisites",
    href: "/docs/building#prerequisites",
    content:
      "5 prerequisites: Rust toolchain 1.78+, wasm32-wasi target, Node.js 20+ (for Baileys sidecar), SQLite dev headers 3.35+, OpenSSL dev headers 1.1+. Per-platform install commands.",
    keywords:
      "prerequisites rust toolchain 1.78 wasm32 nodejs node sqlite openssl dev headers install",
  },
  {
    id: 90,
    pageSlug: "building",
    pageTitle: "Building from Source",
    category: "Developers",
    section: "Clone & Build",
    href: "/docs/building#clone-build",
    content:
      "git clone the repository, cd into omni, run cargo build --release. Full release build takes 3-5 minutes.",
    keywords:
      "clone build git cargo build release compile 3-5 minutes source code",
  },
  {
    id: 91,
    pageSlug: "building",
    pageTitle: "Building from Source",
    category: "Developers",
    section: "Crate Structure",
    href: "/docs/building#crate-structure",
    content:
      "7 crates: omni-core (agent loop, event bus, config, DB), omni-channels (21+ messaging), omni-runtime (WASM, Wasmtime), omni-guardian (anti-injection, permissions), omni-llm (29 native tools, 6 providers), omni-cli (CLI binary), omni-sdk (guest-side SDK).",
    keywords:
      "crate structure workspace omni-core omni-channels omni-runtime omni-guardian omni-tools omni-cli omni-sdk cargo workspace",
  },
  {
    id: 92,
    pageSlug: "building",
    pageTitle: "Building from Source",
    category: "Developers",
    section: "Platform-Specific Notes",
    href: "/docs/building#platform-notes",
    content:
      "Windows: VS Build Tools 2022, vcpkg OpenSSL, OPENSSL_DIR env vars. macOS: Xcode Command Line Tools, Apple Silicon native. Linux: apt install build-essential pkg-config libssl-dev libsqlite3-dev.",
    keywords:
      "platform notes windows vs build tools vcpkg openssl macos xcode apple silicon linux apt build-essential pkg-config libssl-dev",
  },
  {
    id: 93,
    pageSlug: "building",
    pageTitle: "Building from Source",
    category: "Developers",
    section: "Running Tests",
    href: "/docs/building#tests",
    content:
      "Run cargo test --workspace for full suite, or cargo test -p omni-runtime for a specific crate. Use RUST_LOG=debug for verbose output. Integration tests need bot token env vars.",
    keywords:
      "tests cargo test workspace specific crate RUST_LOG debug integration test bot token env",
  },
  {
    id: 94,
    pageSlug: "building",
    pageTitle: "Building from Source",
    category: "Developers",
    section: "Development Workflow",
    href: "/docs/building#dev-workflow",
    content:
      "Tips: cargo watch -x run for hot reload, cargo clippy --workspace for linting, cargo fmt --check for formatting, RUST_LOG=omni_core=debug for targeted logging.",
    keywords:
      "development workflow cargo watch clippy fmt lint format hot reload logging RUST_LOG debug",
  },

  // ─── Changelog ─────────────────────────────────────────────────────
  {
    id: 95,
    pageSlug: "changelog",
    pageTitle: "Changelog",
    category: "Resources",
    section: "Changelog",
    href: "/docs/changelog",
    content:
      "Omni release history following Keep a Changelog conventions. v1.0.0 is planned for March 6, 2026. Tracks platform releases, SDK updates, and security patches.",
    keywords:
      "changelog releases versions v1.0.0 keep a changelog semver updates patches history",
  },
];
