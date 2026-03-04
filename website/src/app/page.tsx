import type { Metadata } from "next";
import Link from "next/link";
import {
  ArrowRight,
  Check,
  Heart,
  MessageSquare,
  Headphones,
  Users,
  Mail,
  ShoppingCart,
  BookOpen,
  BarChart3,
  Terminal,
  FileText,
  Globe,
  Database,
  Search,
  Bell,
  Monitor,
  GitBranch,
  FlaskConical,
  ClipboardCopy,
  Code,
  Bug,
  Play,
  Plug,
  Bot,
  Wrench,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { FeaturedExtensions } from "@/components/landing/FeaturedExtensions";
import { ParticleField } from "@/components/landing/ParticleField";
import { RotatingText } from "@/components/landing/RotatingText";
import { JsonLd } from "@/components/seo/JsonLd";

export const metadata: Metadata = {
  title: {
    absolute: "Omni — Build AI Agents for Any Task | Free Desktop App for Windows, macOS & Linux",
  },
  description:
    "Omni is a free, open-source desktop application that lets you build AI agents with full computer control — execute commands, read/write files, call APIs, automate desktop apps, and more. Available for Windows, macOS, and Linux. Connect 21+ messaging channels, use 29 built-in tools, choose from 6 LLM providers, and run everything locally with full privacy. All actions are sandboxed and permission-gated.",
  openGraph: {
    title: "Omni — Build AI Agents for Any Task | Free Desktop App",
    description:
      "Build AI agents with full computer control — execute commands, manage files, call APIs, automate desktop apps, and more across 21+ channels. Free desktop app for Windows, macOS & Linux. Download now.",
    url: "/",
  },
  alternates: {
    canonical: "/",
  },
};

const scanLayers = [
  {
    name: "Signature Scanning",
    desc: "79+ malicious patterns matched against compiled binary",
    weight: "30%",
  },
  {
    name: "Heuristic Analysis",
    desc: "Behavioral rules flag suspicious permission combinations",
    weight: "25%",
  },
  {
    name: "AI Code Review",
    desc: "Claude analyzes code semantics for hidden threats",
    weight: "30%",
  },
  {
    name: "Sandbox Testing",
    desc: "Isolated execution detects runtime anomalies",
    weight: "15%",
  },
];

const capabilities = [
  "Desktop app for Windows, macOS & Linux",
  "Build AI agents for any task",
  "21+ messaging channels (Discord, Slack, Telegram & more)",
  "29 built-in native tools",
  "6 LLM providers (OpenAI, Anthropic, Gemini, Ollama & more)",
  "Open source & fully local — your data stays yours",
];

const channels = [
  "Discord", "Telegram", "Slack", "WhatsApp", "Microsoft Teams",
  "Matrix", "IRC", "Google Chat", "Signal", "iMessage",
  "Line", "Twitch", "Mattermost", "Feishu", "Nostr",
  "Nextcloud Talk", "Zalo", "BlueBubbles", "Synology Chat",
  "Twitter / X", "Urbit", "WebChat",
];

const useCases = [
  {
    icon: Headphones,
    title: "Customer Support",
    desc: "Build an agent that answers customer questions across Discord, email, and webchat using your docs as context.",
  },
  {
    icon: Users,
    title: "Community Moderation",
    desc: "Moderate Discord servers, Telegram groups, and Twitch chat — enforce rules, answer FAQs, and manage conversations automatically.",
  },
  {
    icon: Mail,
    title: "Email & Message Triage",
    desc: "Let your agent read, categorize, and draft replies to incoming messages across Slack, Teams, email, and more.",
  },
  {
    icon: ShoppingCart,
    title: "E-commerce Automation",
    desc: "Handle order inquiries, track shipments, process returns, and update customers across WhatsApp and webchat.",
  },
  {
    icon: BookOpen,
    title: "Knowledge Base Agent",
    desc: "Create an agent that searches your files and documentation to answer questions on any connected channel.",
  },
  {
    icon: BarChart3,
    title: "Workflow Automation",
    desc: "Chain tools together — fetch data from APIs, process files, run scripts, and post results to any channel.",
  },
  {
    icon: Wrench,
    title: "Developer Automation",
    desc: "Run tests, debug failures, search code symbols, manage git branches, and spawn sub-agents for parallel development tasks.",
  },
];

const providers = [
  { name: "OpenAI", models: "GPT-4o, GPT-4, GPT-3.5" },
  { name: "Anthropic", models: "Claude Opus, Sonnet, Haiku" },
  { name: "Google Gemini", models: "Gemini 2.5 Pro, Flash" },
  { name: "Ollama", models: "Llama, Mistral, Phi & more" },
  { name: "AWS Bedrock", models: "Claude, Llama, Mistral" },
  { name: "Custom", models: "Any OpenAI-compatible endpoint" },
];

const nativeTools = [
  { icon: Terminal, name: "Execute Commands", desc: "Run shell commands, scripts, and programs directly from your agent" },
  { icon: FileText, name: "Read & Write Files", desc: "Read, create, edit, and patch files — logs, configs, documents, code" },
  { icon: Search, name: "Grep & Search", desc: "Regex search across files and directories with line numbers and context" },
  { icon: Globe, name: "HTTP & Web Search", desc: "Call any API, fetch web pages, search the web, and scrape content with anti-bot stealth" },
  { icon: Database, name: "Persistent Memory", desc: "Save and search agent memory that persists across sessions with tags and categories" },
  { icon: GitBranch, name: "Git Version Control", desc: "10 structured actions — commit, branch, merge, diff, stash — with automatic secret scanning" },
  { icon: FlaskConical, name: "Test Runner", desc: "Auto-detect and run tests — Rust, Jest, Vitest, pytest, Go, .NET — with structured pass/fail results" },
  { icon: Code, name: "Code Intelligence", desc: "Offline symbol search across 9 languages plus real-time LSP for go-to-definition, references, hover, and diagnostics" },
  { icon: Bug, name: "Debugger (DAP)", desc: "Launch, attach, set breakpoints, step through code, and inspect variables via Debug Adapter Protocol" },
  { icon: Play, name: "Interactive REPL", desc: "Persistent Python and Node.js sessions with state between executions — up to 3 concurrent" },
  { icon: Bot, name: "Sub-Agents", desc: "Spawn parallel sub-agents with their own context and tools for concurrent task execution" },
  { icon: Plug, name: "MCP Client", desc: "Connect external MCP tool servers via stdio — tools are auto-discovered, namespaced, and Guardian-scanned" },
  { icon: Monitor, name: "App Automation", desc: "Launch and control desktop apps — click, type, read, screenshot — with LOLBIN blocklist and password protection" },
  { icon: ClipboardCopy, name: "Clipboard", desc: "Read from and write to the system clipboard for quick data transfer" },
  { icon: Bell, name: "Notifications & Cron", desc: "Desktop notifications for alerts and cron scheduling for recurring automated tasks" },
];

const BASE_URL = process.env.NEXT_PUBLIC_APP_URL || "https://www.omniapp.org";

export default function HomePage() {
  return (
    <div>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "SoftwareApplication",
          name: "Omni",
          url: BASE_URL,
          logo: `${BASE_URL}/og-image.png`,
          description:
            "Omni is a free, open-source desktop application for Windows, macOS, and Linux that lets you build and run AI agents to automate any task. Connect 21+ messaging channels, extend with sandboxed plugins, and keep everything private and local.",
          applicationCategory: "DeveloperApplication",
          operatingSystem: "Windows, macOS, Linux",
          offers: {
            "@type": "Offer",
            price: "0",
            priceCurrency: "USD",
          },
          downloadUrl: `${BASE_URL}/download`,
        }}
      />
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "WebSite",
          name: "Omni — AI Agent Builder",
          url: BASE_URL,
          potentialAction: {
            "@type": "SearchAction",
            target: {
              "@type": "EntryPoint",
              urlTemplate: `${BASE_URL}/extensions?q={search_term_string}`,
            },
            "query-input": "required name=search_term_string",
          },
        }}
      />

      {/* Hero */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <ParticleField />
        <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 pt-24 pb-20 md:pt-32 md:pb-28">
          <div className="max-w-2xl">
            <p className="text-sm font-medium text-muted-foreground mb-4">
              Free desktop app for Windows, macOS & Linux
            </p>
            <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold tracking-tight leading-[1.1]">
              <RotatingText />
            </h1>
            <p className="mt-6 text-base md:text-lg text-muted-foreground max-w-lg leading-relaxed">
              Omni is a desktop application that gives your AI agent full computer control — execute commands, read and write files, call APIs, and automate workflows. Connect to 21+ channels, choose from 6 LLM providers, and extend with sandboxed plugins. Everything runs locally on your machine with every action permission-gated.
            </p>
            <div className="mt-8 flex items-center gap-3">
              <Link href="/download">
                <Button size="xl">Download for free</Button>
              </Link>
              <Link href="/docs/getting-started">
                <Button size="xl" variant="outline">
                  Get started
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Capabilities strip */}
      <section className="border-y border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-6">
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
            {capabilities.map((cap) => (
              <div key={cap} className="flex items-center gap-2 text-[13px] text-muted-foreground">
                <Check className="h-3.5 w-3.5 text-primary shrink-0" />
                {cap}
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* How your agent works */}
      <section className="py-20 md:py-28">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="grid lg:grid-cols-2 gap-16 items-start">
            <div>
              <p className="text-sm font-medium text-primary mb-3">How It Works</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                Your agent thinks, acts,<br />and responds — automatically
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed max-w-md">
                When a message arrives on any connected channel, Omni&apos;s agent loop takes over. It scans the input for safety, reasons with your chosen LLM, calls tools when needed, and sends a response — all in seconds.
              </p>
              <div className="mt-10 space-y-6">
                {[
                  { step: "01", title: "Message arrives", desc: "A user sends a message on Discord, Telegram, Slack, or any of your 21+ connected channels" },
                  { step: "02", title: "Guardian scans input", desc: "The 4-layer security pipeline checks for prompt injection and adversarial attacks before anything reaches the LLM" },
                  { step: "03", title: "LLM reasons & calls tools", desc: "Your chosen AI model (GPT-4, Claude, Gemini, or local Ollama) processes the message and decides which tools to use" },
                  { step: "04", title: "Tools execute in sandbox", desc: "Built-in tools and WASM extensions run in isolated sandboxes — file access, HTTP requests, and more, all permission-gated" },
                  { step: "05", title: "Response sent back", desc: "The final answer is scanned for safety, then delivered to the user on the same channel — in real time via streaming" },
                ].map((item) => (
                  <div key={item.step} className="flex gap-4">
                    <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                      {item.step}
                    </div>
                    <div className="flex-1 border-b border-border/50 pb-6">
                      <h4 className="font-medium text-[15px]">{item.title}</h4>
                      <p className="mt-1 text-sm text-muted-foreground">{item.desc}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Terminal agent flow */}
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni agent</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">[discord]</span> user: What&apos;s the weather in NYC?</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary">guardian</span> Input scan <span className="text-success">CLEAN</span> <span className="text-muted-foreground/40">2.1ms</span></p>
                <p><span className="text-primary">llm</span> Routing to <span className="text-foreground/70">gpt-4o</span> via OpenAI</p>
                <p><span className="text-primary">llm</span> Tool call: <span className="text-foreground/70">weather.get_weather</span>({`{`}location: &quot;NYC&quot;{`}`})</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary">perms</span> network.http → api.openweathermap.org <span className="text-success">ALLOW</span></p>
                <p><span className="text-primary">wasm</span> Executing in sandbox <span className="text-muted-foreground/40">mem: 12MB / 64MB</span></p>
                <p><span className="text-primary">wasm</span> Tool result: <span className="text-foreground/70">72°F, partly cloudy</span> <span className="text-muted-foreground/40">340ms</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-primary">guardian</span> Output scan <span className="text-success">CLEAN</span></p>
                <p><span className="text-primary">llm</span> Streaming response...</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">[discord]</span> <span className="text-foreground/70">omni:</span> It&apos;s currently 72°F and partly</p>
                <p><span className="text-foreground/70">cloudy in New York City.</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p className="text-muted-foreground/40">Session saved · 847 tokens · 1.2s total</p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Channels */}
      <section className="border-t border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="text-center max-w-2xl mx-auto mb-12">
            <p className="text-sm font-medium text-primary mb-3">Channels</p>
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
              One agent, 21+ platforms
            </h2>
            <p className="mt-4 text-muted-foreground leading-relaxed">
              Connect your AI agent to every platform where your conversations happen.
              Each channel runs natively — not as a plugin — with full support for messages, groups, and reactions.
            </p>
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3">
            {channels.map((ch) => (
              <div
                key={ch}
                className="flex items-center justify-center gap-2 rounded-lg border border-border/50 bg-card px-3 py-3 text-sm text-muted-foreground hover:text-foreground hover:border-border transition-colors"
              >
                <MessageSquare className="h-3.5 w-3.5 shrink-0 text-primary/60" />
                {ch}
              </div>
            ))}
          </div>
          <p className="mt-6 text-center text-xs text-muted-foreground">
            All channels are built into the desktop app — no extra plugins or subscriptions required.
          </p>
        </div>
      </section>

      {/* Use cases */}
      <section className="border-t border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-3 gap-12">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Use Cases</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                What can you build?
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Omni agents handle real work — from answering customer questions to automating entire workflows across every channel.
              </p>
            </div>
            <div className="lg:col-span-2 grid sm:grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {useCases.map((uc) => (
                <div key={uc.title} className="bg-card p-6 md:p-8">
                  <uc.icon className="h-5 w-5 text-primary/70 mb-3" />
                  <h3 className="font-medium text-[15px] mb-2">{uc.title}</h3>
                  <p className="text-sm text-muted-foreground leading-relaxed">{uc.desc}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* LLM Providers */}
      <section className="border-t border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="text-center max-w-2xl mx-auto mb-12">
            <p className="text-sm font-medium text-primary mb-3">AI Providers</p>
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
              Bring your own AI — or run it locally
            </h2>
            <p className="mt-4 text-muted-foreground leading-relaxed">
              Omni works with 6 LLM providers out of the box. Use cloud APIs, run models locally with Ollama for full offline privacy, or connect any OpenAI-compatible endpoint. Switch providers any time — your agent keeps working.
            </p>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
            {providers.map((p) => (
              <div
                key={p.name}
                className="rounded-lg border border-border/50 bg-card p-4 text-center"
              >
                <div className="font-medium text-[15px] mb-1">{p.name}</div>
                <div className="text-xs text-muted-foreground">{p.models}</div>
              </div>
            ))}
          </div>
          <p className="mt-6 text-center text-xs text-muted-foreground">
            API keys are stored in your OS keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service) — never in plaintext.
          </p>
        </div>
      </section>

      {/* Native Tools — Computer Control */}
      <section className="border-t border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="text-center max-w-2xl mx-auto mb-12">
            <p className="text-sm font-medium text-primary mb-3">Built-in Tools</p>
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
              Full computer control — sandboxed and permission-gated
            </h2>
            <p className="mt-4 text-muted-foreground leading-relaxed">
              29 native tools give your agent full system access — shell commands, file I/O, web requests, git, testing, debugging, code intelligence, interactive REPLs, and desktop app automation. Connect external tools via MCP. Every action is permission-gated, sandboxed, and audited.
            </p>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3">
            {nativeTools.map((tool) => (
              <div
                key={tool.name}
                className="rounded-lg border border-border/50 bg-card p-4"
              >
                <tool.icon className="h-4 w-4 text-primary/70 mb-2" />
                <div className="font-medium text-[14px] mb-1">{tool.name}</div>
                <div className="text-xs text-muted-foreground leading-relaxed">{tool.desc}</div>
              </div>
            ))}
          </div>
          <p className="mt-6 text-center text-xs text-muted-foreground">
            Every tool action requires explicit user permission. Agents can&apos;t access files, run commands, or make network requests without your approval.
          </p>
        </div>
      </section>

      {/* Featured Extensions */}
      <section className="border-t border-border/50 py-20 md:py-28">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="flex items-end justify-between mb-10">
            <div>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">Featured</h2>
              <p className="mt-2 text-sm text-muted-foreground">Hand-picked by the Omni team</p>
            </div>
            <Link
              href="/extensions?sort=downloads"
              className="text-sm text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
            >
              View all <ArrowRight className="h-3.5 w-3.5" />
            </Link>
          </div>
          <FeaturedExtensions />
        </div>
      </section>

      {/* Security pipeline */}
      <section className="border-t border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-2 gap-16 items-start">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Security</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                4-layer security pipeline
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed max-w-md">
                Every extension passes through four independent analysis layers before it can be published. No exceptions.
              </p>
              <div className="mt-10 space-y-6">
                {scanLayers.map((layer, i) => (
                  <div key={layer.name} className="flex gap-4">
                    <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                      {String(i + 1).padStart(2, "0")}
                    </div>
                    <div className="flex-1 border-b border-border/50 pb-6">
                      <div className="flex items-baseline justify-between">
                        <h4 className="font-medium text-[15px]">{layer.name}</h4>
                        <span className="text-xs font-mono text-muted-foreground">{layer.weight}</span>
                      </div>
                      <p className="mt-1 text-sm text-muted-foreground">{layer.desc}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Terminal scan output */}
            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni scan</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">$</span> omni scan weather-tool.wasm</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>Running security pipeline...</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  <span className="text-success">PASS</span>
                  <span className="text-muted-foreground/50">{" "}|</span> Signature scan
                  <span className="text-muted-foreground/40 ml-4">0 matches</span>
                </p>
                <p>
                  <span className="text-success">PASS</span>
                  <span className="text-muted-foreground/50">{" "}|</span> Heuristic analysis
                  <span className="text-muted-foreground/40 ml-4">score 95/100</span>
                </p>
                <p>
                  <span className="text-success">PASS</span>
                  <span className="text-muted-foreground/50">{" "}|</span> AI code review
                  <span className="text-muted-foreground/40 ml-4">score 92/100</span>
                </p>
                <p>
                  <span className="text-success">PASS</span>
                  <span className="text-muted-foreground/50">{" "}|</span> Sandbox testing
                  <span className="text-muted-foreground/40 ml-4">score 98/100</span>
                </p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p>
                  Verdict: <span className="text-success font-medium">CLEAN</span>
                  <span className="text-muted-foreground/40 ml-4">overall 96.3</span>
                </p>
                <p className="text-muted-foreground/40">Extension approved for marketplace.</p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Why Omni */}
      <section className="border-t border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-3 gap-12">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Why Omni</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                All the power.<br />None of the risk.
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Other AI agent tools give you raw computer control with no guardrails. Omni gives you the same power — shell access, file I/O, API calls, process execution — but every action is sandboxed, permission-gated, and audited. Full control for your agent, full visibility for you.
              </p>
            </div>

            <div className="lg:col-span-2 grid sm:grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                {
                  title: "Persistent Desktop Agents",
                  desc: "Agents run continuously on your machine, listening across 21+ channels and responding in real time — not one-shot prompts, but always-on automation.",
                },
                {
                  title: "Full System Access",
                  desc: "Execute commands, read/write files, make HTTP requests, manage git repos, run tests, debug code, and control desktop apps. Your agent can do anything you can — with your permission.",
                },
                {
                  title: "Deny-by-Default Security",
                  desc: "Every capability is blocked until you explicitly allow it. File access is path-scoped. Network requests are domain-scoped. Commands are whitelisted. Full audit trail.",
                },
                {
                  title: "Guardian Anti-Injection",
                  desc: "A 4-layer pipeline (signatures, heuristics, ML classifier, policy validation) scans every input and output for prompt injection attacks in real time.",
                },
              ].map((item) => (
                <div key={item.title} className="bg-card p-6 md:p-8">
                  <h3 className="font-medium text-[15px] mb-2">{item.title}</h3>
                  <p className="text-sm text-muted-foreground leading-relaxed">{item.desc}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="border-t border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="max-w-lg mx-auto text-center">
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">Ready to build your AI agent?</h2>
            <p className="mt-3 text-muted-foreground leading-relaxed">
              Download Omni for free on Windows, macOS, or Linux. Set up your first AI agent in minutes — no coding required.
            </p>
            <div className="mt-8 flex flex-col sm:flex-row flex-wrap items-center justify-center gap-3">
              <Link href="/download">
                <Button size="xl">Download Omni</Button>
              </Link>
              <Link href="/docs/getting-started">
                <Button size="xl" variant="outline">Get started guide</Button>
              </Link>
              <Link href="/donate">
                <Button size="xl" variant="ghost" className="text-muted-foreground">
                  <Heart className="h-4 w-4" />
                  Donate
                </Button>
              </Link>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
