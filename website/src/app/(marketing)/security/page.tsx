import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";

export const metadata: Metadata = {
  title: "Security — WASM Sandboxing & AV Pipeline",
  description:
    "Explore how Omni protects AI agent users with a 4-layer antivirus pipeline, WASM sandboxed execution, capability-based permissions, and responsible disclosure for all marketplace extensions.",
  openGraph: {
    title: "Security — Omni WASM Sandboxing & 4-Layer AV Pipeline",
    description:
      "Explore how Omni protects AI agent users with a 4-layer antivirus pipeline, WASM sandboxed execution, capability-based permissions, and responsible disclosure for extensions.",
    url: "/security",
  },
  alternates: { canonical: "/security" },
};

const pipelineLayers = [
  {
    title: "Signature Scanning",
    weight: "30%",
    description:
      "Every WASM binary is checked against 79+ known malicious patterns. Our signature database covers command injection, data exfiltration, cryptomining payloads, reverse shells, and more. Patterns are compiled into a RegexSet for fast matching.",
    detects: [
      "Known malware byte sequences",
      "Command injection patterns",
      "Data exfiltration URLs and encoding tricks",
      "Cryptomining payloads and wallet addresses",
      "Base64/Unicode obfuscation attempts",
    ],
  },
  {
    title: "Heuristic Analysis",
    weight: "25%",
    description:
      "Behavioral rules analyze whether an extension's permission requests make sense for its stated purpose. A weather tool requesting filesystem write access or a calculator asking for network permissions will be flagged.",
    detects: [
      "Suspicious permission/category combinations",
      "Excessive permissions relative to functionality",
      "Missing or low-quality manifest metadata",
      "Unusual WASM imports and memory patterns",
      "Permission escalation attempts",
    ],
  },
  {
    title: "AI Code Review",
    weight: "30%",
    description:
      "Claude analyzes the extension's manifest, permission list, and embedded code patterns. The AI evaluates data exfiltration risk, checks whether behavior matches the description, and assesses whether permission requests are justified.",
    detects: [
      "Hidden functionality not in manifest",
      "Data exfiltration through covert channels",
      "Behavior mismatching description",
      "Unjustified permission requests",
      "Social engineering in descriptions",
    ],
  },
  {
    title: "Sandbox Testing",
    weight: "15%",
    description:
      "The extension is executed in a fully isolated WASM runtime with synthetic inputs. We monitor memory usage, CPU time, attempted syscalls, and network call attempts. This catches runtime threats that static analysis might miss.",
    detects: [
      "Infinite loops and excessive CPU usage",
      "Memory allocation bombs",
      "Attempted syscalls outside the sandbox",
      "Network access to unexpected hosts",
      "Sandbox escape techniques",
    ],
  },
];

const permissions = [
  { name: "HTTP Network Access", scope: "network.http", severity: "medium" },
  { name: "WebSocket Access", scope: "network.websocket", severity: "medium" },
  { name: "File System Read", scope: "filesystem.read", severity: "high" },
  { name: "File System Write", scope: "filesystem.write", severity: "high" },
  { name: "AI Inference", scope: "ai.inference", severity: "medium" },
  { name: "Channel Messaging", scope: "channel.send", severity: "high" },
  { name: "Web Scraping", scope: "browser.scrape", severity: "high" },
  { name: "Clipboard Read", scope: "clipboard.read", severity: "medium" },
  { name: "Clipboard Write", scope: "clipboard.write", severity: "low" },
  { name: "Notifications", scope: "system.notifications", severity: "low" },
  { name: "Task Scheduling", scope: "system.scheduling", severity: "low" },
  { name: "Persistent Storage", scope: "storage.persistent", severity: "low" },
];

const severityColors: Record<string, string> = {
  low: "text-success",
  medium: "text-warning",
  high: "text-destructive",
};

