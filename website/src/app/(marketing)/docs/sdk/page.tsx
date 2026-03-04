import type { Metadata } from "next";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "SDK Reference — Build WASM AI Extensions",
  description:
    "Build sandboxed Omni AI agent extensions with the Rust SDK. Learn tool definitions, host function bindings, manifest format, WASM compilation targets, and marketplace publishing workflow.",
  openGraph: {
    title: "Omni SDK Reference — Build WASM AI Agent Extensions in Rust",
    description:
      "Build sandboxed Omni extensions with the Rust SDK. Learn tool definitions, host function bindings, manifest format, WASM compilation, and marketplace publishing.",
    url: "/docs/sdk",
  },
  alternates: { canonical: "/docs/sdk" },
};

export default function SdkReferencePage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Developers
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">SDK Reference</h1>
          <p className="text-muted-foreground mb-12">
            Build extensions for Omni using the Rust SDK. Compile to WebAssembly and publish
            to the marketplace.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
              {[
                { href: "#overview", label: "Overview" },
                { href: "#quickstart", label: "Quick Start" },
                { href: "#manifest", label: "Manifest Format" },
                { href: "#extension-trait", label: "Extension Trait" },
                { href: "#context", label: "Context API" },
                { href: "#host-functions", label: "Host Functions" },
                { href: "#permissions", label: "Permissions" },
                { href: "#errors", label: "Error Handling" },
                { href: "#building", label: "Building & Testing" },
                { href: "#examples", label: "Complete Example" },
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
              Omni extensions are written in Rust and compiled to WebAssembly (WASM). They run
              in isolated sandboxes powered by{" "}
              <a href="https://wasmtime.dev" className="text-primary hover:underline">Wasmtime</a>{" "}
              with capability-based permissions — extensions can only access resources you explicitly grant them.
            </p>
            <p className="text-muted-foreground leading-relaxed mb-6">
              The <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">omni-sdk</code> crate
              provides typed clients for every host function, a macro for WASM entry-point generation, and
              re-exports of <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">serde</code> /
              {" "}<code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">serde_json</code> so
              you can focus on your extension&apos;s logic.
            </p>

            <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { label: "Language", value: "Rust" },
                { label: "Compile Target", value: "wasm32-wasip1" },
                { label: "Runtime", value: "Wasmtime (WASI P1)" },
              ].map((item) => (
                <div key={item.label} className="bg-card px-4 py-3">
                  <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-1">{item.label}</p>
                  <p className="text-sm font-medium">{item.value}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Quick Start */}
          <section className="mb-14" id="quickstart">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Quick Start</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Create a new Rust library project and add the Omni SDK:
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">terminal</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p>cargo new --lib my-extension</p>
                <p>cd my-extension</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Add the SDK dependency</p>
                <p>cargo add omni-sdk</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Add serde (re-exported by SDK, but needed for derive macros)</p>
                <p>cargo add serde --features derive</p>
                <p>cargo add serde_json</p>
              </div>
            </div>

            <p className="text-muted-foreground leading-relaxed mb-4">
              Set the crate type to <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">cdylib</code> in
              your <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Cargo.toml</code>:
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">Cargo.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[lib]</span></p>
                <p>crate-type = [<span className="text-success">&quot;cdylib&quot;</span>]</p>
              </div>
            </div>

            <p className="text-muted-foreground leading-relaxed mb-4">
              Write your first extension in <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">src/lib.rs</code>:
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">src/lib.rs</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-primary/70">use</span> omni_sdk::prelude::*;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">#[derive(Default)]</span></p>
                <p><span className="text-primary/70">struct</span> MyExtension;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">impl</span> Extension <span className="text-primary/70">for</span> MyExtension {"{"}</p>
                <p>    <span className="text-primary/70">fn</span> handle_tool(</p>
                <p>        &amp;<span className="text-primary/70">mut self</span>,</p>
                <p>        ctx: &amp;Context,</p>
                <p>        tool_name: &amp;<span className="text-primary/70">str</span>,</p>
                <p>        params: serde_json::Value,</p>
                <p>    ) -&gt; ToolResult {"{"}</p>
                <p>        <span className="text-primary/70">match</span> tool_name {"{"}</p>
                <p>            <span className="text-success">&quot;hello&quot;</span> =&gt; {"{"}</p>
                <p>                <span className="text-primary/70">let</span> name = params[<span className="text-success">&quot;name&quot;</span>]</p>
                <p>                    .as_str()</p>
                <p>                    .unwrap_or(<span className="text-success">&quot;world&quot;</span>);</p>
                <p>                Ok(serde_json::json!({"{"}</p>
                <p>                    <span className="text-success">&quot;message&quot;</span>: format!(<span className="text-success">&quot;Hello, {"{}"}!&quot;</span>, name)</p>
                <p>                {"}"}))</p>
                <p>            {"}"}</p>
                <p>            _ =&gt; Err(SdkError::UnknownTool(tool_name.into())),</p>
                <p>        {"}"}</p>
                <p>    {"}"}</p>
                <p>{"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// Generate the WASM entry point</span></p>
                <p>omni_sdk::omni_main!(MyExtension);</p>
              </div>
            </div>

            <p className="text-muted-foreground leading-relaxed mb-4">
              Build for the WASM target:
            </p>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">terminal</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> One-time setup</p>
                <p>rustup target add wasm32-wasip1</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Build</p>
                <p>cargo build --target wasm32-wasip1 --release</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Output: target/wasm32-wasip1/release/my_extension.wasm</p>
              </div>
            </div>
          </section>

          {/* Manifest */}
          <section className="mb-14" id="manifest">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Manifest Format</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Every extension needs an{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">omni-extension.toml</code>{" "}
              manifest file that describes the extension, its runtime constraints, permission requirements, and tool definitions.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni-extension.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[extension]</span></p>
                <p>id          = <span className="text-success">&quot;com.example.weather&quot;</span></p>
                <p>name        = <span className="text-success">&quot;Weather Tool&quot;</span></p>
                <p>version     = <span className="text-success">&quot;1.0.0&quot;</span></p>
                <p>author      = <span className="text-success">&quot;Your Name &lt;you@example.com&gt;&quot;</span></p>
                <p>description = <span className="text-success">&quot;Get current weather data for any city.&quot;</span></p>
                <p>license     = <span className="text-success">&quot;MIT&quot;</span></p>
                <p>categories  = [<span className="text-success">&quot;weather&quot;</span>, <span className="text-success">&quot;utilities&quot;</span>]</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[runtime]</span></p>
                <p>entrypoint          = <span className="text-success">&quot;weather.wasm&quot;</span></p>
                <p>max_memory_mb       = <span className="text-primary/70">64</span>         <span className="text-foreground/40"># default: 64 MB</span></p>
                <p>max_cpu_ms_per_call  = <span className="text-primary/70">5000</span>       <span className="text-foreground/40"># default: 5 seconds</span></p>
                <p>max_concurrent_calls = <span className="text-primary/70">4</span>          <span className="text-foreground/40"># default: 4</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[[permissions]]</span></p>
                <p>capability = <span className="text-success">&quot;network.http&quot;</span></p>
                <p>reason     = <span className="text-success">&quot;Fetch weather data from OpenWeatherMap API&quot;</span></p>
                <p>required   = <span className="text-primary/70">true</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[permissions.scope]</span></p>
                <p>domains = [<span className="text-success">&quot;api.openweathermap.org&quot;</span>]</p>
                <p>methods = [<span className="text-success">&quot;GET&quot;</span>]</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[[permissions]]</span></p>
                <p>capability = <span className="text-success">&quot;storage.persistent&quot;</span></p>
                <p>reason     = <span className="text-success">&quot;Cache weather responses&quot;</span></p>
                <p>required   = <span className="text-primary/70">false</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[[tools]]</span></p>
                <p>name        = <span className="text-success">&quot;get_weather&quot;</span></p>
                <p>description = <span className="text-success">&quot;Get current weather for a city&quot;</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[tools.parameters]</span></p>
                <p>type     = <span className="text-success">&quot;object&quot;</span></p>
                <p>required = [<span className="text-success">&quot;city&quot;</span>]</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[tools.parameters.properties.city]</span></p>
                <p>type        = <span className="text-success">&quot;string&quot;</span></p>
                <p>description = <span className="text-success">&quot;City name (e.g. London, Tokyo)&quot;</span></p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3 mt-8">Manifest Sections</h3>
            <div className="border border-border/50 rounded-lg overflow-hidden divide-y divide-border/50">
              {[
                {
                  key: "[extension]",
                  fields: "id, name, version, author, description",
                  desc: "Required metadata. ID uses reverse-domain format (min 5 chars, must contain a dot). Version must be valid semver. Optional fields: license, homepage, repository, icon, categories, min_omni_version.",
                },
                {
                  key: "[runtime]",
                  fields: "entrypoint, max_memory_mb, max_cpu_ms_per_call, max_concurrent_calls",
                  desc: "WASM sandbox configuration. entrypoint is the path to the .wasm file (required). Memory default is 64 MB, CPU timeout default is 5000 ms, concurrency default is 4. All limits must be greater than zero.",
                },
                {
                  key: "[[permissions]]",
                  fields: "capability, scope, reason, required",
                  desc: "Array of permission declarations. Each entry names a capability (e.g. network.http), explains why it's needed, and optionally constrains scope (allowed domains, paths, etc.). required defaults to true.",
                },
                {
                  key: "[[tools]]",
                  fields: "name, description, parameters",
                  desc: "Array of tool definitions exposed to the LLM. parameters is a JSON Schema object describing the tool's input. The LLM uses name and description to decide when to invoke the tool.",
                },
                {
                  key: "[config]",
                  fields: "fields.{name}.type, label, help, sensitive, required, default, options",
                  desc: "Optional configuration schema. Defines fields the user can set through the UI (e.g. API keys, preferences). Sensitive fields are hidden in the UI. Extensions read config values via ctx.config().get(key).",
                },
                {
                  key: "[hooks]",
                  fields: "on_install, on_message, on_schedule",
                  desc: "Optional lifecycle hooks. on_install and on_message are booleans. on_schedule accepts a cron expression (e.g. \"0 */6 * * *\").",
                },
              ].map((s) => (
                <div key={s.key} className="p-4 bg-card">
                  <div className="flex items-baseline gap-3 mb-1.5">
                    <code className="text-sm font-mono text-primary shrink-0">{s.key}</code>
                    <span className="text-[11px] font-mono text-muted-foreground/50">{s.fields}</span>
                  </div>
                  <p className="text-sm text-muted-foreground leading-relaxed">{s.desc}</p>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3 mt-8">Extension ID Rules</h3>
            <div className="border border-border/50 rounded-lg p-4 bg-card/30 space-y-2">
              {[
                "Must use reverse-domain format (e.g. com.example.my-tool)",
                "Minimum 5 characters, must contain at least one dot",
                "No leading, trailing, or consecutive dots",
                "No double-dot sequences (..)",
              ].map((rule, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground/50 w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{rule}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Extension Trait */}
          <section className="mb-14" id="extension-trait">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Extension Trait &amp; Entry Point</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Every extension implements the{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Extension</code> trait
              and uses the{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">omni_main!</code> macro
              to generate the WASM entry point. Your struct must implement{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Default</code> since the
              runtime creates the instance on first tool call.
            </p>

            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">Extension trait</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-primary/70">pub trait</span> Extension {"{"}</p>
                <p>    <span className="text-primary/70">fn</span> handle_tool(</p>
                <p>        &amp;<span className="text-primary/70">mut self</span>,</p>
                <p>        ctx: &amp;Context,</p>
                <p>        tool_name: &amp;<span className="text-primary/70">str</span>,</p>
                <p>        params: serde_json::Value,</p>
                <p>    ) -&gt; ToolResult;</p>
                <p>{"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// ToolResult = Result&lt;serde_json::Value, SdkError&gt;</span></p>
              </div>
            </div>

            <p className="text-muted-foreground leading-relaxed mb-4">
              The <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">handle_tool</code> method
              receives the tool name as a string and parameters as a JSON value. Match on the tool name to dispatch
              to the correct handler. Return a{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">serde_json::Value</code> on
              success — the runtime serializes it back to the host.
            </p>

            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">multi-tool extension</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-primary/70">use</span> omni_sdk::prelude::*;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">#[derive(Default)]</span></p>
                <p><span className="text-primary/70">struct</span> FileTools;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">impl</span> Extension <span className="text-primary/70">for</span> FileTools {"{"}</p>
                <p>    <span className="text-primary/70">fn</span> handle_tool(</p>
                <p>        &amp;<span className="text-primary/70">mut self</span>,</p>
                <p>        ctx: &amp;Context,</p>
                <p>        tool_name: &amp;<span className="text-primary/70">str</span>,</p>
                <p>        params: serde_json::Value,</p>
                <p>    ) -&gt; ToolResult {"{"}</p>
                <p>        <span className="text-primary/70">match</span> tool_name {"{"}</p>
                <p>            <span className="text-success">&quot;read_file&quot;</span>  =&gt; self.read_file(ctx, params),</p>
                <p>            <span className="text-success">&quot;write_file&quot;</span> =&gt; self.write_file(ctx, params),</p>
                <p>            <span className="text-success">&quot;list_dir&quot;</span>   =&gt; self.list_dir(ctx, params),</p>
                <p>            _ =&gt; Err(SdkError::UnknownTool(tool_name.into())),</p>
                <p>        {"}"}</p>
                <p>    {"}"}</p>
                <p>{"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">impl</span> FileTools {"{"}</p>
                <p>    <span className="text-primary/70">fn</span> read_file(&amp;self, ctx: &amp;Context, params: serde_json::Value) -&gt; ToolResult {"{"}</p>
                <p>        <span className="text-primary/70">let</span> path = params[<span className="text-success">&quot;path&quot;</span>].as_str()</p>
                <p>            .ok_or_else(|| SdkError::Other(<span className="text-success">&quot;missing path&quot;</span>.into()))?;</p>
                <p>        <span className="text-primary/70">let</span> content = ctx.fs().read_string(path)?;</p>
                <p>        Ok(serde_json::json!({"{"} <span className="text-success">&quot;content&quot;</span>: content {"}"}))</p>
                <p>    {"}"}</p>
                <p>    <span className="text-foreground/40">// ... other handlers</span></p>
                <p>{"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>omni_sdk::omni_main!(FileTools);</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">omni_main! Macro</h3>
            <p className="text-sm text-muted-foreground leading-relaxed">
              The <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">omni_main!</code> macro
              generates a <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">#[no_mangle] pub extern &quot;C&quot; fn handle_tool</code> function
              that the Wasmtime runtime calls. It creates a static extension instance via{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Default::default()</code> on
              first invocation, reads the tool name and parameters from WASM linear memory, calls your{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">handle_tool</code> implementation,
              and writes the result back to memory using a packed pointer encoding.
            </p>
          </section>

          {/* Context API */}
          <section className="mb-14" id="context">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Context API</h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Every tool invocation receives a{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Context</code> reference.
              The Context provides typed clients for every host function. Each client method that
              accesses a protected resource requires the corresponding permission in your manifest.
            </p>

            <div className="border border-border/50 rounded-lg overflow-hidden divide-y divide-border/50 mb-6">
              {[
                {
                  method: "ctx.http()",
                  returns: "HttpClient",
                  permission: "network.http",
                  desc: "Make HTTP requests (GET, POST, PUT, DELETE). Returns HttpResponse with status and body.",
                },
                {
                  method: "ctx.fs()",
                  returns: "FsClient",
                  permission: "filesystem.read / filesystem.write",
                  desc: "Read and write files on the host filesystem. Supports both raw bytes and string content.",
                },
                {
                  method: "ctx.process()",
                  returns: "ProcessClient",
                  permission: "process.spawn",
                  desc: "Execute commands and capture stdout/stderr. Returns ProcessOutput with exit code.",
                },
                {
                  method: "ctx.storage()",
                  returns: "StorageClient",
                  permission: "none",
                  desc: "Extension-scoped persistent key-value store. Always available, no permission needed.",
                },
                {
                  method: "ctx.llm()",
                  returns: "LlmClient",
                  permission: "ai.inference",
                  desc: "Send prompts to the user's configured LLM provider. Returns response text.",
                },
                {
                  method: "ctx.channels()",
                  returns: "ChannelClient",
                  permission: "channel.send",
                  desc: "Send messages through connected channels (Discord, Telegram, Slack, etc.).",
                },
                {
                  method: "ctx.config()",
                  returns: "ConfigClient",
                  permission: "none",
                  desc: "Read extension config values set by the user. Always available, no permission needed.",
                },
                {
                  method: "ctx.log() / .info() / .warn() / .error() / .debug()",
                  returns: "()",
                  permission: "none",
                  desc: "Structured logging at four levels (Error, Warn, Info, Debug). Always available.",
                },
              ].map((item, i) => (
                <div key={i} className="p-4 bg-card">
                  <div className="flex items-baseline justify-between gap-4 mb-1.5">
                    <code className="text-[13px] font-mono text-primary">{item.method}</code>
                    <span className="text-[11px] font-mono text-muted-foreground/50 shrink-0">{item.permission}</span>
                  </div>
                  <p className="text-sm text-muted-foreground">
                    <span className="text-foreground/60 font-mono text-xs mr-1.5">&rarr; {item.returns}</span>
                    {item.desc}
                  </p>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">HttpClient</h3>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">HTTP examples</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">// Simple GET request</span></p>
                <p><span className="text-primary/70">let</span> resp = ctx.http().get(<span className="text-success">&quot;https://api.example.com/data&quot;</span>)?;</p>
                <p><span className="text-primary/70">let</span> body = resp.text()?;            <span className="text-foreground/40">// String</span></p>
                <p><span className="text-primary/70">let</span> data: MyType = resp.json()?;    <span className="text-foreground/40">// Deserialize</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// POST with JSON body and custom headers</span></p>
                <p><span className="text-primary/70">let</span> resp = ctx.http()</p>
                <p>    .post(<span className="text-success">&quot;https://api.example.com/submit&quot;</span>)</p>
                <p>    .header(<span className="text-success">&quot;Authorization&quot;</span>, <span className="text-success">&quot;Bearer token123&quot;</span>)</p>
                <p>    .json(&amp;my_data)?</p>
                <p>    .send()?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// PUT and DELETE also available</span></p>
                <p><span className="text-primary/70">let</span> resp = ctx.http().delete(<span className="text-success">&quot;https://api.example.com/item/42&quot;</span>)?;</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">FsClient</h3>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">filesystem examples</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">// Read file as string</span></p>
                <p><span className="text-primary/70">let</span> content = ctx.fs().read_string(<span className="text-success">&quot;/path/to/file.txt&quot;</span>)?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// Read file as raw bytes</span></p>
                <p><span className="text-primary/70">let</span> bytes = ctx.fs().read(<span className="text-success">&quot;/path/to/image.png&quot;</span>)?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// Write string content</span></p>
                <p>ctx.fs().write_string(<span className="text-success">&quot;/path/to/output.md&quot;</span>, &amp;markdown)?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// Write raw bytes</span></p>
                <p>ctx.fs().write(<span className="text-success">&quot;/path/to/data.bin&quot;</span>, &amp;bytes)?;</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">ProcessClient</h3>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">process example</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-primary/70">let</span> output = ctx.process().exec(<span className="text-success">&quot;git&quot;</span>, &amp;[<span className="text-success">&quot;log&quot;</span>, <span className="text-success">&quot;--oneline&quot;</span>, <span className="text-success">&quot;-5&quot;</span>])?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">if</span> output.exit_code == 0 {"{"}</p>
                <p>    ctx.info(&amp;format!(<span className="text-success">&quot;git output: {"{}}"}&quot;</span>, output.stdout));</p>
                <p>{"}"} <span className="text-primary/70">else</span> {"{"}</p>
                <p>    ctx.error(&amp;format!(<span className="text-success">&quot;git failed: {"{}}"}&quot;</span>, output.stderr));</p>
                <p>{"}"}</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">LlmClient &amp; ChannelClient</h3>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">LLM and channel examples</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">// LLM inference — 0 means use provider's default max tokens</span></p>
                <p><span className="text-primary/70">let</span> response = ctx.llm().request(<span className="text-success">&quot;Summarize this text: ...&quot;</span>, 0)?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// LLM with explicit token limit</span></p>
                <p><span className="text-primary/70">let</span> response = ctx.llm().request(<span className="text-success">&quot;Translate to French: hello&quot;</span>, 200)?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// Send a message through a connected channel</span></p>
                <p><span className="text-foreground/40">// Channel ID uses compound format: type:instance_id</span></p>
                <p><span className="text-primary/70">let</span> result = ctx.channels().send(</p>
                <p>    <span className="text-success">&quot;discord:production&quot;</span>,  <span className="text-foreground/40">// channel_id</span></p>
                <p>    <span className="text-success">&quot;#general&quot;</span>,              <span className="text-foreground/40">// recipient</span></p>
                <p>    <span className="text-success">&quot;Build deployed!&quot;</span>,       <span className="text-foreground/40">// message text</span></p>
                <p>)?;</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">StorageClient &amp; ConfigClient</h3>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">storage and config examples</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">// Persistent key-value storage (extension-scoped)</span></p>
                <p>ctx.storage().set(<span className="text-success">&quot;last_run&quot;</span>, <span className="text-success">&quot;2025-01-15T10:30:00Z&quot;</span>)?;</p>
                <p><span className="text-primary/70">let</span> value = ctx.storage().get(<span className="text-success">&quot;last_run&quot;</span>)?;  <span className="text-foreground/40">// Option&lt;String&gt;</span></p>
                <p>ctx.storage().delete(<span className="text-success">&quot;old_key&quot;</span>)?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">// Read user-set configuration values</span></p>
                <p><span className="text-primary/70">let</span> api_key = ctx.config().get(<span className="text-success">&quot;api_key&quot;</span>)?;        <span className="text-foreground/40">// Option&lt;String&gt;</span></p>
                <p><span className="text-primary/70">let</span> theme = ctx.config().get_or(<span className="text-success">&quot;theme&quot;</span>, <span className="text-success">&quot;dark&quot;</span>)?; <span className="text-foreground/40">// String (with default)</span></p>
              </div>
            </div>
          </section>

          {/* Host Functions */}
          <section className="mb-14" id="host-functions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Host Functions</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Host functions are provided by the Omni runtime and bound into the WASM sandbox
              under the <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">omni</code> import
              module. The SDK wraps these in typed clients — you should use the Context API above
              rather than calling FFI functions directly.
            </p>

            <div className="border border-border/50 rounded-lg overflow-hidden divide-y divide-border/50">
              {[
                { name: "log", cap: "none", desc: "Log a message at Error/Warn/Info/Debug level. Always available." },
                { name: "storage_get", cap: "none", desc: "Read a value from extension-scoped persistent storage." },
                { name: "storage_set", cap: "none", desc: "Write a value to extension-scoped persistent storage." },
                { name: "http_request", cap: "network.http", desc: "Make an HTTP request (GET/POST/PUT/DELETE/PATCH/HEAD). 30s timeout, 5 MB response limit." },
                { name: "fs_read", cap: "filesystem.read", desc: "Read a file from the host filesystem. 10 MB file size limit." },
                { name: "fs_write", cap: "filesystem.write", desc: "Write data to a file on the host filesystem. Creates parent directories if needed." },
                { name: "process_spawn", cap: "process.spawn", desc: "Execute a command with arguments and capture stdout/stderr. Output capped at 50 KB each." },
                { name: "llm_request", cap: "ai.inference", desc: "Send a prompt to the user's configured LLM provider and receive the response." },
                { name: "channel_send", cap: "channel.send", desc: "Send a text message through a connected channel plugin." },
                { name: "config_get", cap: "none", desc: "Read an extension configuration value set by the user. No permission required." },
              ].map((fn_, i) => (
                <div
                  key={fn_.name}
                  className={`flex items-start gap-4 p-4 bg-card`}
                >
                  <code className="text-xs font-mono text-primary shrink-0 w-28 pt-0.5">{fn_.name}</code>
                  <p className="text-sm text-muted-foreground flex-1">{fn_.desc}</p>
                  <span className="text-[11px] font-mono text-muted-foreground/50 shrink-0">{fn_.cap}</span>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3 mt-6">Return Code Convention</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Permission-gated host functions use a consistent return code scheme. The SDK clients
              handle these automatically and return typed errors:
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden text-sm">
              {[
                { code: "-1", meaning: "Permission denied" },
                { code: "-2", meaning: "Needs user prompt" },
                { code: "-3", meaning: "Operation failed" },
                { code: "-4", meaning: "No callback set" },
              ].map((item) => (
                <div key={item.code} className="bg-card px-4 py-3">
                  <code className="font-mono text-primary text-xs">{item.code}</code>
                  <p className="text-xs text-muted-foreground mt-0.5">{item.meaning}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Permissions */}
          <section className="mb-14" id="permissions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Permissions</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Omni uses a deny-by-default capability system. Extensions must declare every
              capability they need in their manifest, and users must grant each one before it
              takes effect. Scope constraints let you limit access to specific domains, paths,
              or executables.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Available Capabilities</h3>
            <div className="border border-border/50 rounded-lg overflow-hidden divide-y divide-border/50 mb-6">
              {[
                { cap: "network.http", scope: "domains, methods, ports", desc: "HTTP requests to allowed domains" },
                { cap: "network.websocket", scope: "domains", desc: "WebSocket connections" },
                { cap: "filesystem.read", scope: "paths, extensions, max_size", desc: "Read files from the host filesystem" },
                { cap: "filesystem.write", scope: "paths, extensions, max_size", desc: "Write files to the host filesystem" },
                { cap: "process.spawn", scope: "executables, allowed_args, denied_args, max_concurrent", desc: "Execute commands on the host" },
                { cap: "storage.persistent", scope: "max_bytes", desc: "Persistent key-value storage" },
                { cap: "ai.inference", scope: "max_tokens, rate_limit", desc: "LLM inference requests" },
                { cap: "channel.send", scope: "channels, rate_limit", desc: "Send messages through channels" },
                { cap: "browser.scrape", scope: "domains, max_pages", desc: "Scrape web content via browser" },
                { cap: "search.web", scope: "providers, rate_limit", desc: "Web search queries" },
                { cap: "messaging.sms", scope: "recipients, rate_limit", desc: "Send SMS messages" },
                { cap: "messaging.email", scope: "recipients, rate_limit", desc: "Send email messages" },
                { cap: "messaging.chat", scope: "recipients, rate_limit", desc: "Send chat messages" },
                { cap: "clipboard.read", scope: "none", desc: "Read clipboard contents" },
                { cap: "clipboard.write", scope: "none", desc: "Write to clipboard" },
                { cap: "system.notifications", scope: "none", desc: "Show system notifications" },
                { cap: "system.scheduling", scope: "none", desc: "Schedule recurring/one-time tasks (cron)" },
                { cap: "device.camera", scope: "none", desc: "Access device camera" },
                { cap: "device.microphone", scope: "none", desc: "Access device microphone" },
                { cap: "device.location", scope: "none", desc: "Access device location" },
              ].map((item) => (
                <div key={item.cap} className="flex items-start gap-4 p-3.5 bg-card">
                  <code className="text-xs font-mono text-primary shrink-0 w-36 pt-0.5">{item.cap}</code>
                  <p className="text-sm text-muted-foreground flex-1">{item.desc}</p>
                  <span className="text-[11px] font-mono text-muted-foreground/40 shrink-0">{item.scope}</span>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Scope Examples</h3>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni-extension.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40"># HTTP: restrict to specific domains and methods</span></p>
                <p><span className="text-foreground/60">[[permissions]]</span></p>
                <p>capability = <span className="text-success">&quot;network.http&quot;</span></p>
                <p>reason     = <span className="text-success">&quot;Fetch data from GitHub API&quot;</span></p>
                <p><span className="text-foreground/60">[permissions.scope]</span></p>
                <p>domains = [<span className="text-success">&quot;api.github.com&quot;</span>, <span className="text-success">&quot;*.githubusercontent.com&quot;</span>]</p>
                <p>methods = [<span className="text-success">&quot;GET&quot;</span>]</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40"># Filesystem: restrict to specific paths and file types</span></p>
                <p><span className="text-foreground/60">[[permissions]]</span></p>
                <p>capability = <span className="text-success">&quot;filesystem.read&quot;</span></p>
                <p>reason     = <span className="text-success">&quot;Read markdown files from Documents&quot;</span></p>
                <p><span className="text-foreground/60">[permissions.scope]</span></p>
                <p>paths      = [<span className="text-success">&quot;~/Documents&quot;</span>]</p>
                <p>extensions = [<span className="text-success">&quot;.md&quot;</span>, <span className="text-success">&quot;.txt&quot;</span>]</p>
                <p>max_size   = <span className="text-primary/70">10000000</span>  <span className="text-foreground/40"># 10 MB</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40"># Process: whitelist specific executables</span></p>
                <p><span className="text-foreground/60">[[permissions]]</span></p>
                <p>capability = <span className="text-success">&quot;process.spawn&quot;</span></p>
                <p>reason     = <span className="text-success">&quot;Run git commands&quot;</span></p>
                <p><span className="text-foreground/60">[permissions.scope]</span></p>
                <p>executables = [<span className="text-success">&quot;git&quot;</span>]</p>
                <p>denied_args = [<span className="text-success">&quot;push.*--force&quot;</span>]  <span className="text-foreground/40"># regex patterns</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40"># AI: limit token usage and rate</span></p>
                <p><span className="text-foreground/60">[[permissions]]</span></p>
                <p>capability = <span className="text-success">&quot;ai.inference&quot;</span></p>
                <p>reason     = <span className="text-success">&quot;Summarize file contents&quot;</span></p>
                <p><span className="text-foreground/60">[permissions.scope]</span></p>
                <p>max_tokens = <span className="text-primary/70">4000</span></p>
                <p>rate_limit = <span className="text-primary/70">60</span>  <span className="text-foreground/40"># per minute</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40"># Channel: restrict to specific instances</span></p>
                <p><span className="text-foreground/60">[[permissions]]</span></p>
                <p>capability = <span className="text-success">&quot;channel.send&quot;</span></p>
                <p>reason     = <span className="text-success">&quot;Send notifications to Discord&quot;</span></p>
                <p><span className="text-foreground/60">[permissions.scope]</span></p>
                <p>channels   = [<span className="text-success">&quot;discord:production&quot;</span>]</p>
                <p>rate_limit = <span className="text-primary/70">10</span></p>
              </div>
            </div>
          </section>

          {/* Error Handling */}
          <section className="mb-14" id="errors">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Error Handling</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              The SDK provides an{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">SdkError</code> enum
              for all error conditions. Tool functions return{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">ToolResult</code> which is{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Result&lt;serde_json::Value, SdkError&gt;</code>.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni_sdk::SdkError</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-primary/70">pub enum</span> SdkError {"{"}</p>
                <p>    UnknownTool(String),      <span className="text-foreground/40">// Tool name not recognized</span></p>
                <p>    Serde(String),            <span className="text-foreground/40">// JSON serialization error</span></p>
                <p>    PermissionDenied(String),  <span className="text-foreground/40">// Capability not granted</span></p>
                <p>    HttpError(String),         <span className="text-foreground/40">// HTTP request failed</span></p>
                <p>    StorageError(String),      <span className="text-foreground/40">// Storage operation failed</span></p>
                <p>    FsError(String),           <span className="text-foreground/40">// Filesystem operation failed</span></p>
                <p>    ProcessError(String),      <span className="text-foreground/40">// Process execution failed</span></p>
                <p>    LlmError(String),          <span className="text-foreground/40">// LLM inference failed</span></p>
                <p>    ChannelError(String),      <span className="text-foreground/40">// Channel send failed</span></p>
                <p>    NotAvailable(String),      <span className="text-foreground/40">// No callback configured</span></p>
                <p>    Other(String),             <span className="text-foreground/40">// Generic error</span></p>
                <p>{"}"}</p>
              </div>
            </div>

            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">SdkError</code> implements{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">Display</code> with descriptive
              messages for each variant. Use the <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">?</code> operator
              to propagate errors naturally:
            </p>
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">error handling patterns</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">// The ? operator works naturally with ToolResult</span></p>
                <p><span className="text-primary/70">fn</span> my_tool(ctx: &amp;Context, params: serde_json::Value) -&gt; ToolResult {"{"}</p>
                <p>    <span className="text-primary/70">let</span> url = params[<span className="text-success">&quot;url&quot;</span>].as_str()</p>
                <p>        .ok_or_else(|| SdkError::Other(<span className="text-success">&quot;missing url parameter&quot;</span>.into()))?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>    <span className="text-primary/70">let</span> resp = ctx.http().get(url)?;   <span className="text-foreground/40">// HttpError or PermissionDenied</span></p>
                <p>    <span className="text-primary/70">let</span> text = resp.text()?;           <span className="text-foreground/40">// Serde error</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>    Ok(serde_json::json!({"{"} <span className="text-success">&quot;content&quot;</span>: text {"}"}))</p>
                <p>{"}"}</p>
              </div>
            </div>
          </section>

          {/* Building */}
          <section className="mb-14" id="building">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Building &amp; Testing</h2>

            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">build commands</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> One-time: add the WASM target</p>
                <p>rustup target add wasm32-wasip1</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Debug build</p>
                <p>cargo build --target wasm32-wasip1</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Release build (use for publishing)</p>
                <p>cargo build --target wasm32-wasip1 --release</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Optional: optimize with wasm-opt (binaryen)</p>
                <p>wasm-opt -Oz target/wasm32-wasip1/release/my_extension.wasm -o my_extension.wasm</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Testing Locally</h3>
            <div className="space-y-3 mb-6">
              {[
                "Build the WASM binary in release mode",
                "Place the .wasm file and omni-extension.toml in a directory together",
                "Copy the directory into Omni's extensions folder (see paths below)",
                "Restart Omni — extension discovery runs on startup",
                "Review and grant permissions when prompted, then test your tools",
              ].map((step, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground/50 w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{step}</p>
                </div>
              ))}
            </div>

            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">extensions directory</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> Place extensions under the &quot;user&quot; subdirectory:</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Windows: %APPDATA%/com.omni.agent/extensions/user/</p>
                <p><span className="text-foreground/40">#</span> macOS:   ~/Library/Application Support/com.omni.agent/extensions/user/</p>
                <p><span className="text-foreground/40">#</span> Linux:   ~/.local/share/com.omni.agent/extensions/user/</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Directory structure:</p>
                <p><span className="text-foreground/40">#</span>   extensions/user/com.example.weather/</p>
                <p><span className="text-foreground/40">#</span>     ├── omni-extension.toml</p>
                <p><span className="text-foreground/40">#</span>     └── weather.wasm</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Sandbox Limits</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Extensions run in a Wasmtime sandbox with enforced resource limits. If an extension
              exceeds its CPU timeout, Wasmtime triggers an epoch interrupt and the call returns a
              timeout error. Memory limits are enforced via{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">StoreLimits</code>.
              Concurrency is controlled by a per-extension semaphore.
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { label: "Memory", default: "64 MB", field: "max_memory_mb" },
                { label: "CPU Timeout", default: "5,000 ms", field: "max_cpu_ms_per_call" },
                { label: "Concurrency", default: "4 parallel", field: "max_concurrent_calls" },
              ].map((item) => (
                <div key={item.label} className="bg-card px-4 py-3">
                  <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-1">{item.label}</p>
                  <p className="text-sm font-medium">{item.default}</p>
                  <code className="text-[11px] font-mono text-muted-foreground/50">{item.field}</code>
                </div>
              ))}
            </div>
          </section>

          {/* Prelude */}
          <section className="mb-14" id="prelude">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Prelude</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Import everything you need with a single{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">use omni_sdk::prelude::*</code> statement.
              The prelude re-exports all types, clients, and{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">serde</code> /
              {" "}<code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">serde_json</code>:
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden text-sm">
              {[
                { name: "Context", kind: "struct" },
                { name: "Extension", kind: "trait" },
                { name: "ToolResult", kind: "type alias" },
                { name: "SdkError", kind: "enum" },
                { name: "LogLevel", kind: "enum" },
                { name: "HttpClient", kind: "struct" },
                { name: "HttpResponse", kind: "struct" },
                { name: "RequestBuilder", kind: "struct" },
                { name: "FsClient", kind: "struct" },
                { name: "ProcessClient", kind: "struct" },
                { name: "ProcessOutput", kind: "struct" },
                { name: "StorageClient", kind: "struct" },
                { name: "LlmClient", kind: "struct" },
                { name: "ChannelClient", kind: "struct" },
                { name: "ConfigClient", kind: "struct" },
                { name: "Serialize", kind: "serde" },
                { name: "Deserialize", kind: "serde" },
                { name: "serde_json", kind: "module" },
              ].map((item) => (
                <div key={item.name} className="bg-card px-4 py-2.5 flex items-baseline justify-between gap-2">
                  <code className="font-mono text-primary text-xs">{item.name}</code>
                  <span className="text-[10px] font-mono text-muted-foreground/40">{item.kind}</span>
                </div>
              ))}
            </div>
          </section>

          {/* Complete Example */}
          <section id="examples">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Complete Example</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              A complete extension that fetches weather data, uses configuration for the API key,
              caches results in storage, and logs its activity:
            </p>

            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni-extension.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[extension]</span></p>
                <p>id          = <span className="text-success">&quot;com.example.weather&quot;</span></p>
                <p>name        = <span className="text-success">&quot;Weather Tool&quot;</span></p>
                <p>version     = <span className="text-success">&quot;1.0.0&quot;</span></p>
                <p>author      = <span className="text-success">&quot;Jane Doe &lt;jane@example.com&gt;&quot;</span></p>
                <p>description = <span className="text-success">&quot;Get current weather data for any city.&quot;</span></p>
                <p>license     = <span className="text-success">&quot;MIT&quot;</span></p>
                <p>categories  = [<span className="text-success">&quot;weather&quot;</span>, <span className="text-success">&quot;utilities&quot;</span>]</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[runtime]</span></p>
                <p>entrypoint = <span className="text-success">&quot;weather.wasm&quot;</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[[permissions]]</span></p>
                <p>capability = <span className="text-success">&quot;network.http&quot;</span></p>
                <p>reason     = <span className="text-success">&quot;Fetch weather data from OpenWeatherMap&quot;</span></p>
                <p><span className="text-foreground/60">[permissions.scope]</span></p>
                <p>domains = [<span className="text-success">&quot;api.openweathermap.org&quot;</span>]</p>
                <p>methods = [<span className="text-success">&quot;GET&quot;</span>]</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[config.fields.api_key]</span></p>
                <p>type      = <span className="text-success">&quot;string&quot;</span></p>
                <p>label     = <span className="text-success">&quot;OpenWeatherMap API Key&quot;</span></p>
                <p>sensitive = <span className="text-primary/70">true</span></p>
                <p>required  = <span className="text-primary/70">true</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/60">[[tools]]</span></p>
                <p>name        = <span className="text-success">&quot;get_weather&quot;</span></p>
                <p>description = <span className="text-success">&quot;Get current weather for a city&quot;</span></p>
                <p><span className="text-foreground/60">[tools.parameters]</span></p>
                <p>type     = <span className="text-success">&quot;object&quot;</span></p>
                <p>required = [<span className="text-success">&quot;city&quot;</span>]</p>
                <p><span className="text-foreground/60">[tools.parameters.properties.city]</span></p>
                <p>type        = <span className="text-success">&quot;string&quot;</span></p>
                <p>description = <span className="text-success">&quot;City name (e.g. London, Tokyo)&quot;</span></p>
              </div>
            </div>

            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">src/lib.rs</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-primary/70">use</span> omni_sdk::prelude::*;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">#[derive(Deserialize)]</span></p>
                <p><span className="text-primary/70">struct</span> WeatherResponse {"{"}</p>
                <p>    main: MainData,</p>
                <p>    weather: Vec&lt;WeatherInfo&gt;,</p>
                <p>    name: String,</p>
                <p>{"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">#[derive(Deserialize)]</span></p>
                <p><span className="text-primary/70">struct</span> MainData {"{"} temp: f64, humidity: u32 {"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">#[derive(Deserialize)]</span></p>
                <p><span className="text-primary/70">struct</span> WeatherInfo {"{"} description: String {"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">#[derive(Default)]</span></p>
                <p><span className="text-primary/70">struct</span> WeatherExtension;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary/70">impl</span> Extension <span className="text-primary/70">for</span> WeatherExtension {"{"}</p>
                <p>    <span className="text-primary/70">fn</span> handle_tool(</p>
                <p>        &amp;<span className="text-primary/70">mut self</span>,</p>
                <p>        ctx: &amp;Context,</p>
                <p>        tool_name: &amp;<span className="text-primary/70">str</span>,</p>
                <p>        params: serde_json::Value,</p>
                <p>    ) -&gt; ToolResult {"{"}</p>
                <p>        <span className="text-primary/70">match</span> tool_name {"{"}</p>
                <p>            <span className="text-success">&quot;get_weather&quot;</span> =&gt; {"{"}</p>
                <p>                <span className="text-primary/70">let</span> city = params[<span className="text-success">&quot;city&quot;</span>].as_str()</p>
                <p>                    .ok_or_else(|| SdkError::Other(<span className="text-success">&quot;missing city&quot;</span>.into()))?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>                <span className="text-foreground/40">// Read API key from user configuration</span></p>
                <p>                <span className="text-primary/70">let</span> api_key = ctx.config().get(<span className="text-success">&quot;api_key&quot;</span>)?</p>
                <p>                    .ok_or_else(|| SdkError::Other(<span className="text-success">&quot;api_key not configured&quot;</span>.into()))?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>                <span className="text-foreground/40">// Check cache first</span></p>
                <p>                <span className="text-primary/70">let</span> cache_key = format!(<span className="text-success">&quot;weather_{"{}}"}&quot;</span>, city.to_lowercase());</p>
                <p>                <span className="text-primary/70">if let</span> Some(cached) = ctx.storage().get(&amp;cache_key)? {"{"}</p>
                <p>                    ctx.debug(<span className="text-success">&quot;returning cached weather data&quot;</span>);</p>
                <p>                    <span className="text-primary/70">let</span> v: serde_json::Value = serde_json::from_str(&amp;cached)</p>
                <p>                        .map_err(|e| SdkError::Serde(e.to_string()))?;</p>
                <p>                    <span className="text-primary/70">return</span> Ok(v);</p>
                <p>                {"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>                <span className="text-foreground/40">// Fetch from API</span></p>
                <p>                ctx.info(&amp;format!(<span className="text-success">&quot;fetching weather for {"{}}"}&quot;</span>, city));</p>
                <p>                <span className="text-primary/70">let</span> url = format!(</p>
                <p>                    <span className="text-success">&quot;https://api.openweathermap.org/data/2.5/weather?q={"{}"}&amp;appid={"{}"}&amp;units=metric&quot;</span>,</p>
                <p>                    city, api_key</p>
                <p>                );</p>
                <p>                <span className="text-primary/70">let</span> resp = ctx.http().get(&amp;url)?;</p>
                <p>                <span className="text-primary/70">let</span> weather: WeatherResponse = resp.json()?;</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>                <span className="text-primary/70">let</span> result = serde_json::json!({"{"}</p>
                <p>                    <span className="text-success">&quot;city&quot;</span>: weather.name,</p>
                <p>                    <span className="text-success">&quot;temperature&quot;</span>: weather.main.temp,</p>
                <p>                    <span className="text-success">&quot;humidity&quot;</span>: weather.main.humidity,</p>
                <p>                    <span className="text-success">&quot;description&quot;</span>: weather.weather.first()</p>
                <p>                        .map(|w| w.description.as_str()).unwrap_or(<span className="text-success">&quot;unknown&quot;</span>),</p>
                <p>                {"}"});</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>                <span className="text-foreground/40">// Cache the result</span></p>
                <p>                <span className="text-primary/70">let</span> _ = ctx.storage().set(</p>
                <p>                    &amp;cache_key,</p>
                <p>                    &amp;serde_json::to_string(&amp;result).unwrap_or_default(),</p>
                <p>                );</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>                Ok(result)</p>
                <p>            {"}"}</p>
                <p>            _ =&gt; Err(SdkError::UnknownTool(tool_name.into())),</p>
                <p>        {"}"}</p>
                <p>    {"}"}</p>
                <p>{"}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>omni_sdk::omni_main!(WeatherExtension);</p>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
