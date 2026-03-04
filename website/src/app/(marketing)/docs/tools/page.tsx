import type { Metadata } from "next";
import Link from "next/link";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Native Tools — Built-in AI Agent Functions",
  description:
    "Explore all 29 built-in native tools in the Omni AI agent including file operations, web access, persistent memory, version control, testing, code intelligence, debugging, REPL, sub-agents, MCP client, and desktop app automation.",
  openGraph: {
    title: "Omni Native Tools — 29 Built-in AI Agent Functions",
    description:
      "Explore all 29 built-in native tools in the Omni AI agent including file operations, web access, persistent memory, version control, testing, code intelligence, debugging, REPL, sub-agents, MCP client, and desktop automation.",
    url: "/docs/tools",
  },
  alternates: { canonical: "/docs/tools" },
};

const toolGroups = [
  {
    group: "System",
    tools: [
      {
        name: "exec",
        capability: "process.spawn",
        desc: "Execute shell commands on the host system. Returns stdout, stderr, and exit code.",
        params: "command (string), args (string[], optional), cwd (string, optional)",
      },
      {
        name: "read_file",
        capability: "filesystem.read",
        desc: "Read the contents of a file as text. Supports any text-based file format.",
        params: "path (string)",
      },
      {
        name: "write_file",
        capability: "filesystem.write",
        desc: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does.",
        params: "path (string), content (string)",
      },
      {
        name: "edit_file",
        capability: "filesystem.write",
        desc: "Edit a file by replacing a specific string with new content. Fails if the old string is not found.",
        params: "path (string), old_string (string), new_string (string)",
      },
      {
        name: "list_files",
        capability: "filesystem.read",
        desc: "List files and directories at a given path. Returns names, types, and sizes.",
        params: "path (string), recursive (bool, optional)",
      },
      {
        name: "apply_patch",
        capability: "filesystem.write",
        desc: "Apply a unified diff patch to one or more files. Supports standard patch format.",
        params: "patch (string)",
      },
      {
        name: "grep_search",
        capability: "filesystem.read",
        desc: "Search file contents using regex patterns. Returns matching lines with file paths and line numbers.",
        params: "pattern (string), path (string, optional), include (string, optional)",
      },
    ],
  },
  {
    group: "Web",
    tools: [
      {
        name: "web_fetch",
        capability: "network.http",
        desc: "Fetch content from a URL via HTTP. Supports GET, POST, PUT, DELETE with custom headers and body.",
        params: "url (string), method (string, optional), headers (object, optional), body (string, optional)",
      },
      {
        name: "web_search",
        capability: "search.web",
        desc: "Search the web and return results. Returns titles, URLs, and snippets.",
        params: "query (string), num_results (integer, optional)",
      },
      {
        name: "web_scrape",
        capability: "browser.scrape",
        desc: "Scrape web content with 3 modes: extract (fast HTML parsing), browser (Puppeteer with anti-bot stealth), or crawl (BFS multi-page). Converts HTML to Markdown.",
        params: "url (string), mode (string), selector (string, optional), max_pages (integer, optional), max_depth (integer, optional), url_pattern (string, optional)",
      },
    ],
  },
  {
    group: "Memory",
    tools: [
      {
        name: "memory_save",
        capability: "storage.persistent",
        desc: "Save text to the agent's memory store. Persists across sessions for long-term recall.",
        params: "key (string), content (string), tags (string[], optional)",
      },
      {
        name: "memory_search",
        capability: "storage.persistent",
        desc: "Search saved memories by keyword or tag. Returns matching entries sorted by relevance.",
        params: "query (string), limit (integer, optional)",
      },
      {
        name: "memory_get",
        capability: "storage.persistent",
        desc: "Retrieve a specific memory entry by its key.",
        params: "key (string)",
      },
    ],
  },
  {
    group: "Vision",
    tools: [
      {
        name: "image_analyze",
        capability: "ai.inference",
        desc: "Analyze an image using the LLM's vision capabilities. Describe, extract text, or answer questions about the image.",
        params: "image_path (string), prompt (string, optional)",
      },
    ],
  },
  {
    group: "Messaging",
    tools: [
      {
        name: "send_message",
        capability: "messaging.chat",
        desc: "Send a message through a connected channel. The channel instance and recipient are specified by the agent. Checks channel bindings before sending.",
        params: "channel_id (string), recipient (string), text (string), media_url (string, optional)",
      },
      {
        name: "list_channels",
        capability: "messaging.chat",
        desc: "List all connected channel instances with their status and features.",
        params: "None",
      },
    ],
  },
  {
    group: "Notifications & Scheduling",
    tools: [
      {
        name: "notify",
        capability: "system.notifications",
        desc: "Send a system notification to the user's desktop. Returns structured JSON for the UI to display.",
        params: "title (string), body (string)",
      },
      {
        name: "cron_schedule",
        capability: "system.scheduling",
        desc: "Schedule a recurring task using a cron expression. The task is stored and executed at the specified intervals.",
        params: "name (string), cron_expression (string), action (string)",
      },
    ],
  },
  {
    group: "Sessions",
    tools: [
      {
        name: "session_list",
        capability: "storage.persistent",
        desc: "List all chat sessions with their IDs, creation time, and metadata. Requires database access.",
        params: "limit (integer, optional)",
      },
      {
        name: "session_history",
        capability: "storage.persistent",
        desc: "Retrieve the full message history for a specific session. Requires database access.",
        params: "session_id (string), limit (integer, optional)",
      },
    ],
  },
  {
    group: "Desktop Automation",
    tools: [
      {
        name: "app_interact",
        capability: "app.automation",
        desc: "Launch and control desktop applications via Windows UI Automation APIs. Supports 11 actions: launch, list_windows, find_element, find_elements, click, type_text, read_text, get_tree, get_subtree, screenshot, and close. Security-hardened with LOLBIN blocklist, password field protection, rate limiting, and audit logging.",
        params: "action (string), executable (string, optional), window_title (string, optional), process_name (string, optional), element_name (string, optional), element_type (string, optional), automation_id (string, optional), element_ref (string, optional), text (string, optional), max_depth (integer, optional), max_results (integer, optional), timeout_ms (integer, optional), args (string[], optional)",
      },
    ],
  },
  {
    group: "Version Control",
    tools: [
      {
        name: "git",
        capability: "vcs.operations",
        desc: "Version control operations returning structured JSON. 10 actions: status, diff, log, commit, branch, checkout, stash, merge, show_conflict, resolve. Includes automatic secret scanning before commits and conflict marker parsing.",
        params: "action (string), repo_path (string, optional), message (string, for commit), files (string[], for commit), branch (string), name (string), create (bool), delete (bool), list (bool), staged (bool), file (string), content (string), count (integer), since (string), author (string), pop (bool)",
      },
    ],
  },
  {
    group: "Testing",
    tools: [
      {
        name: "test_runner",
        capability: "process.spawn",
        desc: "Run tests with automatic framework detection and structured output. 3 actions: run (execute tests and parse results), list (discover available tests), coverage (run with coverage enabled). Auto-detects: cargo test (Rust), jest/vitest/mocha (JS/TS), pytest (Python), go test (Go), dotnet test (.NET).",
        params: "action (string), framework (string, optional — auto-detected), file (string, optional), pattern (string, optional), coverage (bool, optional), working_dir (string, optional)",
      },
    ],
  },
  {
    group: "Clipboard",
    tools: [
      {
        name: "clipboard",
        capability: "clipboard.read",
        desc: "Read from or write to the system clipboard. 2 actions: read (get current clipboard text) and write (set clipboard text). Maximum content size: 1 MB.",
        params: "action (string: read | write), content (string, required for write)",
      },
    ],
  },
  {
    group: "Code Intelligence",
    tools: [
      {
        name: "code_search",
        capability: "filesystem.read",
        desc: "Offline code intelligence using syntax-aware regex analysis. 4 actions: index (build symbol index for a project), search (query symbols by name with type/language filters), symbols (list all symbols in a file), dependencies (show imports/uses for a file). Supports 9 languages: Rust, TypeScript, JavaScript, Python, Go, C, C++, Java, C#. Works without a language server.",
        params: "action (string), root_path (string), languages (string[], optional), query (string), type (string, optional), language (string, optional), limit (integer, optional), file (string)",
      },
      {
        name: "lsp",
        capability: "code.intelligence",
        desc: "Language Server Protocol client for real-time code intelligence. 8 actions: start (launch a language server), stop, goto_definition, find_references, hover, diagnostics, symbols (document or workspace), rename_preview. Auto-detects servers: rust-analyzer, typescript-language-server, pyright, gopls.",
        params: "action (string), language (string), root_path (string), file (string), position ({ line, character }), query (string, for workspace symbols)",
      },
    ],
  },
  {
    group: "Agent Orchestration",
    tools: [
      {
        name: "agent_spawn",
        capability: "agent.spawn",
        desc: "Spawn a sub-agent to handle a task in parallel. The sub-agent gets its own conversation context and tool access (except agent_spawn, to prevent recursion). Set wait=true to block until the sub-agent completes, or wait=false to get a task ID for later retrieval.",
        params: "task (string), context_files (string[], optional), model (string, optional), max_iterations (integer, optional — default 15), wait (bool, optional — default true)",
      },
    ],
  },
  {
    group: "Debugging",
    tools: [
      {
        name: "debugger",
        capability: "debug.session",
        desc: "Debug Adapter Protocol (DAP) client for controlling debug sessions. 11 actions: launch (start debug session), attach (connect to running process by PID), set_breakpoints, continue, step_over, step_into, step_out, evaluate (evaluate expression in frame), variables (list variables in scope), stack_trace, disconnect.",
        params: "action (string), program (string), adapter (string, optional — auto-detected), file (string), breakpoints (array of { line }), expression (string), frame_id (integer), process_id (integer, for attach)",
      },
    ],
  },
  {
    group: "Interactive Execution",
    tools: [
      {
        name: "repl",
        capability: "process.spawn",
        desc: "Persistent REPL sessions for interactive code execution. 4 actions: execute (run code in a session), list (show active sessions), reset (clear session state), close (terminate session). Supports Python and Node.js. Up to 3 concurrent sessions, 30-second execution timeout.",
        params: "action (string), language (string: python | javascript), code (string), session_id (string, optional — auto-generated)",
      },
    ],
  },
];

