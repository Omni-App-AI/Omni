import type { Metadata } from "next";
import Link from "next/link";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Publishing Guide — Deploy to Marketplace",
  description:
    "Publish your WASM extension to the Omni AI agent marketplace. Learn the CLI publishing workflow, API key setup, 4-layer security scanning process, semantic versioning, and review guidelines.",
  openGraph: {
    title: "Omni Publishing Guide — Deploy Extensions to Marketplace",
    description:
      "Publish your WASM extension to the Omni marketplace. Learn the CLI workflow, API key setup, 4-layer security scanning, semantic versioning, and review guidelines.",
    url: "/docs/publishing",
  },
  alternates: { canonical: "/docs/publishing" },
};

export default function PublishingGuidePage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-3xl">
          <p className="text-sm font-medium text-primary mb-3">Docs</p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">Publishing Guide</h1>
          <p className="text-muted-foreground mb-12">
            Publish your extension to the Omni Marketplace and share it with the community.
          </p>

          {/* Overview */}
          <section className="mb-14" id="overview">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Overview</h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Publishing an extension to the Omni Marketplace makes it available for anyone to
              discover, install, and use. The process is designed to be simple for developers
              while maintaining high security standards.
            </p>
            <div className="space-y-4">
              {[
                { title: "Build your extension", desc: "Write Rust code, compile to WASM, create a manifest" },
                { title: "Create an API key", desc: "Generate a publishing key from your dashboard" },
                { title: "Publish via CLI", desc: "Upload your WASM binary and manifest with one command" },
                { title: "Security scan", desc: "Our 4-layer pipeline automatically scans your submission" },
                { title: "Go live", desc: "Once the scan passes, your extension appears in the marketplace" },
              ].map((item, i) => (
                <div key={item.title} className="flex gap-4">
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

          {/* Prerequisites */}
          <section className="mb-14" id="prerequisites">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Prerequisites</h2>
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                A <Link href="/signup" className="text-primary hover:underline">developer account</Link> on the Omni Marketplace
              </li>
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                A built WASM extension with a valid <Link href="/docs/sdk#manifest" className="text-primary hover:underline">manifest.toml</Link>
              </li>
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                The Omni CLI installed (<code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">cargo install omni-cli</code>)
              </li>
              <li className="flex gap-2">
                <span className="text-muted-foreground/40 shrink-0">-</span>
                An API key for authentication (see below)
              </li>
            </ul>
          </section>

          {/* API Keys */}
          <section className="mb-14" id="api-keys">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Creating an API Key</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              API keys authenticate your CLI with the marketplace. You can create multiple keys
              for different machines or CI pipelines.
            </p>
            <div className="space-y-3 mb-6">
              {[
                "Log in to your Dashboard → API Keys",
                "Click Create API Key",
                "Give it a name (e.g., \"My Laptop\" or \"GitHub Actions\")",
                "Copy the generated key immediately — it won't be shown again",
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
                <p><span className="text-foreground/40">#</span> Key format:</p>
                <p>omni_pk_a1b2c3d4e5f6...  <span className="text-foreground/40">(72 characters)</span></p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> Store securely:</p>
                <p>export OMNI_API_KEY=<span className="text-success">&quot;omni_pk_a1b2c3d4e5f6...&quot;</span></p>
              </div>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Keys can be revoked from the dashboard at any time. We store only a SHA-256 hash
              of your key, never the key itself.
            </p>
          </section>

          {/* CLI Publishing */}
          <section className="mb-14" id="cli-publish">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Publishing via CLI</h2>
            <div className="terminal mb-4">
              <div className="terminal-header">
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <div className="terminal-dot" />
                <span className="ml-2 text-xs text-muted-foreground">omni ext publish</span>
              </div>
              <div className="p-5 code-block text-muted-foreground space-y-1">
                <p><span className="text-foreground/40">#</span> First-time publish</p>
                <p>omni ext publish --api-key $OMNI_API_KEY</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> The CLI will:</p>
                <p><span className="text-foreground/40">#</span>  1. Read manifest.toml</p>
                <p><span className="text-foreground/40">#</span>  2. Upload the WASM binary</p>
                <p><span className="text-foreground/40">#</span>  3. Verify SHA-256 checksum</p>
                <p><span className="text-foreground/40">#</span>  4. Create extension entry</p>
                <p><span className="text-foreground/40">#</span>  5. Trigger security scan</p>
                <p className="text-muted-foreground/60">&nbsp;</p>
                <p><span className="text-foreground/40">#</span> New version with changelog</p>
                <p>omni ext publish --api-key $OMNI_API_KEY \</p>
                <p>  --changelog <span className="text-success">&quot;Added temperature unit support&quot;</span></p>
              </div>
            </div>
          </section>

          {/* Security Scan */}
          <section className="mb-14" id="security-scan">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">The Security Scan</h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Every extension submission is automatically scanned by our 4-layer antivirus
              pipeline. This process typically takes 30-60 seconds.
            </p>

            <div className="space-y-4 mb-8">
              {[
                { layer: "Signature Scanning", weight: "30%", desc: "Checks the WASM binary against 79+ known malicious patterns — command injection, data exfiltration, cryptomining, and more." },
                { layer: "Heuristic Analysis", weight: "25%", desc: "Analyzes permission requests versus extension category. Flags suspicious combinations like a weather tool requesting filesystem write access." },
                { layer: "AI Code Review", weight: "30%", desc: "Claude analyzes your extension's manifest, permissions, and embedded strings for data exfiltration risk and hidden functionality." },
                { layer: "Sandbox Testing", weight: "15%", desc: "Executes your extension in an isolated environment with synthetic inputs. Monitors memory, CPU, attempted syscalls, and network calls." },
              ].map((layer, i) => (
                <div key={layer.layer} className="flex gap-4">
                  <div className="text-xs font-mono text-muted-foreground w-5 pt-0.5 shrink-0">
                    {String(i + 1).padStart(2, "0")}
                  </div>
                  <div className="flex-1 border-b border-border/50 pb-4">
                    <div className="flex items-baseline justify-between mb-1">
                      <h4 className="font-medium text-[15px]">{layer.layer}</h4>
                      <span className="text-xs font-mono text-muted-foreground">{layer.weight}</span>
                    </div>
                    <p className="text-sm text-muted-foreground">{layer.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </section>

          {/* Scan Verdicts */}
          <section className="mb-14" id="verdicts">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Scan Verdicts</h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Each layer produces a score from 0-100. The overall score is a weighted average,
              and the verdict determines what happens next:
            </p>
            <div className="space-y-0 border border-border/50 rounded-lg overflow-hidden mb-4">
              <div className="bg-card p-4 flex items-start gap-3">
                <div className="h-2 w-2 rounded-full bg-success shrink-0 mt-1.5" />
                <div>
                  <div className="font-medium text-sm">Clean (score &ge; 80, no layer below 60)</div>
                  <p className="text-xs text-muted-foreground mt-1">
                    Auto-approved. Published immediately to the marketplace.
                  </p>
                </div>
              </div>
              <div className="bg-card p-4 flex items-start gap-3 border-t border-border/50">
                <div className="h-2 w-2 rounded-full bg-warning shrink-0 mt-1.5" />
                <div>
                  <div className="font-medium text-sm">Suspicious (score 50-79)</div>
                  <p className="text-xs text-muted-foreground mt-1">
                    Flagged for manual review. Usually takes 1-3 business days.
                  </p>
                </div>
              </div>
              <div className="bg-card p-4 flex items-start gap-3 border-t border-border/50">
                <div className="h-2 w-2 rounded-full bg-destructive shrink-0 mt-1.5" />
                <div>
                  <div className="font-medium text-sm">Malicious (score &lt; 50 or critical flags)</div>
                  <p className="text-xs text-muted-foreground mt-1">
                    Auto-rejected. Detailed scan results explain what was flagged.
                  </p>
                </div>
              </div>
            </div>
            <p className="text-sm text-muted-foreground">
              View scan results from your{" "}
              <Link href="/dashboard/extensions" className="text-primary hover:underline">Dashboard &rarr; Extensions</Link>.
            </p>
          </section>

          {/* Versioning */}
          <section className="mb-14" id="versioning">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Versioning</h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              Extensions follow <strong className="text-foreground">semantic versioning</strong> (semver).
              The version in your manifest must be higher than the currently published version.
            </p>
            <div className="grid grid-cols-3 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden mb-6">
              {[
                { type: "Patch", example: "1.0.0 → 1.0.1", desc: "Bug fixes" },
                { type: "Minor", example: "1.0.0 → 1.1.0", desc: "New features" },
                { type: "Major", example: "1.0.0 → 2.0.0", desc: "Breaking changes" },
              ].map((v) => (
                <div key={v.type} className="bg-card p-4">
                  <div className="font-medium text-sm">{v.type}</div>
                  <code className="text-xs font-mono text-muted-foreground">{v.example}</code>
                  <p className="text-xs text-muted-foreground mt-1">{v.desc}</p>
                </div>
              ))}
            </div>
          </section>

          {/* Best Practices */}
          <section className="mb-14" id="best-practices">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Best Practices</h2>
            <div className="space-y-6">
              {[
                { title: "Minimize permissions", desc: "Only request capabilities your extension actually needs. Scope network access to specific domains. This improves your scan score and builds user trust." },
                { title: "Write clear descriptions", desc: "The AI code review checks whether your extension's behavior matches its description. A clear, accurate description helps pass the scan." },
                { title: "Include source code", desc: "Set the repository field in your manifest to link to your source code. This builds trust and may help you earn \"verified\" status." },
                { title: "Write meaningful changelogs", desc: "Include a changelog with every version update. Users can see changelogs on the extension detail page." },
                { title: "Respond to reviews", desc: "Address reported issues promptly and publish fixes to maintain a good rating." },
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

          {/* Troubleshooting */}
          <section id="troubleshooting">
            <h2 className="text-xl font-medium mb-4 pb-2 border-b border-border/50">Troubleshooting</h2>
            <div className="space-y-0 border border-border/50 rounded-lg overflow-hidden">
              {[
                { err: "Suspicious permission combination", fix: "Review your [capabilities] section and remove unused permissions. Scope network access to specific domains." },
                { err: "Signature match", fix: "Your binary contains patterns matching known malicious signatures. Review the flagged code and refactor if needed." },
                { err: "Sandbox timeout", fix: "Your extension exceeded CPU time limits. Check for infinite loops or expensive operations. Increase max_cpu_ms_per_call if genuinely needed." },
                { err: "Version already exists", fix: "Bump the version number in your manifest.toml. Each version must be unique and higher than the previous." },
              ].map((item, i) => (
                <div
                  key={i}
                  className={`bg-card p-4 ${i > 0 ? "border-t border-border/50" : ""}`}
                >
                  <h3 className="font-medium text-sm text-foreground mb-1">
                    &ldquo;{item.err}&rdquo;
                  </h3>
                  <p className="text-xs text-muted-foreground">{item.fix}</p>
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
