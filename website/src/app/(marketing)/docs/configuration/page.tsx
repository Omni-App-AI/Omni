import type { Metadata } from "next";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Configuration — Settings & Options Guide",
  description:
    "Complete reference for the Omni AI agent configuration file. Configure LLM providers, agent behavior, guardian security, WASM extension permissions, UI settings, and messaging channels.",
  openGraph: {
    title: "Omni Configuration Reference — AI Agent Settings & Options",
    description:
      "Complete reference for the Omni AI agent configuration file. Configure LLM providers, agent behavior, guardian security, extension permissions, UI settings, and channels.",
    url: "/docs/configuration",
  },
  alternates: { canonical: "/docs/configuration" },
};

export default function ConfigurationPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Reference
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">
            Configuration Reference
          </h1>
          <p className="text-muted-foreground mb-12">
            Complete reference for the <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">omni.toml</code> configuration file.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
              {[
                { href: "#file-location", label: "File Location" },
                { href: "#general", label: "General" },
                { href: "#providers", label: "Providers" },
                { href: "#agent", label: "Agent" },
                { href: "#guardian", label: "Guardian" },
                { href: "#permissions", label: "Permissions" },
                { href: "#ui", label: "UI / Appearance" },
                { href: "#channels", label: "Channels" },
                { href: "#marketplace", label: "Marketplace" },
                { href: "#full-example", label: "Full Example" },
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

          {/* File Location */}
          <section className="mb-14" id="file-location">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              File Location
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Omni reads its configuration from a TOML file. The file is created automatically on
              first launch with sensible defaults. You can edit it manually or through the Settings
              panel in the UI.
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                { label: "Linux", value: "~/.config/omni/omni.toml" },
                { label: "macOS", value: "~/Library/Application Support/Omni/omni.toml" },
                { label: "Windows", value: "%APPDATA%\\Omni\\omni.toml" },
              ].map((item) => (
                <div key={item.label} className="bg-card px-4 py-3">
                  <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-1">
                    {item.label}
                  </p>
                  <p className="text-sm font-mono text-muted-foreground">{item.value}</p>
                </div>
              ))}
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              All sections are optional. Omni uses defaults for any missing values.
            </p>
          </section>

          {/* General */}
          <section className="mb-14" id="general">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              [general]
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Top-level application settings like logging and data storage.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Key
                </div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Type
                </div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Default
                </div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">
                  Description
                </div>
                {[
                  { key: "data_dir", type: "string?", def: "OS default", desc: "Override the data directory path. Uses OS-specific default if not set." },
                  { key: "telemetry", type: "bool", def: "false", desc: "Enable anonymous usage telemetry." },
                  { key: "log_level", type: "string", def: "\"info\"", desc: "Log verbosity. Values: trace, debug, info, warn, error." },
                  { key: "max_history", type: "integer", def: "1000", desc: "Maximum number of messages to keep in session history." },
                ].map((row) => (
                  <>
                    <div key={`k-${row.key}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">
                      {row.key}
                    </div>
                    <div key={`t-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">
                      {row.type}
                    </div>
                    <div key={`d-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">
                      {row.def}
                    </div>
                    <div key={`desc-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">
                      {row.desc}
                    </div>
                  </>
                ))}
              </div>
            </div>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[general]</span></p>
                <p>telemetry = <span className="text-warning">false</span></p>
                <p>log_level = <span className="text-success">&quot;info&quot;</span></p>
                <p>max_history = <span className="text-warning">1000</span></p>
              </div>
            </div>
          </section>

          {/* Providers */}
          <section className="mb-14" id="providers">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              [providers.&lt;name&gt;]
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Configure one or more LLM providers. Each provider has a unique key (e.g.,{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">providers.openai</code>).
              You can configure multiple providers and Omni will automatically fall back between them.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Key</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Default</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { key: "provider_type", type: "string", def: "required", desc: "Provider type: openai, anthropic, ollama, gemini, bedrock, custom." },
                  { key: "default_model", type: "string?", def: "—", desc: "Default model to use (e.g., \"gpt-4o\", \"claude-opus-4-6\")." },
                  { key: "endpoint", type: "string?", def: "—", desc: "Custom API endpoint URL. Required for ollama and custom providers." },
                  { key: "max_tokens", type: "integer?", def: "—", desc: "Maximum tokens per response." },
                  { key: "temperature", type: "float?", def: "—", desc: "Sampling temperature (0.0 – 2.0). Lower = more deterministic." },
                  { key: "enabled", type: "bool", def: "true", desc: "Whether this provider is active." },
                ].map((row) => (
                  <>
                    <div key={`k-${row.key}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.key}</div>
                    <div key={`t-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.type}</div>
                    <div key={`d-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.def}</div>
                    <div key={`desc-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              API keys are stored securely in the OS keychain (or via environment variables), not in the config file.
            </p>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[providers.openai]</span></p>
                <p>provider_type = <span className="text-success">&quot;openai&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;gpt-4o&quot;</span></p>
                <p>max_tokens = <span className="text-warning">4096</span></p>
                <p>temperature = <span className="text-warning">0.7</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[providers.anthropic]</span></p>
                <p>provider_type = <span className="text-success">&quot;anthropic&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;claude-opus-4-6&quot;</span></p>
                <p>temperature = <span className="text-warning">0.8</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[providers.ollama]</span></p>
                <p>provider_type = <span className="text-success">&quot;ollama&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;llama3.1&quot;</span></p>
                <p>endpoint = <span className="text-success">&quot;http://localhost:11434&quot;</span></p>
              </div>
            </div>
          </section>

          {/* Agent */}
          <section className="mb-14" id="agent">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              [agent]
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Controls the AI agent&apos;s behavior — the system prompt, how many tool-use iterations
              it can perform, and the overall timeout.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Key</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Default</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { key: "system_prompt", type: "string?", def: "—", desc: "Custom system prompt prepended to every conversation." },
                  { key: "max_iterations", type: "integer", def: "25", desc: "Max tool-use iterations per turn before the agent stops." },
                  { key: "timeout_secs", type: "integer", def: "120", desc: "Maximum seconds for a single agent turn." },
                ].map((row) => (
                  <>
                    <div key={`k-${row.key}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.key}</div>
                    <div key={`t-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.type}</div>
                    <div key={`d-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.def}</div>
                    <div key={`desc-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[agent]</span></p>
                <p>system_prompt = <span className="text-success">&quot;You are a helpful assistant.&quot;</span></p>
                <p>max_iterations = <span className="text-warning">25</span></p>
                <p>timeout_secs = <span className="text-warning">120</span></p>
              </div>
            </div>
          </section>

          {/* Guardian */}
          <section className="mb-14" id="guardian">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              [guardian]
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Configure the Guardian anti-injection pipeline that scans all inputs and outputs
              for prompt injection attacks.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Key</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Default</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { key: "enabled", type: "bool", def: "true", desc: "Enable or disable the Guardian pipeline." },
                  { key: "sensitivity", type: "string", def: "\"balanced\"", desc: "Detection sensitivity: strict, balanced, or permissive." },
                  { key: "custom_signatures", type: "string?", def: "—", desc: "Path to a custom regex signatures JSON file." },
                  { key: "allow_override", type: "bool", def: "true", desc: "Allow users to override blocked content from the UI." },
                ].map((row) => (
                  <>
                    <div key={`k-${row.key}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.key}</div>
                    <div key={`t-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.type}</div>
                    <div key={`d-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.def}</div>
                    <div key={`desc-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[guardian]</span></p>
                <p>enabled = <span className="text-warning">true</span></p>
                <p>sensitivity = <span className="text-success">&quot;balanced&quot;</span></p>
                <p>allow_override = <span className="text-warning">true</span></p>
              </div>
            </div>
          </section>

          {/* Permissions */}
          <section className="mb-14" id="permissions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              [permissions]
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Default behavior for the permission system. Individual extensions can request
              specific permissions through their manifests.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Key</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Default</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { key: "default_policy", type: "string", def: "\"deny\"", desc: "What happens when no rule matches: deny (block silently) or prompt (ask user)." },
                  { key: "trust_verified", type: "bool", def: "false", desc: "Auto-approve permissions for marketplace-verified extensions." },
                  { key: "audit_enabled", type: "bool", def: "true", desc: "Log all permission decisions to the audit trail." },
                ].map((row) => (
                  <>
                    <div key={`k-${row.key}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.key}</div>
                    <div key={`t-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.type}</div>
                    <div key={`d-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.def}</div>
                    <div key={`desc-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[permissions]</span></p>
                <p>default_policy = <span className="text-success">&quot;deny&quot;</span></p>
                <p>trust_verified = <span className="text-warning">false</span></p>
                <p>audit_enabled = <span className="text-warning">true</span></p>
              </div>
            </div>
          </section>

          {/* UI */}
          <section className="mb-14" id="ui">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              [ui]
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Customize the appearance of the desktop application. All settings here can also be
              changed from <strong className="text-foreground">Settings &rarr; Appearance</strong> in
              the UI.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Key</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Default</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { key: "theme", type: "string", def: "\"system\"", desc: "Color theme: light, dark, or system." },
                  { key: "accent_color", type: "string", def: "\"#3b82f6\"", desc: "Primary accent color as a hex value." },
                  { key: "font_family", type: "string", def: "\"system\"", desc: "Font family: system, Inter, JetBrains Mono, Fira Code, Source Sans 3." },
                  { key: "font_size", type: "integer", def: "14", desc: "Base font size in pixels." },
                  { key: "line_height", type: "string", def: "\"normal\"", desc: "Line spacing: normal, relaxed, or loose." },
                  { key: "ui_density", type: "string", def: "\"comfortable\"", desc: "UI spacing: compact, comfortable, or spacious." },
                  { key: "sidebar_width", type: "integer", def: "250", desc: "Sidebar width in pixels." },
                  { key: "message_style", type: "string", def: "\"bubbles\"", desc: "Chat message layout: bubbles, flat, or compact." },
                  { key: "max_message_width", type: "integer", def: "75", desc: "Maximum message width as a percentage of the chat area." },
                  { key: "code_theme", type: "string", def: "\"dark\"", desc: "Code block theme: light, dark, or auto." },
                  { key: "show_timestamps", type: "bool", def: "false", desc: "Show timestamps on chat messages." },
                  { key: "border_radius", type: "integer", def: "8", desc: "Corner radius in pixels for UI elements." },
                  { key: "reduce_animations", type: "bool", def: "false", desc: "Disable animations for accessibility." },
                  { key: "high_contrast", type: "bool", def: "false", desc: "Increase text contrast for readability." },
                  { key: "auto_update", type: "bool", def: "true", desc: "Automatically check for and install updates." },
                ].map((row) => (
                  <>
                    <div key={`k-${row.key}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.key}</div>
                    <div key={`t-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.type}</div>
                    <div key={`d-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.def}</div>
                    <div key={`desc-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Channels */}
          <section className="mb-14" id="channels">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              [channels]
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Pre-configure channel instances and bindings. Instances define which messaging platforms
              to connect to; bindings route incoming messages to specific extensions.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Channel Instances</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Each instance is keyed by a compound ID in the format{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">{"{type}:{instance_id}"}</code>.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Key</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Default</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { key: "channel_type", type: "string", def: "required", desc: "Channel type: discord, telegram, slack, whatsapp_web, etc." },
                  { key: "display_name", type: "string?", def: "—", desc: "Human-readable label shown in the UI." },
                  { key: "auto_connect", type: "bool", def: "false", desc: "Connect automatically on startup." },
                ].map((row) => (
                  <>
                    <div key={`k-${row.key}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.key}</div>
                    <div key={`t-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.type}</div>
                    <div key={`d-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.def}</div>
                    <div key={`desc-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Channel Bindings</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Bindings route incoming messages from a channel instance to an extension. Use glob
              patterns in <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">peer_filter</code> and{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">group_filter</code> to
              match specific senders or groups.
            </p>

            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[channels.instances.&quot;discord:production&quot;]</span></p>
                <p>channel_type = <span className="text-success">&quot;discord&quot;</span></p>
                <p>display_name = <span className="text-success">&quot;Main Server&quot;</span></p>
                <p>auto_connect = <span className="text-warning">true</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[channels.instances.&quot;telegram:alerts&quot;]</span></p>
                <p>channel_type = <span className="text-success">&quot;telegram&quot;</span></p>
                <p>display_name = <span className="text-success">&quot;Alert Bot&quot;</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[[channels.bindings]]</span></p>
                <p>channel_instance = <span className="text-success">&quot;discord:production&quot;</span></p>
                <p>extension_id = <span className="text-success">&quot;com.example.support-bot&quot;</span></p>
                <p>peer_filter = <span className="text-success">&quot;*&quot;</span></p>
                <p>group_filter = <span className="text-success">&quot;support-*&quot;</span></p>
                <p>priority = <span className="text-warning">100</span></p>
              </div>
            </div>
          </section>

          {/* Marketplace */}
          <section className="mb-14" id="marketplace">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              [marketplace]
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Marketplace connection settings.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Key</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Type</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Default</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { key: "api_url", type: "string", def: "\"https://omniapp.org/api/v1/marketplace\"", desc: "Marketplace API endpoint. Override for self-hosted instances." },
                ].map((row) => (
                  <>
                    <div key={`k-${row.key}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.key}</div>
                    <div key={`t-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.type}</div>
                    <div key={`d-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground font-mono">{row.def}</div>
                    <div key={`desc-${row.key}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Full Example */}
          <section className="mb-14" id="full-example">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Full Example
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              A complete configuration file showing all sections with typical values.
            </p>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1 text-[13px]">
                <p><span className="text-foreground/60">[general]</span></p>
                <p>telemetry = <span className="text-warning">false</span></p>
                <p>log_level = <span className="text-success">&quot;info&quot;</span></p>
                <p>max_history = <span className="text-warning">1000</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[providers.openai]</span></p>
                <p>provider_type = <span className="text-success">&quot;openai&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;gpt-4o&quot;</span></p>
                <p>max_tokens = <span className="text-warning">4096</span></p>
                <p>temperature = <span className="text-warning">0.7</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[providers.anthropic]</span></p>
                <p>provider_type = <span className="text-success">&quot;anthropic&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;claude-opus-4-6&quot;</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[providers.ollama]</span></p>
                <p>provider_type = <span className="text-success">&quot;ollama&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;llama3.1&quot;</span></p>
                <p>endpoint = <span className="text-success">&quot;http://localhost:11434&quot;</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[agent]</span></p>
                <p>system_prompt = <span className="text-success">&quot;You are a helpful assistant.&quot;</span></p>
                <p>max_iterations = <span className="text-warning">25</span></p>
                <p>timeout_secs = <span className="text-warning">120</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[guardian]</span></p>
                <p>enabled = <span className="text-warning">true</span></p>
                <p>sensitivity = <span className="text-success">&quot;balanced&quot;</span></p>
                <p>allow_override = <span className="text-warning">true</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[permissions]</span></p>
                <p>default_policy = <span className="text-success">&quot;deny&quot;</span></p>
                <p>trust_verified = <span className="text-warning">false</span></p>
                <p>audit_enabled = <span className="text-warning">true</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[ui]</span></p>
                <p>theme = <span className="text-success">&quot;system&quot;</span></p>
                <p>accent_color = <span className="text-success">&quot;#3b82f6&quot;</span></p>
                <p>message_style = <span className="text-success">&quot;bubbles&quot;</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[channels.instances.&quot;discord:production&quot;]</span></p>
                <p>channel_type = <span className="text-success">&quot;discord&quot;</span></p>
                <p>display_name = <span className="text-success">&quot;Main Server&quot;</span></p>
                <p>auto_connect = <span className="text-warning">true</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[[channels.bindings]]</span></p>
                <p>channel_instance = <span className="text-success">&quot;discord:production&quot;</span></p>
                <p>extension_id = <span className="text-success">&quot;com.example.bot&quot;</span></p>
                <p>priority = <span className="text-warning">100</span></p>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
