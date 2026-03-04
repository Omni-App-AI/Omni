import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Getting Started — Setup & Installation",
  description:
    "Get started with the Omni AI agent in minutes. Install the platform, connect your first messaging channel, configure an LLM provider, and add your first sandboxed WASM extension from the marketplace.",
  openGraph: {
    title: "Getting Started with Omni — AI Agent Setup & Installation",
    description:
      "Get started with the Omni AI agent in minutes. Install the platform, connect your first messaging channel, configure an LLM provider, and add your first WASM extension.",
    url: "/docs/getting-started",
  },
  alternates: { canonical: "/docs/getting-started" },
};

export default function GettingStartedPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-3xl">
          <p className="text-sm font-medium text-primary mb-3">Docs</p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">Getting Started</h1>
          <p className="text-muted-foreground mb-12">
            Set up Omni, connect your first channel, and install extensions in under 10 minutes.
          </p>

          {/* Introduction */}
          <section className="mb-14" id="introduction">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">What is Omni?</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Omni is a privacy-first AI agent that runs locally on your desktop. It connects to
              your preferred LLM provider (OpenAI, Anthropic, Google Gemini, Ollama, AWS Bedrock,
              or any custom endpoint) and gives your AI agent real capabilities through a secure
              extension system.
            </p>
            <p className="text-muted-foreground leading-relaxed mb-4">
              With Omni, your AI agent can browse the web, read and write files, send messages
              across 21+ communication channels, schedule tasks, analyze images, search your
              memory, and much more — all controlled by a fine-grained permission system that
              keeps you in charge.
            </p>
            <p className="text-muted-foreground leading-relaxed">
              Extensions are written in Rust, compiled to WebAssembly (WASM), and run in isolated
              sandboxes. Every extension published to the marketplace is scanned by our 4-layer
              antivirus pipeline before it reaches users.
            </p>
          </section>

          {/* Installation */}
          <section className="mb-14" id="installation">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Installation</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Omni is a desktop application available for Windows, macOS, and Linux.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">System Requirements</h3>
            <ul className="list-disc list-inside text-muted-foreground space-y-1 mb-6 text-sm">
              <li>Windows 10/11 (x64), macOS 12+ (Apple Silicon or Intel), or Linux (x64)</li>
              <li>4 GB RAM minimum (8 GB recommended)</li>
              <li>500 MB free disk space</li>
              <li>Internet connection for LLM API access (not required for Ollama local models)</li>
            </ul>

            <h3 className="text-sm font-medium text-foreground mb-3">Download &amp; Install</h3>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">terminal</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> macOS (Apple Silicon)</p>
                <p>curl -L https://github.com/Omni-App-AI/Omni/releases/latest/download/omni-macos-arm64.dmg -o omni.dmg</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> macOS (Intel)</p>
                <p>curl -L https://github.com/Omni-App-AI/Omni/releases/latest/download/omni-macos-x64.dmg -o omni.dmg</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Windows — download the .msi installer from the releases page</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Linux (Debian/Ubuntu)</p>
                <p>curl -L https://github.com/Omni-App-AI/Omni/releases/latest/download/omni-linux-x64.deb -o omni.deb</p>
                <p>sudo dpkg -i omni.deb</p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              On macOS, open the .dmg file and drag Omni to your Applications folder.
              On Windows, run the .msi installer. On Linux, install the .deb package or
              extract the .tar.gz archive.
            </p>
          </section>

          {/* First Launch */}
          <section className="mb-14" id="first-launch">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">First Launch</h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              When you first open Omni, you&apos;ll be guided through the initial setup.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">1. Connect an LLM Provider</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Go to <strong className="text-foreground">Settings &rarr; Providers</strong> and
              add your API key for one of the supported providers:
            </p>
            <div className="space-y-2 mb-6">
              {[
                { name: "OpenAI", desc: "GPT-4o, GPT-4, etc. Requires an API key from platform.openai.com" },
                { name: "Anthropic", desc: "Claude Opus, Sonnet, Haiku. API key from console.anthropic.com" },
                { name: "Google Gemini", desc: "Gemini Pro, Ultra. API key from aistudio.google.com" },
                { name: "Ollama", desc: "Run models locally (Llama, Mistral, etc.). No API key needed" },
                { name: "AWS Bedrock", desc: "Access models through your AWS account" },
                { name: "Custom HTTP", desc: "Any OpenAI-compatible API endpoint" },
              ].map((p) => (
                <div key={p.name} className="flex gap-3 text-sm">
                  <span className="font-mono text-muted-foreground/60 w-24 shrink-0">{p.name}</span>
                  <span className="text-muted-foreground">{p.desc}</span>
                </div>
              ))}
            </div>

            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">terminal</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> For local models with Ollama:</p>
                <p>ollama pull llama3.1</p>
                <p><span className="text-foreground/40">#</span> Then in Omni Settings → Providers, add Ollama</p>
                <p><span className="text-foreground/40">#</span> URL: http://localhost:11434</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">2. Start Chatting</h3>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Once connected, type a message in the chat input and press Enter. The agent will
              respond using your configured LLM and any activated tools.
            </p>
          </section>

          {/* Connecting Channels */}
          <section className="mb-14" id="channels">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Connecting Channels</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Channels let your Omni agent communicate through external messaging platforms.
              Go to <strong className="text-foreground">Settings &rarr; Channels</strong> to configure.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Supported Channels</h3>
            <div className="grid grid-cols-3 sm:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                "Discord", "Telegram", "Slack", "WhatsApp Web", "Teams", "Google Chat",
                "Matrix", "IRC", "Twitch", "LINE", "Mattermost", "Feishu",
                "Signal", "Nostr", "Nextcloud Talk", "Synology Chat",
                "Twitter/X", "BlueBubbles", "iMessage", "Zalo", "WebChat",
              ].map((ch) => (
                <div key={ch} className="bg-card px-3 py-2 text-xs text-muted-foreground">
                  {ch}
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Example: Connect Discord</h3>
            <div className="space-y-3 mb-4">
              {[
                "Create a Discord bot at discord.com/developers",
                "Copy the bot token",
                "In Omni, go to Channels → Add Instance → Discord",
                "Paste the bot token and click Connect",
                "Invite the bot to your Discord server",
              ].map((step, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{step}</p>
                </div>
              ))}
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Each channel type has its own authentication method. Some channels (like WhatsApp Web
              and Signal) use QR code pairing, while others use bot tokens or API keys.
              You can create multiple instances of the same channel type.
            </p>
          </section>

          {/* Installing Extensions */}
          <section className="mb-14" id="extensions">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Installing Extensions</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Extensions add new capabilities to your AI agent. Browse the marketplace to find
              extensions for web scraping, file management, scheduling, and more.
            </p>

            <div className="space-y-3 mb-6">
              {[
                "Open the Extensions tab in Omni or visit the marketplace website",
                "Browse or search for an extension",
                "Click Install and review the permissions it requests",
                "Approve the permissions you're comfortable with",
                "The extension downloads, activates, and its tools become available",
              ].map((step, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{step}</p>
                </div>
              ))}
            </div>

            <div className="terminal mb-4">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">terminal</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> Install via CLI:</p>
                <p>omni ext install com.example.weather-tool</p>
                <p>omni ext list</p>
                <p>omni ext uninstall com.example.weather-tool</p>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section className="mb-14" id="usage">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Basic Usage</h2>

            <div className="space-y-6">
              {[
                { title: "Chat naturally", desc: "Ask your agent to do things in plain language. It will automatically choose the right tools." },
                { title: "Build visual workflows", desc: "Use the Flowchart Builder to create no-code automations with 19 node types. Flowcharts can call LLMs, make HTTP requests, send messages, and use all 29 native tools." },
                { title: "Channel routing", desc: "Use channel bindings to route incoming messages from specific channels to specific extensions." },
                { title: "Permission prompts", desc: "When a tool needs a permission you haven't pre-approved, Omni will prompt you. Allow once, always, or deny." },
                { title: "Multiple providers", desc: "Configure multiple LLM providers and Omni will automatically rotate with exponential backoff if one fails." },
              ].map((item, i) => (
                <div key={i} className="flex gap-4">
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
          </section>

          {/* Next Steps */}
          <section id="next-steps">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Next Steps</h2>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <Link
                href="/docs/flowcharts"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Build Visual Workflows
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Create automations with the drag-and-drop Flowchart Builder.
                </p>
                <span className="mt-3 flex items-center text-sm text-primary">
                  Flowchart Builder <ArrowRight className="h-3 w-3 ml-1" />
                </span>
              </Link>
              <Link
                href="/docs/sdk"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Build an Extension
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Learn the Omni SDK and create your own tools.
                </p>
                <span className="mt-3 flex items-center text-sm text-primary">
                  SDK Reference <ArrowRight className="h-3 w-3 ml-1" />
                </span>
              </Link>
              <Link
                href="/docs/publishing"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Publish to the Marketplace
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Share your extension with the Omni community.
                </p>
                <span className="mt-3 flex items-center text-sm text-primary">
                  Publishing Guide <ArrowRight className="h-3 w-3 ml-1" />
                </span>
              </Link>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
