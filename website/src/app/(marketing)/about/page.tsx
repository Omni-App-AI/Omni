import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";

export const metadata: Metadata = {
  title: "About Omni — Desktop AI Agent Builder for Windows, macOS & Linux",
  description:
    "Omni is a free, open-source desktop application for Windows, macOS, and Linux that lets you build AI agents to automate any task. Connect 21+ channels, use 29 native tools, choose from 6 LLM providers, and keep everything private and local.",
  openGraph: {
    title: "About Omni — Build AI Agents on Your Desktop",
    description:
      "Omni is a free desktop app for Windows, macOS & Linux. Build AI agents that automate tasks across 21+ channels with full privacy — no cloud required.",
    url: "/about",
  },
  alternates: { canonical: "/about" },
};

const principles = [
  {
    title: "Privacy First",
    description:
      "Your data stays on your machine. Omni processes everything locally by default — no telemetry, no cloud storage of conversations, no data mining.",
  },
  {
    title: "Security by Default",
    description:
      "Every extension runs in an isolated WebAssembly sandbox with capability-based permissions. Our 4-layer pipeline scans every submission before it reaches users.",
  },
  {
    title: "Open Extension Ecosystem",
    description:
      "Build and share extensions using the Omni SDK. Write tools in Rust, compile to WASM, and publish to the marketplace with access to LLM inference, channels, and more.",
  },
  {
    title: "Channel Agnostic",
    description:
      "Connect to 21+ communication platforms — Discord, Telegram, Slack, WhatsApp, Teams, Matrix, IRC, and more. Your AI agent works wherever your conversations happen.",
  },
];

const stats = [
  { value: "21+", label: "Channels" },
  { value: "29", label: "Native Tools" },
  { value: "6", label: "LLM Providers" },
  { value: "4", label: "Scan Layers" },
];

const steps = [
  { title: "Build", desc: "Write your extension in Rust using the Omni SDK and compile to WASM" },
  { title: "Publish", desc: "Upload your extension via the CLI or dashboard with a single command" },
  { title: "Scan", desc: "Our 4-layer AV pipeline analyzes your code for safety and security" },
  { title: "Discover", desc: "Users find and install your extension from the marketplace" },
];

export default function AboutPage() {
  return (
    <div>
      {/* Hero */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 pt-20 pb-16 md:pt-28 md:pb-24">
          <div className="max-w-2xl">
            <p className="text-sm font-medium text-muted-foreground mb-4">
              About Omni
            </p>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight leading-[1.1]">
              A desktop app to build
              <br />
              <span className="text-gradient">AI agents for any task.</span>
            </h1>
            <p className="mt-6 text-base md:text-lg text-muted-foreground max-w-lg leading-relaxed">
              Omni is a free, open-source desktop application for Windows, macOS, and Linux.
              Create AI agents that automate tasks, answer questions, moderate communities,
              and connect to 21+ messaging channels — all running locally on your machine
              with full privacy.
            </p>
          </div>
        </div>
      </section>

      {/* Stats strip */}
      <section className="border-y border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-6">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-8">
            {stats.map((stat) => (
              <div key={stat.label}>
                <div className="text-2xl font-bold text-foreground">{stat.value}</div>
                <div className="text-xs font-mono text-muted-foreground mt-0.5">{stat.label}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* What is Omni */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-2 gap-16">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Desktop Software</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                Not a web app — a real desktop application
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Omni is a native desktop application you download and install on
                Windows, macOS, or Linux. Unlike cloud-based AI tools, everything
                runs on your machine — your data, conversations, and API keys never
                leave your device.
              </p>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Build AI agents that handle any task: automate customer support, moderate
                Discord servers, triage emails, manage Telegram groups, or create custom
                workflows. Connect your preferred LLM provider — OpenAI, Anthropic,
                Google Gemini, Ollama for fully local models, AWS Bedrock, or any custom endpoint.
              </p>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                With 21+ built-in channel integrations, your AI agent can operate across
                Discord, Telegram, Slack, WhatsApp, Teams, Matrix, IRC, email, webchat,
                and more — all managed from one interface on your desktop.
              </p>
            </div>
            <div>
              <p className="text-sm font-medium text-primary mb-3">Marketplace</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                Built on trust
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                The Omni Marketplace is where developers publish extensions and
                users discover new capabilities for their AI agents. Unlike typical
                app stores, every extension goes through a rigorous security review.
              </p>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Our 4-layer antivirus pipeline — combining signature scanning,
                heuristic analysis, AI-powered code review by Claude, and sandboxed
                execution testing — ensures that only safe extensions are published.
              </p>
              <div className="mt-6">
                <Link href="/security">
                  <Button variant="outline" size="sm">
                    Learn about our security
                    <ArrowRight className="h-3.5 w-3.5" />
                  </Button>
                </Link>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Key Principles */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-3 gap-12">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Values</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                Built on principles
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Every decision in Omni&apos;s design is guided by these core values.
              </p>
            </div>

            <div className="lg:col-span-2 grid sm:grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {principles.map((p) => (
                <div key={p.title} className="bg-card p-6 md:p-8">
                  <h3 className="font-medium text-[15px] mb-2">{p.title}</h3>
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    {p.description}
                  </p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* Publishing flow */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-2 gap-16 items-start">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Workflow</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                From code to marketplace
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed max-w-md">
                Build, publish, scan, and distribute — the entire pipeline is
                automated and transparent.
              </p>
            </div>
            <div className="space-y-6">
              {steps.map((item, i) => (
                <div key={item.title} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {String(i + 1).padStart(2, "0")}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-6">
                    <h4 className="font-medium text-[15px]">{item.title}</h4>
                    <p className="mt-1 text-sm text-muted-foreground">{item.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* Open Source */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="max-w-2xl">
            <p className="text-sm font-medium text-primary mb-3">Open Source</p>
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
              Transparent and auditable
            </h2>
            <p className="mt-4 text-muted-foreground leading-relaxed">
              The Omni desktop application and its core runtime are open source.
              The extension SDK, permission system, Guardian anti-injection pipeline,
              and all 21 channel integrations are available for inspection and
              contribution.
            </p>
            <div className="mt-8 flex items-center gap-3">
              <Link href="https://github.com/Omni-App-AI/Omni" target="_blank" rel="noopener noreferrer">
                <Button variant="outline" size="sm">
                  View on GitHub
                  <ArrowRight className="h-3.5 w-3.5" />
                </Button>
              </Link>
              <Link href="/docs/sdk">
                <Button variant="outline" size="sm">
                  Read the SDK docs
                </Button>
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section>
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="max-w-lg">
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
              Ready to build your AI agent?
            </h2>
            <p className="mt-3 text-muted-foreground leading-relaxed">
              Download Omni for free on Windows, macOS, or Linux. Create your first
              AI agent in minutes.
            </p>
            <div className="mt-8 flex flex-col sm:flex-row flex-wrap items-start sm:items-center gap-3">
              <Link href="/download">
                <Button size="xl">
                  Download Omni
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </Link>
              <Link href="/docs/getting-started">
                <Button size="xl" variant="outline">Get started</Button>
              </Link>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
