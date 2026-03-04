import type { Metadata } from "next";
import Link from "next/link";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "LLM Providers — OpenAI, Anthropic & More",
  description:
    "Configure LLM providers in the Omni AI agent including OpenAI, Anthropic Claude, Google Gemini, Ollama, AWS Bedrock, and custom HTTP endpoints. Multi-provider routing and failover supported.",
  openGraph: {
    title: "Omni LLM Providers — OpenAI, Anthropic, Gemini & More",
    description:
      "Configure LLM providers in the Omni AI agent including OpenAI, Anthropic Claude, Google Gemini, Ollama, AWS Bedrock, and custom HTTP endpoints with multi-provider routing.",
    url: "/docs/providers",
  },
  alternates: { canonical: "/docs/providers" },
};

export default function ProvidersPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Guide
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">
            LLM Providers
          </h1>
          <p className="text-muted-foreground mb-12">
            Connect Omni to one or more LLM providers. Use cloud APIs, run models locally,
            or bring your own endpoint.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
              {[
                { href: "#overview", label: "Overview" },
                { href: "#openai", label: "OpenAI" },
                { href: "#anthropic", label: "Anthropic" },
                { href: "#gemini", label: "Google Gemini" },
                { href: "#ollama", label: "Ollama" },
                { href: "#bedrock", label: "AWS Bedrock" },
                { href: "#custom", label: "Custom HTTP" },
                { href: "#rotation", label: "Provider Rotation" },
                { href: "#streaming", label: "Streaming" },
                { href: "#tokens", label: "Token Counting" },
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
              Omni&apos;s LLM bridge is a provider-agnostic abstraction layer. You configure one or
              more providers, and Omni handles streaming, token counting, and automatic failover.
              All providers support tool calling (function calling), which is how the agent invokes
              native tools and extension tools.
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                { label: "OpenAI", value: "GPT-4o, GPT-4, o1, etc." },
                { label: "Anthropic", value: "Claude Opus, Sonnet, Haiku" },
                { label: "Google Gemini", value: "Gemini Pro, Ultra, Flash" },
                { label: "Ollama", value: "Llama, Mistral, Phi, etc." },
                { label: "AWS Bedrock", value: "Claude, Titan, Llama via AWS" },
                { label: "Custom HTTP", value: "Any OpenAI-compatible API" },
              ].map((item) => (
                <div key={item.label} className="bg-card px-4 py-3">
                  <p className="text-sm font-medium mb-0.5">{item.label}</p>
                  <p className="text-xs text-muted-foreground">{item.value}</p>
                </div>
              ))}
            </div>
          </section>

          {/* OpenAI */}
          <section className="mb-14" id="openai">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">OpenAI</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Access GPT-4o, GPT-4, and other OpenAI models. Requires an API key from{" "}
              <a href="https://platform.openai.com" className="text-primary hover:underline">platform.openai.com</a>.
            </p>
            <div className="terminal mb-6">
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
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              The API key is stored in the OS keychain (not the config file). Enter it through{" "}
              <strong className="text-foreground">Settings &rarr; Providers &rarr; OpenAI</strong>.
              Token counting uses the <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">cl100k_base</code> tokenizer
              via tiktoken-rs for accurate billing estimation.
            </p>
          </section>

          {/* Anthropic */}
          <section className="mb-14" id="anthropic">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Anthropic</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Access Claude models. Requires an API key from{" "}
              <a href="https://console.anthropic.com" className="text-primary hover:underline">console.anthropic.com</a>.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[providers.anthropic]</span></p>
                <p>provider_type = <span className="text-success">&quot;anthropic&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;claude-opus-4-6&quot;</span></p>
                <p>temperature = <span className="text-warning">0.8</span></p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Uses streaming SSE for real-time responses. Token counting uses{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">cl100k_base</code> (same as OpenAI)
              for estimation.
            </p>
          </section>

          {/* Google Gemini */}
          <section className="mb-14" id="gemini">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Google Gemini</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Access Gemini models. Get an API key from{" "}
              <a href="https://aistudio.google.com" className="text-primary hover:underline">aistudio.google.com</a>.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[providers.gemini]</span></p>
                <p>provider_type = <span className="text-success">&quot;gemini&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;gemini-pro&quot;</span></p>
                <p>max_tokens = <span className="text-warning">4096</span></p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Uses the Gemini API v1 beta. Token counting uses character-based estimation (characters / 4).
            </p>
          </section>

          {/* Ollama */}
          <section className="mb-14" id="ollama">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Ollama (Local)</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Run models locally on your machine using{" "}
              <a href="https://ollama.com" className="text-primary hover:underline">Ollama</a>.
              No API key needed — completely private and offline.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">terminal</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> Install Ollama and pull a model</p>
                <p>ollama pull llama3.1</p>
                <p>ollama pull mistral</p>
                <p>ollama pull phi3</p>
              </div>
            </div>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[providers.ollama]</span></p>
                <p>provider_type = <span className="text-success">&quot;ollama&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;llama3.1&quot;</span></p>
                <p>endpoint = <span className="text-success">&quot;http://localhost:11434&quot;</span></p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Ollama runs on port 11434 by default. Set the{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">endpoint</code> if
              you&apos;re running it on a different host or port. Token counting uses character-based estimation.
            </p>
          </section>

          {/* AWS Bedrock */}
          <section className="mb-14" id="bedrock">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">AWS Bedrock</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Access models through your AWS account using Bedrock. Supports Claude, Titan, Llama,
              and other models available in your AWS region.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[providers.bedrock]</span></p>
                <p>provider_type = <span className="text-success">&quot;bedrock&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;anthropic.claude-v2&quot;</span></p>
                <p>endpoint = <span className="text-success">&quot;https://bedrock-runtime.us-east-1.amazonaws.com&quot;</span></p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Authentication uses AWS SigV4 signing. Configure your AWS credentials via environment
              variables (<code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">AWS_ACCESS_KEY_ID</code>,{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">AWS_SECRET_ACCESS_KEY</code>) or
              the standard AWS credentials file. Uses the InvokeModelWithResponseStream API for streaming.
            </p>
          </section>

          {/* Custom HTTP */}
          <section className="mb-14" id="custom">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Custom HTTP</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Connect to any API endpoint that follows the OpenAI chat completions format. Use this
              for self-hosted models, LLM proxies, or alternative providers.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/60">[providers.my-proxy]</span></p>
                <p>provider_type = <span className="text-success">&quot;custom&quot;</span></p>
                <p>default_model = <span className="text-success">&quot;my-model&quot;</span></p>
                <p>endpoint = <span className="text-success">&quot;https://my-llm-proxy.example.com/v1&quot;</span></p>
                <p>max_tokens = <span className="text-warning">2048</span></p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              The endpoint must support the <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">/chat/completions</code> path
              with SSE streaming. API key authentication is optional.
            </p>
          </section>

          {/* Provider Rotation */}
          <section className="mb-14" id="rotation">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Provider Rotation &amp; Fallback
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              When you configure multiple providers, Omni automatically handles failover. If the
              primary provider returns an error or times out, the request is retried with the next
              available provider using exponential backoff.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Backoff Schedule</h3>
            <div className="grid grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                { retry: "1st retry", delay: "5 seconds" },
                { retry: "2nd retry", delay: "15 seconds" },
                { retry: "3rd retry", delay: "60 seconds" },
                { retry: "4th retry", delay: "300 seconds" },
              ].map((r) => (
                <div key={r.retry} className="bg-card px-4 py-3 text-center">
                  <p className="text-xs text-muted-foreground mb-1">{r.retry}</p>
                  <p className="text-sm font-mono font-medium">{r.delay}</p>
                </div>
              ))}
            </div>

            <p className="text-sm text-muted-foreground leading-relaxed">
              Providers are tried in order of their configured priority. A provider that repeatedly
              fails is temporarily marked unavailable and skipped until its backoff period expires.
              You can disable a provider without removing it by setting{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">enabled = false</code>.
            </p>
          </section>

          {/* Streaming */}
          <section className="mb-14" id="streaming">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Streaming</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              All providers use Server-Sent Events (SSE) streaming for real-time token delivery.
              As the LLM generates its response, tokens are streamed to the UI character by character.
            </p>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              The streaming architecture uses Tokio async streams with byte buffer accumulation.
              Each chunk can contain:
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { type: "TextDelta", desc: "Partial text content" },
                { type: "ToolCallDelta", desc: "Tool call being assembled" },
                { type: "Usage", desc: "Token count update" },
                { type: "Done", desc: "Stream complete signal" },
              ].map((c) => (
                <div key={c.type} className="bg-card px-4 py-3">
                  <p className="text-sm font-mono font-medium text-primary/80 mb-0.5">{c.type}</p>
                  <p className="text-xs text-muted-foreground">{c.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Token Counting */}
          <section className="mb-14" id="tokens">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Token Counting
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Omni tracks token usage per request to help you monitor costs. The counting method
              varies by provider.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden">
              <div className="grid grid-cols-[1fr_1fr_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Provider</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Method</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Notes</div>
                {[
                  { provider: "OpenAI", method: "tiktoken cl100k_base", notes: "Exact tokenizer match for GPT-4 and GPT-3.5 models." },
                  { provider: "Anthropic", method: "tiktoken cl100k_base", notes: "Close approximation. Actual usage may differ slightly." },
                  { provider: "Gemini", method: "chars / 4", notes: "Character-based estimation." },
                  { provider: "Ollama", method: "chars / 4", notes: "Character-based estimation." },
                  { provider: "Bedrock", method: "chars / 4", notes: "Character-based estimation. AWS may report exact counts." },
                  { provider: "Custom", method: "chars / 4", notes: "Default estimation for unknown tokenizers." },
                ].map((row) => (
                  <>
                    <div key={`p-${row.provider}`} className="bg-card px-3 py-2 text-sm font-medium">{row.provider}</div>
                    <div key={`m-${row.provider}`} className="bg-card px-3 py-2 text-sm font-mono text-muted-foreground">{row.method}</div>
                    <div key={`n-${row.provider}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.notes}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Next Steps */}
          <section id="next-steps">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Next Steps
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <Link
                href="/docs/configuration#providers"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Provider Config Reference
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  All configuration fields for the [providers] section.
                </p>
              </Link>
              <Link
                href="/docs/sdk#context"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  LLM Client SDK
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Make LLM requests from extensions using ctx.llm().
                </p>
              </Link>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
