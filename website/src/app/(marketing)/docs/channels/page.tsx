import type { Metadata } from "next";
import Link from "next/link";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Channels — Discord, Telegram & 19 More",
  description:
    "Connect the Omni AI agent to 21+ messaging platforms including Discord, Telegram, Slack, and WhatsApp. Set up multi-instance support, message routing, and per-channel configuration.",
  openGraph: {
    title: "Omni Channels — Connect Discord, Telegram, Slack & 18 More",
    description:
      "Connect the Omni AI agent to 21+ messaging platforms including Discord, Telegram, Slack, and WhatsApp. Multi-instance support and message routing included.",
    url: "/docs/channels",
  },
  alternates: { canonical: "/docs/channels" },
};

const channels = [
  { name: "Discord", auth: "Bot token", lib: "Serenity WebSocket", limit: "2,000 chars", features: "DM, Groups, Media, Reactions" },
  { name: "Telegram", auth: "Bot token", lib: "Teloxide long-polling", limit: "4,096 chars", features: "DM, Groups, Media, Reactions" },
  { name: "WhatsApp Web", auth: "QR code pairing", lib: "Baileys sidecar", limit: "65,536 chars", features: "DM, Groups, Media, Read Receipts" },
  { name: "Slack", auth: "Bot token / OAuth", lib: "Webhook + Events API", limit: "40,000 chars", features: "DM, Groups, Media, Reactions" },
  { name: "Teams", auth: "Azure Bot Service", lib: "Bot Framework", limit: "28,000 chars", features: "DM, Groups, Media" },
  { name: "Google Chat", auth: "Service account", lib: "Google API", limit: "4,096 chars", features: "DM, Groups" },
  { name: "Matrix", auth: "Access token", lib: "Matrix protocol", limit: "65,536 chars", features: "DM, Groups, Media, Reactions" },
  { name: "IRC", auth: "Server + nick", lib: "IRC protocol", limit: "512 chars/msg", features: "Groups" },
  { name: "Twitch", auth: "OAuth token", lib: "IRC/TMI", limit: "500 chars", features: "Groups" },
  { name: "Signal", auth: "QR code pairing", lib: "Signal protocol", limit: "65,536 chars", features: "DM, Groups, Media, Read Receipts" },
  { name: "LINE", auth: "Channel token", lib: "Messaging API", limit: "5,000 chars", features: "DM, Groups, Media" },
  { name: "Mattermost", auth: "Bot token", lib: "Webhook API", limit: "16,383 chars", features: "DM, Groups, Media, Reactions" },
  { name: "Feishu", auth: "App credentials", lib: "Feishu API", limit: "4,096 chars", features: "DM, Groups, Media" },
  { name: "Nostr", auth: "Private key (nsec)", lib: "Nostr protocol", limit: "65,536 chars", features: "DM, Groups" },
  { name: "Nextcloud Talk", auth: "App password", lib: "REST API", limit: "32,000 chars", features: "DM, Groups" },
  { name: "Synology Chat", auth: "Bot token", lib: "Webhook API", limit: "4,096 chars", features: "DM, Groups" },
  { name: "Twitter/X", auth: "OAuth 1.0a", lib: "X API v2", limit: "280 chars", features: "DM, Mentions" },
  { name: "BlueBubbles", auth: "Server URL + password", lib: "REST API", limit: "20,000 chars", features: "DM, Groups, Media, Read Receipts" },
  { name: "iMessage", auth: "BlueBubbles bridge", lib: "BlueBubbles API", limit: "20,000 chars", features: "DM, Groups, Media, Read Receipts" },
  { name: "Zalo", auth: "App credentials", lib: "Zalo API", limit: "2,000 chars", features: "DM, Groups, Media" },
  { name: "WebChat", auth: "None (local)", lib: "WebSocket", limit: "Unlimited", features: "DM" },
];