export default function SecurityPage() {
  return (
    <div>
      {/* Hero */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 pt-20 pb-16 md:pt-28 md:pb-24">
          <div className="max-w-2xl">
            <p className="text-sm font-medium text-muted-foreground mb-4">
              Security
            </p>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight leading-[1.1]">
              Every extension is scanned.
              <br />
              <span className="text-gradient">No exceptions.</span>
            </h1>
            <p className="mt-6 text-base md:text-lg text-muted-foreground max-w-lg leading-relaxed">
              Our 4-layer antivirus pipeline analyzes every submission before it
              reaches you. Extensions run in isolated WASM sandboxes with
              capability-based permissions.
            </p>
          </div>
        </div>
      </section>

      {/* 4-Layer Pipeline */}
      <section className="border-y border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-3 gap-12 mb-16">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Pipeline</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                4-layer security pipeline
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Each layer produces a score from 0-100. The weighted average
                determines the verdict. No single point of failure.
              </p>
            </div>
          </div>

          <div className="space-y-12">
            {pipelineLayers.map((layer, i) => (
              <div key={layer.title} className="grid lg:grid-cols-2 gap-8">
                <div>
                  <div className="flex items-baseline gap-3 mb-3">
                    <span className="text-xs font-mono text-muted-foreground">
                      {String(i + 1).padStart(2, "0")}
                    </span>
                    <h3 className="text-lg font-medium">{layer.title}</h3>
                    <span className="text-xs font-mono text-muted-foreground ml-auto">
                      {layer.weight}
                    </span>
                  </div>
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    {layer.description}
                  </p>
                </div>
                <div className="border border-border/50 rounded-lg overflow-hidden">
                  <div className="px-4 py-2 border-b border-border/50">
                    <span className="text-xs font-mono text-muted-foreground/60">Detects</span>
                  </div>
                  {layer.detects.map((item, j) => (
                    <div
                      key={j}
                      className={`px-4 py-2 text-sm text-muted-foreground ${j > 0 ? "border-t border-border/50" : ""}`}
                    >
                      {item}
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* WASM Sandboxing */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-2 gap-16">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Isolation</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                WASM Sandboxing
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Every extension runs inside an isolated WebAssembly sandbox powered
                by Wasmtime. Extensions cannot access your system directly — they
                can only use host functions that you&apos;ve explicitly granted through
                the permission system.
              </p>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                The sandbox enforces strict resource limits: maximum memory allocation,
                CPU time per tool call, and concurrent execution limits. If an extension
                exceeds its limits, it&apos;s terminated immediately without affecting other
                extensions or the Omni runtime.
              </p>
            </div>
            <div className="grid grid-cols-2 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <div className="bg-card p-5">
                <h3 className="text-xs font-mono text-destructive/70 mb-3">Cannot access</h3>
                <div className="space-y-2">
                  {["Raw filesystem", "Network sockets", "System processes", "Other extensions' data", "Clipboard (without permission)", "Environment variables"].map((item) => (
                    <p key={item} className="text-sm text-muted-foreground flex items-center gap-2">
                      <span className="h-1.5 w-1.5 rounded-full bg-destructive/50 shrink-0" />
                      {item}
                    </p>
                  ))}
                </div>
              </div>
              <div className="bg-card p-5">
                <h3 className="text-xs font-mono text-success/70 mb-3">With permission</h3>
                <div className="space-y-2">
                  {["HTTP to allowed domains", "Scoped file read/write", "LLM inference via bridge", "Channel messaging", "Extension key-value store"].map((item) => (
                    <p key={item} className="text-sm text-muted-foreground flex items-center gap-2">
                      <span className="h-1.5 w-1.5 rounded-full bg-success/50 shrink-0" />
                      {item}
                    </p>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Permission Model */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="max-w-2xl mb-12">
            <p className="text-sm font-medium text-primary mb-3">Permissions</p>
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
              Capability-based permissions
            </h2>
            <p className="mt-4 text-muted-foreground leading-relaxed">
              Extensions declare what they need in their manifest. You review and
              approve permissions before installation. Every capability has a
              severity level.
            </p>
          </div>

          <div className="border border-border/50 rounded-lg overflow-hidden">
            <div className="grid grid-cols-3 px-4 py-2 border-b border-border/50 text-xs font-mono text-muted-foreground/60">
              <span>Permission</span>
              <span>Scope</span>
              <span className="text-right">Severity</span>
            </div>
            {permissions.map((perm, i) => (
              <div
                key={perm.scope}
                className={`grid grid-cols-3 px-4 py-2.5 bg-card text-sm ${i > 0 ? "border-t border-border/50" : ""}`}
              >
                <span className="text-foreground text-[13px]">{perm.name}</span>
                <code className="text-xs font-mono text-muted-foreground">{perm.scope}</code>
                <span className={`text-right text-xs font-mono ${severityColors[perm.severity]}`}>
                  {perm.severity}
                </span>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Continuous Monitoring */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-2 gap-16 items-start">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Monitoring</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                Continuous monitoring
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed max-w-md">
                Security doesn&apos;t stop at initial publication. We continuously
                monitor the extension ecosystem to catch threats that emerge
                after initial review.
              </p>
            </div>
            <div className="space-y-6">
              {[
                { title: "Weekly re-scans", desc: "All published extensions are periodically re-scanned against updated signature databases to catch newly discovered threats." },
                { title: "Updated signatures", desc: "Our signature database is regularly updated with new patterns discovered by the security community and our own research." },
                { title: "Publisher notifications", desc: "If a re-scan detects issues in a previously clean extension, the publisher is notified immediately and given time to address findings." },
                { title: "Community reporting", desc: "Users can report suspicious extensions through the marketplace. Reports trigger additional scans and human review." },
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
          </div>
        </div>
      </section>

      {/* Trust Levels */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-3 gap-12">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Trust</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                Trust levels
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Extensions are classified based on their publisher&apos;s track
                record and verification status.
              </p>
            </div>

            <div className="lg:col-span-2 grid sm:grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
              <div className="bg-card p-6">
                <h3 className="font-medium text-[15px] mb-1">Verified</h3>
                <p className="text-xs font-mono text-success mb-3">Highest trust</p>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  Publisher identity verified. Source code reviewed. Manually
                  audited by the Omni security team.
                </p>
              </div>
              <div className="bg-card p-6">
                <h3 className="font-medium text-[15px] mb-1">Community</h3>
                <p className="text-xs font-mono text-primary mb-3">Standard</p>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  Published by an established member with positive track record.
                  Passed automated scanning. Most extensions fall here.
                </p>
              </div>
              <div className="bg-card p-6">
                <h3 className="font-medium text-[15px] mb-1">Unverified</h3>
                <p className="text-xs font-mono text-muted-foreground mb-3">New</p>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  New publisher or extension without track record. Passed automated
                  scanning but not manually reviewed. Review permissions carefully.
                </p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Responsible Disclosure */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-2 gap-16 items-start">
            <div>
              <p className="text-sm font-medium text-primary mb-3">Disclosure</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                Responsible disclosure
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed max-w-md">
                We take security vulnerabilities seriously. If you discover a
                security issue in the Omni platform, marketplace, or any published
                extension, please report it responsibly.
              </p>
              <p className="mt-4 text-muted-foreground leading-relaxed max-w-md">
                We acknowledge valid reports within 48 hours, provide regular
                status updates, and do not pursue legal action against researchers
                who act in good faith.
              </p>
            </div>
            <div>
              <div className="border border-border/50 rounded-lg overflow-hidden">
                <div className="px-4 py-2 border-b border-border/50">
                  <span className="text-xs font-mono text-muted-foreground/60">How to report</span>
                </div>
                {[
                  "Email security@omniapp.org with a detailed description",
                  "Include steps to reproduce, affected versions, and impact",
                  "Allow 90 days to address the issue before public disclosure",
                  "Do not exploit beyond what is necessary for demonstration",
                ].map((step, i) => (
                  <div
                    key={i}
                    className={`px-4 py-3 flex gap-3 ${i > 0 ? "border-t border-border/50" : ""}`}
                  >
                    <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 pt-0.5">
                      {String(i + 1).padStart(2, "0")}
                    </span>
                    <p className="text-sm text-muted-foreground">{step}</p>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section>
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="max-w-lg">
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
              Built for trust
            </h2>
            <p className="mt-3 text-muted-foreground leading-relaxed">
              Browse the marketplace with confidence. Every extension has been
              scanned, sandboxed, and permission-gated for your safety.
            </p>
            <div className="mt-8 flex items-center gap-3">
              <Link href="/extensions">
                <Button size="xl">
                  Browse extensions
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </Link>
              <Link href="/docs/publishing#security-scan">
                <Button size="xl" variant="outline">
                  Learn about scanning
                </Button>
              </Link>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
