import type { Metadata } from "next";
import Link from "next/link";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Security & Permissions — Extension Safety",
  description:
    "Learn how Omni's 4-layer Guardian anti-injection pipeline, WASM sandboxing, capability-based permissions, and audit logging protect your AI agent from malicious extensions and prompt attacks.",
  openGraph: {
    title: "Omni Security & Permissions — AI Agent Extension Safety",
    description:
      "Learn how Omni's 4-layer Guardian pipeline, WASM sandboxing, capability-based permissions, and audit logging protect your AI agent from malicious extensions.",
    url: "/docs/security",
  },
  alternates: { canonical: "/docs/security" },
};

export default function SecurityPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Guide
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">
            Security &amp; Permissions
          </h1>
          <p className="text-muted-foreground mb-12">
            How Omni keeps you safe — prompt injection detection, capability-based permissions,
            WASM sandboxing, and full audit trails.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
              {[
                { href: "#overview", label: "Overview" },
                { href: "#guardian", label: "Guardian Pipeline" },
                { href: "#scan-points", label: "Scan Points" },
                { href: "#sensitivity", label: "Sensitivity Levels" },
                { href: "#permissions", label: "Permission System" },
                { href: "#capabilities", label: "All Capabilities" },
                { href: "#sandbox", label: "WASM Sandbox" },
                { href: "#audit", label: "Audit Logging" },
                { href: "#kill-switch", label: "Kill Switch" },
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

          {/* Overview */}
          <section className="mb-14" id="overview">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Overview</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Omni is designed with security as a core principle. Every piece of text that enters
              or leaves the system is scanned for prompt injection attacks. Every action an extension
              takes is gated by a capability-based permission system. And every extension runs in an
              isolated WebAssembly sandbox with strict resource limits.
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                { label: "Guardian", value: "4-layer anti-injection pipeline" },
                { label: "Permissions", value: "21 capability types with scopes" },
                { label: "Sandbox", value: "Wasmtime WASM isolation" },
              ].map((item) => (
                <div key={item.label} className="bg-card px-4 py-3">
                  <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-1">{item.label}</p>
                  <p className="text-sm font-medium">{item.value}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Guardian Pipeline */}
          <section className="mb-14" id="guardian">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Guardian Anti-Injection Pipeline
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The Guardian is a 4-layer scanning pipeline that detects prompt injection attacks,
              malicious instructions, and other adversarial inputs in real time. Each layer
              contributes a weighted score to produce an overall risk assessment.
            </p>

            <div className="space-y-6 mb-8">
              {[
                {
                  num: "01",
                  title: "Signature Scanner",
                  weight: "30%",
                  desc: "Matches input against 79+ compiled regex patterns for known injection techniques. Detects encoding bypasses including base64, Unicode NFKC normalization, Cyrillic homoglyphs, and zero-width character stripping.",
                },
                {
                  num: "02",
                  title: "Heuristic Scanner",
                  weight: "25%",
                  desc: "Applies 5 weighted behavioral rules that detect suspicious patterns independent of specific encodings. Catches novel attack variants that signatures miss.",
                },
                {
                  num: "03",
                  title: "ML Classifier",
                  weight: "30%",
                  desc: "AI-powered classification that analyzes the semantic intent of text. Feature-gated behind the ml-classifier flag. Returns a confidence score when enabled.",
                },
                {
                  num: "04",
                  title: "Output Policy Validator",
                  weight: "15%",
                  desc: "Validates tool calls against their declared schemas. Prevents extensions from invoking tools they haven't declared or passing invalid parameters.",
                },
              ].map((layer) => (
                <div key={layer.num} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {layer.num}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-6">
                    <div className="flex items-center gap-3 mb-1">
                      <span className="font-medium text-[15px]">{layer.title}</span>
                      <span className="text-[11px] font-mono text-primary/70 bg-primary/5 px-2 py-0.5 rounded">
                        {layer.weight}
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground">{layer.desc}</p>
                  </div>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Scan Verdicts</h3>
            <div className="grid grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { verdict: "Clean", score: "\u2265 80", color: "text-success", desc: "No threats detected. Content proceeds normally." },
                { verdict: "Suspicious", score: "50 \u2013 79", color: "text-warning", desc: "Potential threat. User is warned and can override if allow_override is true." },
                { verdict: "Malicious", score: "< 50", color: "text-destructive", desc: "High-confidence threat. Content is blocked." },
              ].map((v) => (
                <div key={v.verdict} className="bg-card px-4 py-4">
                  <p className={`text-sm font-medium ${v.color} mb-0.5`}>{v.verdict}</p>
                  <p className="text-[11px] font-mono text-muted-foreground/60 mb-2">Score {v.score}</p>
                  <p className="text-xs text-muted-foreground">{v.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Scan Points */}
          <section className="mb-14" id="scan-points">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Scan Points
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The Guardian scans at 5 points throughout the agent loop, covering both incoming and
              outgoing data.
            </p>
            <div className="space-y-4">
              {[
                { point: "SP-1", title: "User Input", desc: "Every message from the user is scanned before it enters the conversation." },
                { point: "SP-2", title: "Prompt Assembly", desc: "The full prompt (system + history + user message) is scanned before being sent to the LLM." },
                { point: "SP-3", title: "LLM Output", desc: "Each response chunk from the LLM is scanned for injected instructions." },
                { point: "SP-4", title: "Tool Calls", desc: "Before any tool is executed, its name and arguments are validated against declared schemas." },
                { point: "SP-5", title: "Extension Output", desc: "Data returned by extensions is scanned before being shown to the user or fed back to the LLM." },
              ].map((sp) => (
                <div key={sp.point} className="flex gap-4">
                  <span className="text-[11px] font-mono text-primary/70 bg-primary/5 px-2 py-0.5 rounded h-fit shrink-0 mt-0.5">
                    {sp.point}
                  </span>
                  <div className="flex-1 border-b border-border/50 pb-4">
                    <span className="font-medium text-[15px]">{sp.title}</span>
                    <span className="text-sm text-muted-foreground"> — {sp.desc}</span>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Sensitivity Levels */}
          <section className="mb-14" id="sensitivity">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Sensitivity Levels
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The Guardian sensitivity level controls how aggressively threats are flagged. Set it in{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">omni.toml</code> under{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">[guardian].sensitivity</code>.
            </p>
            <div className="grid grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                {
                  level: "strict",
                  desc: "Lowest thresholds, most alerts. Best for high-security environments where false positives are acceptable.",
                },
                {
                  level: "balanced",
                  desc: "Default. Good balance between detection and usability. Suitable for most use cases.",
                },
                {
                  level: "permissive",
                  desc: "Highest thresholds, fewest alerts. Use when working with content that frequently triggers false positives.",
                },
              ].map((s) => (
                <div key={s.level} className="bg-card px-4 py-4">
                  <p className="text-sm font-mono font-medium text-primary/80 mb-2">{s.level}</p>
                  <p className="text-xs text-muted-foreground leading-relaxed">{s.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Permission System */}
          <section className="mb-14" id="permissions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Permission System
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Every action that accesses a resource — network, filesystem, processes, messaging — is
              gated by a capability-based permission system. Extensions declare the permissions they
              need in their manifest, and users approve them at install time or on first use.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Permission Decisions</h3>
            <div className="grid grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                { decision: "Allow", desc: "Permission granted. The action proceeds." },
                { decision: "Deny", desc: "Permission rejected. The extension receives a PermissionDenied error." },
                { decision: "Prompt", desc: "Ask the user to decide. A dialog appears in the UI." },
              ].map((d) => (
                <div key={d.decision} className="bg-card px-4 py-3">
                  <p className="text-sm font-medium mb-0.5">{d.decision}</p>
                  <p className="text-xs text-muted-foreground">{d.desc}</p>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Permission Duration</h3>
            <div className="grid grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                { duration: "Once", desc: "Granted for this single call only." },
                { duration: "Session", desc: "Granted for the current session. Revoked when Omni restarts." },
                { duration: "Always", desc: "Persisted to the database. Survives restarts." },
              ].map((d) => (
                <div key={d.duration} className="bg-card px-4 py-3">
                  <p className="text-sm font-mono font-medium mb-0.5">{d.duration}</p>
                  <p className="text-xs text-muted-foreground">{d.desc}</p>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Default Policy</h3>
            <p className="text-sm text-muted-foreground leading-relaxed">
              When no rule matches an incoming permission request, the{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">default_policy</code>{" "}
              setting determines the outcome.{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">&quot;deny&quot;</code>{" "}
              blocks silently.{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">&quot;prompt&quot;</code>{" "}
              asks the user. The default is{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">&quot;deny&quot;</code>.
            </p>
          </section>

          {/* All Capabilities */}
          <section className="mb-14" id="capabilities">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              All 26 Capabilities
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Each capability controls access to a specific resource type. Capabilities with scopes
              allow fine-grained control over exactly what the extension can access.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden">
              <div className="grid grid-cols-[1fr_auto_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Capability</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Scope</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { cap: "network.http", scope: "domains, methods, ports", desc: "Make HTTP/HTTPS requests to external APIs." },
                  { cap: "network.websocket", scope: "domains", desc: "Open persistent WebSocket connections." },
                  { cap: "filesystem.read", scope: "paths, extensions, max_size", desc: "Read files from the host filesystem." },
                  { cap: "filesystem.write", scope: "paths, extensions, max_size", desc: "Create or modify files on the host filesystem." },
                  { cap: "clipboard.read", scope: "None", desc: "Read from the system clipboard." },
                  { cap: "clipboard.write", scope: "None", desc: "Write to the system clipboard." },
                  { cap: "messaging.sms", scope: "recipients, rate_limit", desc: "Send SMS text messages." },
                  { cap: "messaging.email", scope: "recipients, rate_limit", desc: "Send emails." },
                  { cap: "messaging.chat", scope: "recipients, rate_limit", desc: "Send chat messages via connected channels." },
                  { cap: "search.web", scope: "providers, rate_limit", desc: "Perform web searches." },
                  { cap: "process.spawn", scope: "executables, allowed_args, denied_args", desc: "Spawn OS processes and commands." },
                  { cap: "system.notifications", scope: "None", desc: "Display system notifications." },
                  { cap: "system.scheduling", scope: "None", desc: "Schedule cron jobs and recurring tasks." },
                  { cap: "device.camera", scope: "None", desc: "Access the device camera." },
                  { cap: "device.microphone", scope: "None", desc: "Access the device microphone." },
                  { cap: "device.location", scope: "None", desc: "Access device location data." },
                  { cap: "storage.persistent", scope: "max_bytes", desc: "Store persistent key-value data." },
                  { cap: "browser.scrape", scope: "domains, max_pages", desc: "Scrape web content using a browser engine." },
                  { cap: "ai.inference", scope: "max_tokens, rate_limit", desc: "Make LLM inference requests through Omni's providers." },
                  { cap: "channel.send", scope: "channels, rate_limit", desc: "Send messages through connected channel instances." },
                  { cap: "app.automation", scope: "allowed_apps, allowed_actions, rate_limit, max_concurrent", desc: "Control desktop applications via UI Automation. LOLBIN blocklist, password field protection, and audit logging enforced." },
                  { cap: "vcs.operations", scope: "allowed_repos, allowed_actions", desc: "Version control operations (git commit, branch, merge). Includes automatic secret scanning." },
                  { cap: "mcp.server", scope: "servers, allowed_tools", desc: "Connect to MCP tool servers and invoke their tools. Scoped by server name." },
                  { cap: "code.intelligence", scope: "None", desc: "Code intelligence features (LSP navigation, code search, symbol lookup)." },
                  { cap: "agent.spawn", scope: "max_concurrent, max_iterations", desc: "Spawn sub-agents for parallel task execution." },
                  { cap: "debug.session", scope: "None", desc: "Control debug sessions — breakpoints, stepping, variable inspection via DAP." },
                ].map((row) => (
                  <>
                    <div key={`c-${row.cap}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.cap}</div>
                    <div key={`s-${row.cap}`} className="bg-card px-3 py-2 text-xs text-muted-foreground font-mono">{row.scope}</div>
                    <div key={`d-${row.cap}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* WASM Sandbox */}
          <section className="mb-14" id="sandbox">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              WASM Sandbox
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Every extension runs in an isolated WebAssembly sandbox powered by Wasmtime.
              Extensions cannot access the host system directly — they must go through host
              functions that are permission-gated.
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                { label: "Memory Limit", value: "64 MB default", desc: "Configurable per extension via max_memory_mb" },
                { label: "CPU Limit", value: "5,000 ms default", desc: "Per tool call via max_cpu_ms_per_call" },
                { label: "Concurrency", value: "4 concurrent calls", desc: "Per extension via max_concurrent_calls" },
              ].map((item) => (
                <div key={item.label} className="bg-card px-4 py-3">
                  <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-1">{item.label}</p>
                  <p className="text-sm font-medium mb-0.5">{item.value}</p>
                  <p className="text-xs text-muted-foreground">{item.desc}</p>
                </div>
              ))}
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Extensions that exceed their memory or CPU limits are terminated immediately. The
              concurrency limit is enforced by a per-extension semaphore — additional calls block
              until a slot becomes available.
            </p>
          </section>

          {/* Audit Logging */}
          <section className="mb-14" id="audit">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Audit Logging
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              When <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">audit_enabled</code> is{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">true</code> (default),
              every permission decision is recorded to the database with:
            </p>
            <ul className="space-y-2 text-sm text-muted-foreground mb-6">
              {[
                "Extension ID that requested the permission",
                "Capability that was requested (e.g., network.http)",
                "Decision made (allow, deny, or prompt result)",
                "Reason or context for the decision",
                "Session ID and timestamp",
              ].map((item, i) => (
                <li key={i} className="flex gap-2">
                  <span className="text-muted-foreground/40 shrink-0">-</span>
                  {item}
                </li>
              ))}
            </ul>
            <p className="text-sm text-muted-foreground leading-relaxed">
              View the audit log from <strong className="text-foreground">Settings &rarr; Permissions &rarr; Audit Log</strong> in
              the UI. You can filter by extension, capability, decision, and time range. Export to JSON or CSV
              for external analysis.
            </p>
          </section>

          {/* Kill Switch */}
          <section className="mb-14" id="kill-switch">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Kill Switch
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              The kill switch instantly revokes <strong className="text-foreground">all</strong> granted
              permissions for <strong className="text-foreground">all</strong> extensions. Use it in
              emergency situations when you suspect a compromised extension.
            </p>
            <p className="text-sm text-muted-foreground leading-relaxed mb-6">
              Access it from <strong className="text-foreground">Settings &rarr; Permissions &rarr; Kill Switch</strong> or
              via the Tauri command{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">kill_switch</code>.
              After activation, every extension must re-request its permissions.
            </p>
            <p className="text-sm text-muted-foreground leading-relaxed">
              You can also revoke permissions for a single extension
              from the extension&apos;s detail panel without affecting others.
            </p>
          </section>

          {/* Next Steps */}
          <section id="next-steps">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Next Steps
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <Link
                href="/docs/sdk#permissions"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  SDK Permission Scopes
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Learn how to declare permissions in your extension manifest.
                </p>
              </Link>
              <Link
                href="/docs/configuration#guardian"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Guardian Configuration
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Tune sensitivity, add custom signatures, and configure overrides.
                </p>
              </Link>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