export default function ChannelsPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-4xl">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
            Guide
          </p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">
            Channels Guide
          </h1>
          <p className="text-muted-foreground mb-12">
            Connect Omni to 21 messaging platforms. Route incoming messages to
            extensions with bindings.
          </p>

          {/* On this page */}
          <nav className="border border-border/50 rounded-lg p-5 mb-14 bg-card/30">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-3">
              On this page
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
              {[
                { href: "#overview", label: "Overview" },
                { href: "#supported-channels", label: "Supported Channels" },
                { href: "#connecting", label: "Connecting a Channel" },
                { href: "#multi-instance", label: "Multi-Instance" },
                { href: "#bindings", label: "Message Routing" },
                { href: "#features", label: "Channel Features" },
                { href: "#auth-methods", label: "Auth Methods" },
                { href: "#troubleshooting", label: "Troubleshooting" },
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
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Overview
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Channels are messaging platform integrations that let your Omni agent send and receive
              messages through external services. Each channel runs as an independent plugin with its
              own connection lifecycle, authentication, and message format.
            </p>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Key concepts:
            </p>
            <div className="space-y-4 mb-6">
              {[
                { title: "Channel Plugin", desc: "An adapter that speaks a specific platform's protocol (e.g., Discord WebSocket, Telegram polling)." },
                { title: "Channel Instance", desc: "A configured connection to a platform. You can have multiple instances of the same type (e.g., two Discord bots)." },
                { title: "Channel Binding", desc: "A routing rule that maps incoming messages from a channel to a specific extension." },
                { title: "Compound Key", desc: "Every instance is identified by a compound key in the format type:instance_id (e.g., \"discord:production\")." },
              ].map((item, i) => (
                <div key={i} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {String(i + 1).padStart(2, "0")}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-4">
                    <span className="font-medium text-[15px]">{item.title}</span>
                    <span className="text-sm text-muted-foreground"> — {item.desc}</span>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Supported Channels */}
          <section className="mb-14" id="supported-channels">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Supported Channels
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Omni supports 21 messaging platforms. All channels are free to use — no premium tiers
              or paid APIs required.
            </p>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[minmax(100px,1fr)_1fr_1fr_auto_1fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Channel</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Auth</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Protocol</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Limit</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Features</div>
                {channels.map((ch) => (
                  <>
                    <div key={`n-${ch.name}`} className="bg-card px-3 py-2 text-sm font-medium">{ch.name}</div>
                    <div key={`a-${ch.name}`} className="bg-card px-3 py-2 text-xs text-muted-foreground">{ch.auth}</div>
                    <div key={`l-${ch.name}`} className="bg-card px-3 py-2 text-xs text-muted-foreground font-mono">{ch.lib}</div>
                    <div key={`lm-${ch.name}`} className="bg-card px-3 py-2 text-xs text-muted-foreground font-mono">{ch.limit}</div>
                    <div key={`f-${ch.name}`} className="bg-card px-3 py-2 text-xs text-muted-foreground">{ch.features}</div>
                  </>
                ))}
              </div>
            </div>
          </section>

          {/* Connecting a Channel */}
          <section className="mb-14" id="connecting">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Connecting a Channel
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Channels can be connected through the UI or pre-configured in{" "}
              <code className="px-1.5 py-0.5 bg-secondary rounded text-sm font-mono">omni.toml</code>.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Via the UI</h3>
            <div className="space-y-3 mb-8">
              {[
                "Open Settings \u2192 Channels",
                "Click Add Instance and select a channel type",
                "Enter an instance ID (e.g., \"production\") and optional display name",
                "Enter authentication credentials (bot token, API key, etc.)",
                "Click Connect — the channel status indicator turns green when ready",
              ].map((step, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{step}</p>
                </div>
              ))}
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Via Config File</h3>
            <div className="terminal mb-6">
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
                <p><span className="text-foreground/40">#</span> Credentials are provided at connect time, not stored in config</p>
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Connection Lifecycle</h3>
            <div className="grid grid-cols-5 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { status: "Disconnected", color: "text-muted-foreground", desc: "No active connection" },
                { status: "Connecting", color: "text-warning", desc: "Establishing connection" },
                { status: "Connected", color: "text-success", desc: "Ready to send/receive" },
                { status: "Reconnecting", color: "text-warning", desc: "Recovering from a drop" },
                { status: "Error", color: "text-destructive", desc: "Connection failed" },
              ].map((s) => (
                <div key={s.status} className="bg-card px-3 py-3 text-center">
                  <p className={`text-xs font-mono font-medium ${s.color} mb-1`}>{s.status}</p>
                  <p className="text-[11px] text-muted-foreground">{s.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Multi-Instance */}
          <section className="mb-14" id="multi-instance">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Multi-Instance Support
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              You can run multiple instances of the same channel type simultaneously. Each instance
              has its own credentials, connection state, and message stream. Instances are identified
              by compound keys.
            </p>
            <div className="terminal mb-6">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">compound keys</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> Format: {"{channel_type}:{instance_id}"}</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-success">&quot;discord:production&quot;</span>    <span className="text-foreground/40">#</span> Discord, production instance</p>
                <p><span className="text-success">&quot;discord:staging&quot;</span>       <span className="text-foreground/40">#</span> Discord, staging instance</p>
                <p><span className="text-success">&quot;telegram:alerts&quot;</span>       <span className="text-foreground/40">#</span> Telegram, alerts bot</p>
                <p><span className="text-success">&quot;telegram:support&quot;</span>      <span className="text-foreground/40">#</span> Telegram, support bot</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Bare type defaults to &quot;default&quot; instance:</p>
                <p><span className="text-success">&quot;discord&quot;</span> &rarr; <span className="text-success">&quot;discord:default&quot;</span></p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Use cases: separate bots for production and development, multiple Telegram bots for
              different teams, or different Slack workspaces connected at once.
            </p>
          </section>

          {/* Message Routing / Bindings */}
          <section className="mb-14" id="bindings">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Message Routing &amp; Bindings
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Channel bindings route incoming messages from a channel instance to a specific extension.
              Without a binding, messages from a channel are not forwarded to any extension.
            </p>

            <h3 className="text-sm font-medium text-foreground mb-3">Binding Fields</h3>
            <div className="border border-border/50 rounded-lg overflow-hidden mb-6">
              <div className="grid grid-cols-[1fr_auto_2fr] gap-px bg-border/50">
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Field</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Required</div>
                <div className="bg-card/60 px-3 py-2 text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">Description</div>
                {[
                  { field: "channel_instance", req: "Yes", desc: "Compound key of the channel instance (e.g., \"discord:production\")." },
                  { field: "extension_id", req: "Yes", desc: "Reverse-domain ID of the extension to route to." },
                  { field: "peer_filter", req: "No", desc: "Glob pattern matching sender usernames. \"*\" matches all." },
                  { field: "group_filter", req: "No", desc: "Glob pattern matching group/channel names. \"support-*\" matches support-general, support-vip, etc." },
                  { field: "priority", req: "No", desc: "Integer priority for conflict resolution. Higher values take precedence." },
                  { field: "enabled", req: "No", desc: "Toggle the binding on or off without removing it." },
                ].map((row) => (
                  <>
                    <div key={`f-${row.field}`} className="bg-card px-3 py-2 text-sm font-mono text-primary/80">{row.field}</div>
                    <div key={`r-${row.field}`} className="bg-card px-3 py-2 text-sm text-muted-foreground text-center">{row.req}</div>
                    <div key={`d-${row.field}`} className="bg-card px-3 py-2 text-sm text-muted-foreground">{row.desc}</div>
                  </>
                ))}
              </div>
            </div>

            <h3 className="text-sm font-medium text-foreground mb-3">Resolution Order</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              When multiple bindings match an incoming message, Omni resolves the conflict using:
            </p>
            <div className="space-y-3 mb-6">
              {[
                "Priority — higher priority bindings are preferred",
                "Specificity — more specific glob patterns (fewer wildcards) win over broader ones",
                "First match — if still tied, the first registered binding wins",
              ].map((step, i) => (
                <div key={i} className="flex gap-3">
                  <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 pt-0.5">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <p className="text-sm text-muted-foreground">{step}</p>
                </div>
              ))}
            </div>

            <div className="terminal">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni.toml</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> Route all Discord support channels to the support bot</p>
                <p><span className="text-foreground/60">[[channels.bindings]]</span></p>
                <p>channel_instance = <span className="text-success">&quot;discord:production&quot;</span></p>
                <p>extension_id = <span className="text-success">&quot;com.example.support-bot&quot;</span></p>
                <p>group_filter = <span className="text-success">&quot;support-*&quot;</span></p>
                <p>priority = <span className="text-warning">100</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Catch-all for everything else on Discord</p>
                <p><span className="text-foreground/60">[[channels.bindings]]</span></p>
                <p>channel_instance = <span className="text-success">&quot;discord:production&quot;</span></p>
                <p>extension_id = <span className="text-success">&quot;com.example.general-bot&quot;</span></p>
                <p>peer_filter = <span className="text-success">&quot;*&quot;</span></p>
                <p>priority = <span className="text-warning">10</span></p>
              </div>
            </div>
          </section>

          {/* Channel Features */}
          <section className="mb-14" id="features">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Channel Features
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Each channel reports which features it supports. Extensions can query these features
              to adapt their behavior.
            </p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              {[
                { feature: "Direct Messages", desc: "1-on-1 conversations" },
                { feature: "Group Messages", desc: "Multi-user channels and rooms" },
                { feature: "Media Attachments", desc: "Images, files, and other media" },
                { feature: "Reactions", desc: "Emoji reactions on messages" },
                { feature: "Read Receipts", desc: "Delivery and read confirmations" },
                { feature: "Typing Indicators", desc: "Show when someone is typing" },
              ].map((f) => (
                <div key={f.feature} className="bg-card px-4 py-3">
                  <p className="text-sm font-medium mb-0.5">{f.feature}</p>
                  <p className="text-xs text-muted-foreground">{f.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Auth Methods */}
          <section className="mb-14" id="auth-methods">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Authentication Methods
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Channels use different authentication flows depending on the platform.
            </p>
            <div className="space-y-6">
              {[
                {
                  title: "Bot Token",
                  channels: "Discord, Telegram, Slack, Mattermost, Synology Chat",
                  desc: "Create a bot on the platform's developer portal, copy the token, and paste it into Omni. The token is stored securely and used for all API calls.",
                },
                {
                  title: "QR Code Pairing",
                  channels: "WhatsApp Web, Signal",
                  desc: "Omni displays a QR code in the UI. Scan it with your phone's app to link the session. No bot account needed — messages appear as coming from your personal account.",
                },
                {
                  title: "OAuth / API Key",
                  channels: "Twitter/X, LINE, Google Chat, Feishu, Zalo",
                  desc: "Register an application on the platform's developer console, obtain API keys or OAuth credentials, and enter them in Omni.",
                },
                {
                  title: "Server URL + Password",
                  channels: "BlueBubbles, iMessage, Nextcloud Talk",
                  desc: "Point Omni to a self-hosted server (e.g., BlueBubbles on a Mac) with its URL and password.",
                },
              ].map((item, i) => (
                <div key={i} className="border-b border-border/50 pb-6">
                  <h3 className="font-medium text-[15px] mb-1">{item.title}</h3>
                  <p className="text-xs text-muted-foreground/60 font-mono mb-2">{item.channels}</p>
                  <p className="text-sm text-muted-foreground">{item.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Troubleshooting */}
          <section className="mb-14" id="troubleshooting">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Troubleshooting
            </h2>
            <div className="space-y-6">
              {[
                {
                  q: "Channel stuck on \"Connecting\"",
                  a: "Check your credentials and internet connection. For bot-token channels, verify the token hasn't been revoked. For QR code channels, try disconnecting and re-scanning.",
                },
                {
                  q: "Messages not arriving",
                  a: "Ensure a binding exists for the channel instance. Without a binding, incoming messages are received but not routed to any extension. Check Settings \u2192 Channels \u2192 Bindings.",
                },
                {
                  q: "\"Permission denied\" when sending",
                  a: "The extension needs the channel.send capability. Check the extension's manifest or grant the permission when prompted.",
                },
                {
                  q: "WhatsApp Web disconnects frequently",
                  a: "WhatsApp Web sessions can expire if the phone loses internet or the app is force-closed. Keep the phone connected and the WhatsApp app running in the background.",
                },
              ].map((item, i) => (
                <div key={i} className="border-b border-border/50 pb-6">
                  <h3 className="font-medium text-[15px] mb-2">{item.q}</h3>
                  <p className="text-sm text-muted-foreground">{item.a}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Next Steps */}
          <section id="next-steps">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">
              Next Steps
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <Link
                href="/docs/configuration#channels"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  Channel Configuration
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Full config reference for instances and bindings.
                </p>
              </Link>
              <Link
                href="/docs/sdk#context"
                className="bg-card p-5 hover:bg-card/80 transition-colors group"
              >
                <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                  ChannelClient SDK
                </h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Send messages from extensions using the SDK.
                </p>
              </Link>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
