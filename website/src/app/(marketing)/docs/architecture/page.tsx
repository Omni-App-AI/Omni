import type { Metadata } from "next";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Architecture — System Design & Data Flow",
  description:
    "Understand the Omni AI agent system architecture including the Rust crate structure, agent loop design, event bus, SQLite database, WASM runtime, and end-to-end data flow diagrams.",
  openGraph: {
    title: "Omni Architecture — AI Agent System Design & Data Flow",
    description:
      "Understand the Omni AI agent architecture including the Rust crate structure, agent loop design, event bus, SQLite database, WASM runtime, and data flow diagrams.",
    url: "/docs/architecture",
  },
  alternates: { canonical: "/docs/architecture" },
};

export default function ArchitecturePage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Deep Dive
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">
            Architecture
          </h1>
          <p className="text-muted-foreground mb-12">
            How Omni is built — the crate structure, data flow, and key design decisions.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
              {[
                { href: "#crates", label: "Crate Map" },
                { href: "#agent-loop", label: "Agent Loop" },
                { href: "#event-bus", label: "Event Bus" },
                { href: "#database", label: "Database" },
                { href: "#data-flow", label: "Data Flow" },
                { href: "#extension-lifecycle", label: "Extension Lifecycle" },
              ].map((link) => (
                <a
                  key={link.href}
                  href={link.href}
                  className="text-[13px] text-muted-foreground hover:text-primary transition-colors px-2 py-1 rounded hover:bg-primary/5"
                >
                  {link.label}
                </a>
              ))}
            </div>
          </nav>

          {/* Crate Map */}
          <section className="mb-14" id="crates">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Crate Map
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Omni is a Rust workspace split into focused crates. Each crate has a single
              responsibility and clearly defined boundaries.
            </p>
            <div className="space-y-4">
              {[
                {
                  crate: "omni-core",
                  role: "Foundation",
                  desc: "Configuration parsing, database (SQLite + SQLCipher encryption), event bus, logging, and shared types. Every other crate depends on this.",
                },
                {
                  crate: "omni-permissions",
                  role: "Permission Gating",
                  desc: "Capability-based permission system with 20 capability types. PolicyEngine for decision caching, AuditLogger for trail recording, and scope validation.",
                },
                {
                  crate: "omni-guardian",
                  role: "Security Scanning",
                  desc: "4-layer anti-injection pipeline — signature scanning, heuristic analysis, ML classification, and output policy validation. Scans at 5 points in the agent loop.",
                },
                {
                  crate: "omni-extensions",
                  role: "Extension Framework",
                  desc: "Extension host, WASM sandbox (Wasmtime), manifest parsing, extension discovery and installation, host functions (FFI bridge), and storage.",
                },
                {
                  crate: "omni-sdk",
                  role: "Developer SDK",
                  desc: "Published crate for extension developers. Typed clients for HTTP, filesystem, process, storage, LLM, channels, and config. The omni_main! macro and prelude.",
                },
                {
                  crate: "omni-llm",
                  role: "LLM Bridge",
                  desc: "Provider abstraction layer with 6 adapters. SSE streaming, token counting, provider rotation with exponential backoff. Also contains the agent loop, native tools, and hook system.",
                },
                {
                  crate: "omni-channels",
                  role: "Channel Plugins",
                  desc: "21 messaging platform adapters, channel manager, multi-instance support, binding registry, message router, and webhook server.",
                },
                {
                  crate: "ui/src-tauri",
                  role: "Desktop Application",
                  desc: "Tauri v2 shell that wires everything together. 26+ IPC commands, event bridging, and application state management. Excluded from the Cargo workspace.",
                },
              ].map((item, i) => (
                <div key={i} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {String(i + 1).padStart(2, "0")}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-4">
                    <div className="flex items-center gap-3 mb-1">
                      <span className="font-mono text-sm text-primary/80">{item.crate}</span>
                      <span className="text-[11px] font-mono text-muted-foreground/60 bg-secondary px-2 py-0.5 rounded">
                        {item.role}
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground">{item.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Agent Loop */}
          <section className="mb-14" id="agent-loop">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Agent Loop
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              The agent loop is the core orchestration engine. It manages multi-turn conversations
              between the user, the LLM, and tools. Each iteration can invoke one or more tools,
              then feed the results back to the LLM.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Loop Steps (per turn)</h3>
            <div className="space-y-3 mb-6">
              {[
                "Receive user message \u2192 Guardian SP-1 scan \u2192 MessageReceived hook",
                "Assemble full prompt (system + history + message) \u2192 Guardian SP-2 scan \u2192 LlmInput hook",
                "Stream response from LLM provider \u2192 Guardian SP-3 scan \u2192 LlmOutput hook",
                "Parse tool calls from response \u2192 Guardian SP-4 scan \u2192 BeforeToolCall hook",
                "Execute tools (native or extension) \u2192 Permission check \u2192 AfterToolCall hook \u2192 Guardian SP-5 scan",
                "If tools were called, append results and go to step 2 (up to max_iterations)",
                "Return final text response to user",
              ].map((step, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{step}</p>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Tool Resolution</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              When the LLM requests a tool call, Omni resolves the tool name in this order:
            </p>
            <div className="grid grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <div className="bg-card px-4 py-3">
                <p className="text-sm font-medium mb-0.5">1. Native Tools</p>
                <p className="text-xs text-muted-foreground">
                  29 built-in tools (exec, read_file, web_fetch, git, debugger, etc.). Checked first.
                </p>
              </div>
              <div className="bg-card px-4 py-3">
                <p className="text-sm font-medium mb-0.5">2. Flowchart Tools</p>
                <p className="text-xs text-muted-foreground">
                  Tools from enabled <a href="/docs/flowcharts" className="text-primary hover:underline">flowcharts</a>. Visual no-code workflows with 19 node types.
                </p>
              </div>
              <div className="bg-card px-4 py-3">
                <p className="text-sm font-medium mb-0.5">3. Extension Tools</p>
                <p className="text-xs text-muted-foreground">
                  Tools from activated extensions. Resolved by reverse-domain ID (e.g., com.example.weather.get_forecast).
                </p>
              </div>
            </div>
          </section>

          {/* Event Bus */}
          <section className="mb-14" id="event-bus">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Event Bus
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              The event bus is a broadcast channel that distributes system events to all subscribers.
              It uses <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">tokio::sync::broadcast</code> for
              lock-free, multi-consumer event delivery.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Event Types</h3>
            <div className="border border-border/50 rounded-lg overflow-hidden">
              <div className="grid grid-cols-[1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Event</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { event: "MessageReceived", desc: "A new message was received from a user or channel." },
                  { event: "PermissionPrompt", desc: "An extension is requesting a permission that needs user approval." },
                  { event: "GuardianAlert", desc: "The Guardian detected a potential threat." },
                  { event: "ExtensionActivated", desc: "An extension sandbox was started." },
                  { event: "ExtensionDeactivated", desc: "An extension sandbox was stopped." },
                  { event: "ChannelInstanceCreated", desc: "A new channel instance was registered." },
                  { event: "ChannelInstanceRemoved", desc: "A channel instance was removed." },
                  { event: "ChannelBindingAdded", desc: "A new channel binding was created." },
                  { event: "ChannelBindingRemoved", desc: "A channel binding was deleted." },
                ].map((row) => (
                  <>
                    <div key={`e-${row.event}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.event}</div>
                    <div key={`d-${row.event}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed mt-4">
              In the Tauri desktop app, events are bridged to the frontend via Tauri&apos;s event
              system, allowing React components to react to backend state changes in real time.
            </p>
          </section>

          {/* Database */}
          <section className="mb-14" id="database">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Database
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Omni uses SQLite with{" "}
              <a href="https://www.zetetic.net/sqlcipher/" className="text-primary hover:underline">SQLCipher</a>{" "}
              encryption for all persistent storage. The database is accessed through{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Arc&lt;Mutex&lt;Database&gt;&gt;</code>{" "}
              with Tokio&apos;s <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">spawn_blocking</code> to
              avoid blocking the async runtime.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Encryption Key Resolution</h3>
            <div className="space-y-3 mb-6">
              {[
                "OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)",
                "OMNI_DB_KEY environment variable",
                "Key file at ~/.data/Omni/db.key",
                "Auto-generate and store a new key",
              ].map((step, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{step}</p>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Tables</h3>
            <div className="border border-border/50 rounded-lg overflow-hidden">
              <div className="grid grid-cols-[1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Table</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Purpose</div>
                {[
                  { table: "sessions", desc: "Chat sessions with metadata and timestamps." },
                  { table: "messages", desc: "All messages (user, assistant, tool) with role, content, and tool call data." },
                  { table: "permissions", desc: "Persisted permission decisions (Allow/Deny) with duration and use counts." },
                  { table: "audit_logs", desc: "Complete audit trail of all permission requests and decisions." },
                  { table: "extensions", desc: "Installed extensions with version, author, and enabled state." },
                  { table: "extension_storage", desc: "Key-value storage scoped per extension." },
                  { table: "channel_instances", desc: "Configured channel instances with type, display name, and auto-connect." },
                  { table: "channel_bindings", desc: "Message routing rules mapping channels to extensions." },
                ].map((row) => (
                  <>
                    <div key={`t-${row.table}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.table}</div>
                    <div key={`d-${row.table}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Data Flow */}
          <section className="mb-14" id="data-flow">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Data Flow
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              A simplified view of how data flows through the system when a user sends a message.
            </p>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">data flow</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p>User Input</p>
                <p>  &darr;</p>
                <p>Tauri IPC (send_message command)</p>
                <p>  &darr;</p>
                <p>Guardian Scan (SP-1: user input)</p>
                <p>  &darr;</p>
                <p>Hook: MessageReceived</p>
                <p>  &darr;</p>
                <p>Agent Loop</p>
                <p>  &darr;</p>
                <p>LLM Bridge &rarr; Provider Adapter &rarr; SSE Stream</p>
                <p>  &darr;</p>
                <p>Guardian Scan (SP-3: LLM output)</p>
                <p>  &darr;</p>
                <p>Tool Call? &rarr; Permission Check &rarr; Native/Extension Tool</p>
                <p>  &darr;</p>
                <p>Guardian Scan (SP-5: tool output)</p>
                <p>  &darr;</p>
                <p>Response &rarr; Tauri Event (omni:llm-chunk) &rarr; React UI</p>
              </div>
            </div>
          </section>

          {/* Extension Lifecycle */}
          <section className="mb-14" id="extension-lifecycle">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Extension Lifecycle
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              An extension goes through several states from installation to active use.
            </p>
            <div className="space-y-4">
              {[
                {
                  phase: "Install",
                  desc: "The .omni package is extracted, its manifest validated (ID format, version, entrypoint existence), and files copied to the extensions directory. Symlinks are rejected to prevent directory traversal.",
                },
                {
                  phase: "Register",
                  desc: "The extension is registered in the database and in-memory registry. If a previous version exists, SemVer comparison determines whether to upgrade.",
                },
                {
                  phase: "Enable",
                  desc: "The extension is marked as enabled in the database. Enabling an extension auto-activates it.",
                },
                {
                  phase: "Activate",
                  desc: "A Wasmtime WASM sandbox is created with resource limits from the manifest. Host functions are linked, callbacks (LLM, channel) are wired, and a concurrency semaphore is allocated.",
                },
                {
                  phase: "Invoke",
                  desc: "When a tool call is received, the sandbox's handle_tool export is called with the tool name and JSON parameters. The call is bounded by the CPU timeout and concurrency semaphore.",
                },
                {
                  phase: "Deactivate",
                  desc: "The sandbox is torn down, the concurrency semaphore is dropped, and an ExtensionDeactivated event is emitted.",
                },
                {
                  phase: "Uninstall",
                  desc: "The extension's files are deleted, its database record is removed, and all associated permissions are revoked.",
                },
              ].map((item, i) => (
                <div key={i} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {String(i + 1).padStart(2, "0")}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-4">
                    <span className="font-medium text-[15px]">{item.phase}</span>
                    <span className="text-sm text-muted-foreground"> — {item.desc}</span>
                  </div>
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