export default function ToolsPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Reference
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">
            Native Tools
          </h1>
          <p className="text-muted-foreground mb-12">
            29 built-in tools available to the agent out of the box. No extensions needed.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-1.5">
              {toolGroups.map((g) => (
                <a
                  key={g.group}
                  href={`#${g.group.toLowerCase().replace(/ & /g, "-")}`}
                  className="text-[13px] text-muted-foreground hover:text-primary transition-colors px-2 py-1 rounded hover:bg-primary/5"
                >
                  {g.group} ({g.tools.length})
                </a>
              ))}
            </div>
          </nav>

          {/* Overview */}
          <section className="mb-14" id="overview">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Overview
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Native tools are built into the Omni runtime and available to the agent immediately.
              The LLM can call these tools during conversations to interact with the filesystem,
              make web requests, manage memory, send messages, and more.
            </p>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Every native tool is permission-gated. The required capability is listed next to each
              tool. If the agent hasn&apos;t been granted the capability, a permission prompt appears
              in the UI.
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { label: "Total Tools", value: "29" },
                { label: "System", value: "7 tools" },
                { label: "Web", value: "3 tools" },
                { label: "Dev Tools", value: "8 tools" },
                { label: "Other", value: "11 tools" },
              ].map((item) => (
                <div key={item.label} className="bg-card px-4 py-3 text-center">
                  <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-1">{item.label}</p>
                  <p className="text-sm font-medium">{item.value}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Tool Groups */}
          {toolGroups.map((group) => (
            <section
              key={group.group}
              className="mb-14"
              id={group.group.toLowerCase().replace(/ & /g, "-")}
            >
              <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
                {group.group} Tools
              </h2>
              <div className="space-y-6">
                {group.tools.map((tool) => (
                  <div key={tool.name} className="border border-border/50 rounded-lg overflow-hidden">
                    <div className="bg-card/60 px-4 py-3 flex items-center justify-between">
                      <code className="text-sm font-mono font-medium text-primary/90">{tool.name}</code>
                      <span className="text-[11px] font-mono text-muted-foreground/60 bg-secondary px-2 py-0.5 rounded">
                        {tool.capability}
                      </span>
                    </div>
                    <div className="bg-card px-4 py-3 space-y-2">
                      <p className="text-sm text-muted-foreground">{tool.desc}</p>
                      <div className="flex gap-2 items-start">
                        <span className="text-[11px] font-mono text-muted-foreground/60 uppercase tracking-widest shrink-0 pt-0.5">
                          Params
                        </span>
                        <p className="text-xs font-mono text-muted-foreground">{tool.params}</p>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          ))}

          {/* Web Scrape Details */}
          <section className="mb-14" id="web-scrape-modes">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Web Scrape Modes
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">web_scrape</code> tool
              supports three modes with increasing capability and resource usage.
            </p>
            <div className="grid grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                {
                  mode: "extract",
                  desc: "Fast HTML parsing using the scraper crate. No browser needed. Best for static pages with predictable HTML structure.",
                  limits: "500 KB/page, 2 MB download",
                },
                {
                  mode: "browser",
                  desc: "Full Puppeteer browser with stealth plugins. Handles JavaScript rendering, anti-bot protection, and dynamic content. Uses Mozilla Readability + Turndown for content extraction.",
                  limits: "500 KB/page, random viewport/delays",
                },
                {
                  mode: "crawl",
                  desc: "BFS multi-page crawl. Follows links matching a URL pattern up to a configurable depth. Combines content from all visited pages.",
                  limits: "100 pages max, depth 5, 5 MB total",
                },
              ].map((m) => (
                <div key={m.mode} className="bg-card px-4 py-4">
                  <p className="text-sm font-mono font-medium text-primary/80 mb-2">{m.mode}</p>
                  <p className="text-xs text-muted-foreground leading-relaxed mb-2">{m.desc}</p>
                  <p className="text-[11px] font-mono text-muted-foreground/60">{m.limits}</p>
                </div>
              ))}
            </div>
          </section>

          {/* App Interact Actions */}
          <section className="mb-14" id="app-interact-actions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              App Interact Actions
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">app_interact</code> tool
              supports 11 actions for full desktop application control. Windows only (uses native UI Automation APIs).
            </p>

            <div className="space-y-4 mb-8">
              {[
                {
                  action: "launch",
                  desc: "Start a desktop application. Returns PID and window title.",
                  params: "executable (required), args (optional)",
                  returns: '{ pid, executable, window_title }',
                },
                {
                  action: "list_windows",
                  desc: "List all visible top-level windows with title, process name, PID, and bounds.",
                  params: "process_name (optional filter)",
                  returns: '{ windows: [...], count }',
                },
                {
                  action: "find_element",
                  desc: "Find a single UI element by name, type, or automation ID. Returns an opaque element_ref for use in subsequent actions.",
                  params: "window_title, process_name, element_name, element_type, automation_id, timeout_ms (default 5000)",
                  returns: '{ element_ref, name, control_type, automation_id, is_enabled, patterns }',
                },
                {
                  action: "find_elements",
                  desc: "Find multiple matching elements. Returns up to max_results matches.",
                  params: "Same as find_element + max_results (default 20, max 100)",
                  returns: '{ elements: [...], count }',
                },
                {
                  action: "click",
                  desc: "Click a UI element using semantic patterns (InvokePattern, TogglePattern, SelectionItemPattern). Never uses screen coordinates.",
                  params: "element_ref (required)",
                  returns: '{ status: "clicked" }',
                },
                {
                  action: "type_text",
                  desc: "Type text into an input element. Uses ValuePattern with SendKeys fallback. Blocked on password fields.",
                  params: "element_ref (required), text (required)",
                  returns: '{ status: "typed" }',
                },
                {
                  action: "read_text",
                  desc: "Read text from an element. Tries ValuePattern, TextPattern, then element name. Blocked on password fields.",
                  params: "element_ref (required)",
                  returns: '{ text: "..." }',
                },
                {
                  action: "get_tree",
                  desc: "Get the UI element tree of a window. Includes truncation reporting when element cap (500) or depth limit is hit.",
                  params: "window_title or process_name, max_depth (default 4, max 8)",
                  returns: '{ root: { name, control_type, children: [...] }, total_elements, depth_reached, truncated }',
                },
                {
                  action: "get_subtree",
                  desc: "Get a subtree starting from a specific element. Useful for exploring deeper when get_tree is truncated.",
                  params: "element_ref (required), max_depth (default 4, max 8)",
                  returns: 'Same structure as get_tree',
                },
                {
                  action: "screenshot",
                  desc: "Capture a window as PNG. Uses Windows GDI PrintWindow (works for occluded windows) with BitBlt fallback. Capped at 4K. Returns base64 image via multimodal pipeline.",
                  params: "window_title or process_name",
                  returns: '{ window_title, width, height, _image_data: [{ mime_type, data }] }',
                },
                {
                  action: "close",
                  desc: "Close a window. Tries graceful close first, then force-kills by PID if that fails.",
                  params: "window_title or process_name",
                  returns: '{ status: "closed" | "force_closed" }',
                },
              ].map((a) => (
                <div key={a.action} className="border border-border/50 rounded-lg overflow-hidden">
                  <div className="bg-card/60 px-4 py-3">
                    <code className="text-sm font-mono font-medium text-primary/90">{a.action}</code>
                  </div>
                  <div className="bg-card px-4 py-3 space-y-2">
                    <p className="text-sm text-muted-foreground">{a.desc}</p>
                    <div className="flex gap-2 items-start">
                      <span className="text-[11px] font-mono text-muted-foreground/60 uppercase tracking-widest shrink-0 pt-0.5">
                        Params
                      </span>
                      <p className="text-xs font-mono text-muted-foreground">{a.params}</p>
                    </div>
                    <div className="flex gap-2 items-start">
                      <span className="text-[11px] font-mono text-muted-foreground/60 uppercase tracking-widest shrink-0 pt-0.5">
                        Returns
                      </span>
                      <p className="text-xs font-mono text-muted-foreground">{a.returns}</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* App Interact Security */}
          <section className="mb-14" id="app-interact-security">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              App Interact Security
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Desktop app automation is a high-risk capability. The{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">app_interact</code> tool
              enforces 12 layers of defense-in-depth to prevent misuse.
            </p>

            <div className="space-y-4 mb-8">
              {[
                {
                  num: "01",
                  title: "Permission Gating",
                  desc: 'The entire tool is gated by the app.automation capability. Requires explicit user approval before any action.',
                },
                {
                  num: "02",
                  title: "LOLBIN Blocklist",
                  desc: "43 dangerous Windows executables (cmd.exe, powershell.exe, rundll32.exe, certutil.exe, mshta.exe, etc.) are permanently blocked from being launched. Case-insensitive, checked against filename regardless of path.",
                },
                {
                  num: "03",
                  title: "Executable Allowlist",
                  desc: "The app.automation scope can restrict which applications are launchable via allowed_apps. Only apps on the list can be opened.",
                },
                {
                  num: "04",
                  title: "Password Field Hard-Block",
                  desc: "The Windows backend checks the IsPassword property before any read or write. Password fields cannot be typed into or read from.",
                },
                {
                  num: "05",
                  title: "Sensitive Name Guard",
                  desc: "Regex patterns detect element names containing password, secret, token, api_key, credit_card, cvv, ssn, pin_code, 2fa, otp, and similar. These elements are blocked for click, type_text, and read_text.",
                },
                {
                  num: "06",
                  title: "Rate Limiting",
                  desc: "60-second sliding window per app, default 60 actions/minute. Configurable via scope. Prevents rapid-fire automation.",
                },
                {
                  num: "07",
                  title: "Max Concurrent Processes",
                  desc: "Default 3 simultaneously running managed processes. Configurable via scope. Prevents resource exhaustion.",
                },
                {
                  num: "08",
                  title: "Tree Depth + Element Cap",
                  desc: "UI tree walks are capped at depth 8 and 500 elements to prevent LLM context overflow. Truncation is reported with actionable suggestions.",
                },
                {
                  num: "09",
                  title: "Value Redaction",
                  desc: "Password field values are automatically replaced with \"[REDACTED]\" in tree output. Sensitive data never enters the LLM context.",
                },
                {
                  num: "10",
                  title: "Semantic Actions Only",
                  desc: "Interactions use UI Automation patterns (InvokePattern, ValuePattern), never raw screen coordinates or simulated mouse events. No way to bypass UI structure.",
                },
                {
                  num: "11",
                  title: "Guardian Scanning",
                  desc: "All text scraped from desktop apps passes through the existing 4-layer Guardian pipeline at scan point SP-5, preventing prompt injection via app content.",
                },
                {
                  num: "12",
                  title: "Audit Events",
                  desc: "Every action (launch, click, type_text, screenshot, etc.) emits an AppAutomationAction audit event with action type, target app, target element, and success/failure status.",
                },
              ].map((layer) => (
                <div key={layer.num} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {layer.num}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-4">
                    <span className="font-medium text-[15px]">{layer.title}</span>
                    <p className="text-sm text-muted-foreground mt-1">{layer.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* App Interact Scope */}
          <section className="mb-14" id="app-interact-scope">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              App Automation Scope
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">app.automation</code>{" "}
              capability accepts a scope with 4 configurable fields to restrict what the tool can do.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden">
              <div className="grid grid-cols-[1fr_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Field</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Default</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { field: "allowed_apps", def: "None (all non-blocked)", desc: "Whitelist of executable names or paths that can be launched. All others are rejected." },
                  { field: "allowed_actions", def: "None (all 11 actions)", desc: "Whitelist of action names that can be used. All others are rejected." },
                  { field: "rate_limit", def: "60", desc: "Maximum actions per minute per app. Sliding 60-second window." },
                  { field: "max_concurrent", def: "3", desc: "Maximum simultaneously running managed processes." },
                ].map((row) => (
                  <>
                    <div key={`f-${row.field}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.field}</div>
                    <div key={`d-${row.field}`} className="bg-card px-3 py-2 text-xs text-muted-foreground font-mono">{row.def}</div>
                    <div key={`e-${row.field}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Element References */}
          <section className="mb-14" id="element-references">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Element References
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              When you call <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">find_element</code> or{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">find_elements</code>, each result
              includes an opaque <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">element_ref</code> string.
              This reference is used in subsequent actions like <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">click</code>,{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">type_text</code>,{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">read_text</code>, and{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">get_subtree</code>.
            </p>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Element references are re-resolved on each use by re-searching the window for the matching element.
              This means references remain valid even if the window is restructured between calls. If the element
              is no longer found, the tool returns a descriptive error.
            </p>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Do not parse or construct element references manually. Always obtain them from{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">find_element</code>,{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">find_elements</code>, or{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">get_tree</code> results.
            </p>
          </section>

          {/* MCP Client */}
          <section className="mb-14" id="mcp-client">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              MCP Client (Model Context Protocol)
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Omni includes a built-in MCP client that can connect to external MCP servers and expose their tools
              to the agent. MCP tools are automatically namespaced as{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">mcp_&lt;server&gt;_&lt;tool&gt;</code>{" "}
              and appear alongside native tools in the agent loop. All MCP tool output is scanned by Guardian at SP-6.
            </p>
            <div className="space-y-4 mb-8">
              {[
                {
                  feature: "Stdio Transport",
                  desc: "Communicates with MCP servers over stdin/stdout using JSON-RPC 2.0. No HTTP server needed — fully local, no network surface.",
                },
                {
                  feature: "Auto-Connect",
                  desc: "MCP servers listed in [mcp.servers] config with auto_start=true are launched automatically on startup.",
                },
                {
                  feature: "Tool Discovery",
                  desc: "On connection, Omni sends tools/list to discover available tools and their JSON schemas. Tools are registered dynamically.",
                },
                {
                  feature: "Namespacing",
                  desc: "Each MCP tool is prefixed with the server name (e.g., filesystem server's read tool becomes mcp_filesystem_read) to prevent collisions.",
                },
                {
                  feature: "Permission Gating",
                  desc: "MCP tool execution requires the mcp.server capability. Scoped by server name and allowed tools list.",
                },
                {
                  feature: "Guardian Scanning",
                  desc: "All MCP tool responses are scanned at SP-6 before being returned to the LLM, preventing prompt injection via external tool output.",
                },
                {
                  feature: "Lifecycle Management",
                  desc: "McpManager supports add, remove, restart, list, and shutdown operations. Servers are killed on drop if unresponsive.",
                },
              ].map((item) => (
                <div key={item.feature} className="border border-border/50 rounded-lg overflow-hidden">
                  <div className="bg-card/60 px-4 py-3">
                    <span className="text-sm font-medium">{item.feature}</span>
                  </div>
                  <div className="bg-card px-4 py-3">
                    <p className="text-sm text-muted-foreground">{item.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Git Actions */}
          <section className="mb-14" id="git-actions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Git Tool Actions
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">git</code> tool
              provides 10 structured version control actions. Prefer this over{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">exec git ...</code>{" "}
              for parsed, JSON-structured output.
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-5 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {["status", "diff", "log", "commit", "branch", "checkout", "stash", "merge", "show_conflict", "resolve"].map((action) => (
                <div key={action} className="bg-card px-3 py-2 text-center">
                  <code className="text-xs font-mono text-primary/80">{action}</code>
                </div>
              ))}
            </div>
            <div className="border border-border/50 rounded-lg overflow-hidden p-4 bg-card/30">
              <p className="text-sm text-muted-foreground mb-2">
                <span className="font-medium text-foreground">Secret scanning:</span>{" "}
                The commit action automatically scans staged content for API keys, tokens, passwords, and other secrets
                before committing. If secrets are detected, the commit is blocked with a detailed warning.
              </p>
              <p className="text-sm text-muted-foreground">
                <span className="font-medium text-foreground">Conflict resolution:</span>{" "}
                The show_conflict action parses conflict markers into structured JSON (ours/theirs/ancestor sections).
                The resolve action writes the final resolved content.
              </p>
            </div>
          </section>

          {/* Debugger Actions */}
          <section className="mb-14" id="debugger-actions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Debugger Actions
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">debugger</code> tool
              implements the Debug Adapter Protocol (DAP) for controlling debug sessions across languages.
              It auto-detects debug adapters for Rust (codelldb), Python (debugpy), Node.js (node-debug), and Go (dlv-dap).
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { action: "launch", desc: "Start a debug session for a program" },
                { action: "attach", desc: "Attach to a running process by PID" },
                { action: "set_breakpoints", desc: "Set breakpoints in a source file" },
                { action: "continue", desc: "Resume execution until next breakpoint" },
                { action: "step_over", desc: "Step over to the next line" },
                { action: "step_into", desc: "Step into a function call" },
                { action: "step_out", desc: "Step out of the current function" },
                { action: "evaluate", desc: "Evaluate an expression in the current frame" },
                { action: "variables", desc: "List variables in the current scope" },
                { action: "stack_trace", desc: "Get the current call stack" },
                { action: "disconnect", desc: "End the debug session" },
              ].map((a) => (
                <div key={a.action} className="bg-card px-3 py-3">
                  <code className="text-xs font-mono font-medium text-primary/80">{a.action}</code>
                  <p className="text-xs text-muted-foreground mt-1">{a.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* LSP Actions */}
          <section className="mb-14" id="lsp-actions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              LSP Tool Actions
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">lsp</code> tool
              manages Language Server Protocol connections and exposes real-time code intelligence. Auto-detects
              servers: rust-analyzer (Rust), typescript-language-server (TS/JS), pyright (Python), gopls (Go).
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { action: "start", desc: "Launch a language server for a project" },
                { action: "stop", desc: "Shut down a running language server" },
                { action: "goto_definition", desc: "Jump to the definition of a symbol" },
                { action: "find_references", desc: "Find all references to a symbol" },
                { action: "hover", desc: "Get type info and docs for a position" },
                { action: "diagnostics", desc: "Get compiler errors and warnings" },
                { action: "symbols", desc: "List symbols in a file or workspace" },
                { action: "rename_preview", desc: "Preview renames across files" },
              ].map((a) => (
                <div key={a.action} className="bg-card px-3 py-3">
                  <code className="text-xs font-mono font-medium text-primary/80">{a.action}</code>
                  <p className="text-xs text-muted-foreground mt-1">{a.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Next Steps */}
          <section id="next-steps">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Next Steps
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <Link
                href="/docs/flowcharts"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Flowchart Builder
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Use all 29 tools visually — no code required.
                </p>
              </Link>
              <Link
                href="/docs/hooks"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Hook System
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Intercept and modify tool calls with hooks.
                </p>
              </Link>
              <Link
                href="/docs/sdk"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Build Extension Tools
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Create your own tools with the Omni SDK.
                </p>
              </Link>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
