import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Documentation — Setup, SDK & API Guides",
  description:
    "Complete Omni documentation covering AI agent setup, configuration, LLM providers, messaging channels, WASM SDK, extension publishing, security permissions, and system architecture.",
  openGraph: {
    title: "Omni Documentation — AI Agent Setup, SDK & API Guides",
    description:
      "Complete Omni documentation covering AI agent setup, configuration, LLM providers, messaging channels, WASM SDK, extension publishing, security permissions, and architecture.",
    url: "/docs",
  },
  alternates: { canonical: "/docs" },
};

const sections = [
  {
    href: "/docs/getting-started",
    title: "Getting Started",
    description:
      "Set up Omni on your machine, connect your first LLM provider and channels, and install extensions from the marketplace.",
  },
  {
    href: "/docs/configuration",
    title: "Configuration",
    description:
      "Complete reference for the omni.toml config file — providers, agent, guardian, permissions, UI, channels, and marketplace.",
  },
  {
    href: "/docs/providers",
    title: "LLM Providers",
    description:
      "Connect to OpenAI, Anthropic, Google Gemini, Ollama, AWS Bedrock, or any custom HTTP endpoint. Automatic failover and rotation.",
  },
  {
    href: "/docs/channels",
    title: "Channels",
    description:
      "Connect to 21 messaging platforms — Discord, Telegram, Slack, WhatsApp, and more. Multi-instance support and message routing.",
  },
  {
    href: "/docs/security",
    title: "Security & Permissions",
    description:
      "The 4-layer Guardian anti-injection pipeline, 26 capability-based permissions, WASM sandboxing, and audit logging.",
  },
  {
    href: "/docs/tools",
    title: "Native Tools",
    description:
      "All 29 built-in tools — file operations, web access, memory, version control, testing, code intelligence, debugging, REPL, sub-agents, MCP client, desktop automation, and more.",
  },
  {
    href: "/docs/flowcharts",
    title: "Flowchart Builder",
    description:
      "Build AI workflows visually with 19 node types. Drag-and-drop editor, expression evaluator, auto-triggers, sub-flows, and full native tool access — no code required.",
  },
  {
    href: "/docs/hooks",
    title: "Hook System",
    description:
      "Intercept and modify data at 7 points in the agent loop. Block tool calls, transform messages, and react to session events.",
  },
  {
    href: "/docs/architecture",
    title: "Architecture",
    description:
      "System architecture — crate map, agent loop, event bus, database, data flow, and extension lifecycle.",
  },
  {
    href: "/docs/sdk",
    title: "SDK Reference",
    description:
      "Build extensions with the Omni Rust SDK. Tool definitions, host functions, the manifest format, and WASM compilation.",
  },
  {
    href: "/docs/publishing",
    title: "Publishing Guide",
    description:
      "Publish your extension to the marketplace. API keys, CLI workflow, security scan pipeline, and versioning.",
  },
];

const quickLinks = [
  { href: "/docs/getting-started#installation", label: "Install Omni" },
  { href: "/docs/providers#ollama", label: "Local Models (Ollama)" },
  { href: "/docs/channels#supported-channels", label: "21 Channels" },
  { href: "/docs/security#capabilities", label: "26 Permissions" },
  { href: "/docs/tools#system", label: "Native Tools" },
  { href: "/docs/hooks#hook-points", label: "Hook Points" },
  { href: "/docs/flowcharts#node-types", label: "19 Node Types" },
  { href: "/docs/flowcharts#expressions", label: "Expressions" },
  { href: "/docs/configuration#full-example", label: "Full Config Example" },
  { href: "/docs/architecture#agent-loop", label: "Agent Loop" },
  { href: "/docs/sdk#manifest", label: "Manifest Format" },
  { href: "/docs/sdk#context", label: "Context API" },
  { href: "/docs/publishing#api-keys", label: "Create API Key" },
  { href: "/docs/publishing#security-scan", label: "Security Pipeline" },
];

export default function DocsPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0">
          <div className="max-w-2xl mb-12">
            <p className="text-sm font-medium text-primary mb-3">Docs</p>
            <h1 className="text-3xl md:text-4xl font-bold tracking-tight">
              Documentation
            </h1>
            <p className="mt-4 text-muted-foreground leading-relaxed">
              Everything you need to use Omni, build extensions, and publish to
              the marketplace.
            </p>
          </div>

          {/* Main sections */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-16">
            {sections.map((section) => (
              <Link
                key={section.href}
                href={section.href}
                className="bg-card p-6 hover:bg-card/80 transition-colors group"
              >
                <h2 className="font-medium text-[15px] mb-2 group-hover:text-primary transition-colors">
                  {section.title}
                </h2>
                <p className="text-sm text-muted-foreground leading-relaxed mb-4">
                  {section.description}
                </p>
                <span className="flex items-center text-sm text-primary font-medium">
                  Read more
                  <ArrowRight className="h-3.5 w-3.5 ml-1 group-hover:translate-x-1 transition-transform" />
                </span>
              </Link>
            ))}
          </div>

          {/* Quick Links */}
          <div>
            <h2 className="text-lg font-medium mb-4">Quick links</h2>
            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {quickLinks.map((link) => (
                <Link
                  key={link.href}
                  href={link.href}
                  className="bg-card px-4 py-3 text-sm text-muted-foreground hover:text-foreground hover:bg-card/80 transition-colors"
                >
                  {link.label}
                </Link>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
