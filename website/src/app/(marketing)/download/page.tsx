import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRight, ExternalLink } from "lucide-react";
import { headers } from "next/headers";
import { Button } from "@/components/ui/button";
import { PlatformCard } from "@/components/download/PlatformCard";
import { SystemRequirements } from "@/components/download/SystemRequirements";
import { createServiceClient } from "@/lib/supabase/server";

export const dynamic = "force-dynamic";

export const metadata: Metadata = {
  title: "Download Omni — Free AI Agent Builder for Windows, macOS & Linux",
  description:
    "Download Omni for free. Build and run AI agents to automate any task on Windows, macOS, or Linux. Connect 21+ channels, use 29 built-in tools, choose from 6 LLM providers, and keep everything private and local.",
  openGraph: {
    title: "Download Omni — Free AI Agent Builder for Windows, macOS & Linux",
    description:
      "Build AI agents to automate any task. Download Omni free for Windows, macOS, or Linux. 21+ channels, 29 tools, 6 LLM providers, fully local and private.",
    url: "/download",
  },
  alternates: { canonical: "/download" },
};

// ── OS icon SVGs (lucide-react doesn't have OS logos) ──────

function WindowsIcon() {
  return (
    <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
      <path d="M0 3.449L9.75 2.1v9.451H0m10.949-9.602L24 0v11.4H10.949M0 12.6h9.75v9.451L0 20.699M10.949 12.6H24V24l-12.9-1.801" />
    </svg>
  );
}

function AppleIcon() {
  return (
    <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
      <path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11" />
    </svg>
  );
}

function LinuxIcon() {
  return (
    <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12.504 0c-.155 0-.315.008-.48.021-4.226.333-3.105 4.807-3.17 6.298-.076 1.092-.3 1.953-1.05 3.02-.885 1.051-2.127 2.75-2.716 4.521-.278.832-.41 1.684-.287 2.489a.424.424 0 00-.11.135c-.26.268-.45.6-.663.839-.199.199-.485.267-.797.4-.313.136-.658.269-.864.68-.09.189-.136.394-.132.602 0 .199.027.4.055.536.058.399.116.728.04.97-.249.68-.28 1.145-.106 1.484.174.334.535.47.94.601.81.2 1.91.135 2.774.6.926.466 1.866.67 2.616.47.526-.116.97-.464 1.208-.946.587-.003 1.23-.269 2.26-.334.699-.058 1.574.267 2.577.2.025.134.063.198.114.333l.003.003c.391.778 1.113 1.368 1.884 1.43.868.074 1.741-.313 2.452-.6.949-.38 1.846-.792 2.44-.792.543.202 1.3.535 1.952.422.551-.1.92-.398 1.108-.814.49-1.09-.27-1.942-.58-2.863a2.476 2.476 0 01-.046-.145 2.91 2.91 0 00-.12-.476c.12-.27.18-.596.18-.936 0-.71-.27-1.34-.51-1.676a.47.47 0 00-.106-.127c-.074-.098-.182-.178-.265-.267a.792.792 0 01-.136-.354c-.068-.408-.068-1.082-.178-1.624-.081-.484-.245-1.013-.652-1.342-.176-.176-.44-.256-.67-.38-.396-.21-.746-.414-.746-1.162 0-1.186.094-3.072-.413-4.443C16.67 1.463 14.73 0 12.504 0z" />
    </svg>
  );
}

// ── Data fetching ──────────────────────────────────────────

interface PlatformAsset {
  url: string;
  signature: string;
  size_bytes: number;
  asset_name: string;
  installer_type?: string;
}

interface ReleaseData {
  version: string;
  channel: string;
  release_notes: string;
  published_at: string;
  is_prerelease: boolean;
  platforms: Record<string, PlatformAsset>;
}

async function getLatestRelease(): Promise<ReleaseData | null> {
  try {
    const supabase = createServiceClient();
    const { data, error } = await (supabase.from("app_releases") as any)
      .select("*")
      .eq("channel", "stable")
      .eq("is_draft", false)
      .order("published_at", { ascending: false })
      .limit(1)
      .single();

    if (error || !data) return null;

    return {
      version: data.version,
      channel: data.channel,
      release_notes: data.release_notes,
      published_at: data.published_at,
      is_prerelease: data.is_prerelease,
      platforms: data.platforms,
    };
  } catch {
    return null;
  }
}

function detectPlatform(userAgent: string): string {
  const ua = userAgent.toLowerCase();
  if (ua.includes("win")) return "windows";
  if (ua.includes("mac") || ua.includes("darwin")) return "macos";
  if (ua.includes("linux")) return "linux";
  return "windows";
}

// ── Page ───────────────────────────────────────────────────

export default async function DownloadPage() {
  const release = await getLatestRelease();
  const headerList = await headers();
  const userAgent = headerList.get("user-agent") || "";
  const detectedOS = detectPlatform(userAgent);

  return (
    <div>
      {/* Hero */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 pt-20 pb-16 md:pt-28 md:pb-24">
          <div className="max-w-2xl">
            <p className="text-sm font-medium text-muted-foreground mb-4">
              Download
            </p>
            <h1 className="text-4xl md:text-5xl font-bold tracking-tight leading-[1.1]">
              Download Omni for
              <br />
              <span className="text-gradient">Windows, macOS & Linux.</span>
            </h1>
            <p className="mt-6 text-base md:text-lg text-muted-foreground max-w-lg leading-relaxed">
              Build AI agents that automate any task — right from your desktop.
              Connect 21+ channels, use 29 built-in tools, and keep your data
              fully private. Free and open source.
            </p>
            {release && (
              <p className="mt-3 text-xs font-mono text-muted-foreground">
                Latest: v{release.version} &middot;{" "}
                {new Date(release.published_at).toLocaleDateString()}
              </p>
            )}
          </div>
        </div>
      </section>

      {/* Platform Cards */}
      <section className="border-y border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-20">
          {release ? (
            <div className="grid md:grid-cols-3 gap-6 max-w-4xl mx-auto">
              <PlatformCard
                name="Windows"
                platformKey="windows-x86_64"
                asset={release.platforms["windows-x86_64"] || null}
                version={release.version}
                isDetected={detectedOS === "windows"}
                icon={<WindowsIcon />}
                installerLabel=".exe installer (NSIS)"
              />
              <PlatformCard
                name="macOS"
                platformKey="darwin-aarch64"
                asset={release.platforms["darwin-aarch64"] || null}
                version={release.version}
                isDetected={detectedOS === "macos"}
                icon={<AppleIcon />}
                installerLabel=".dmg (Apple Silicon)"
                secondaryAsset={{
                  label: "Intel (x86_64)",
                  platformKey: "darwin-x86_64",
                  asset: release.platforms["darwin-x86_64"] || null,
                }}
              />
              <PlatformCard
                name="Linux"
                platformKey="linux-x86_64"
                asset={release.platforms["linux-x86_64"] || null}
                version={release.version}
                isDetected={detectedOS === "linux"}
                icon={<LinuxIcon />}
                installerLabel=".AppImage / .deb"
              />
            </div>
          ) : (
            <div className="max-w-md mx-auto text-center py-12">
              <h2 className="text-xl font-semibold mb-3">
                First release coming soon
              </h2>
              <p className="text-sm text-muted-foreground leading-relaxed mb-6">
                Omni is currently in development. Star the repository to get
                notified when the first release is available.
              </p>
              <Link
                href="https://github.com/OWNER/omni"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Button variant="outline">
                  Star on GitHub
                  <ExternalLink className="h-3.5 w-3.5" />
                </Button>
              </Link>
            </div>
          )}
        </div>
      </section>

      {/* System Requirements */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="grid lg:grid-cols-3 gap-12">
            <div>
              <p className="text-sm font-medium text-primary mb-3">
                Requirements
              </p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
                System requirements
              </h2>
              <p className="mt-4 text-muted-foreground leading-relaxed">
                Omni runs on all major desktop platforms with minimal
                dependencies.
              </p>
            </div>
            <div className="lg:col-span-2">
              <SystemRequirements />
            </div>
          </div>
        </div>
      </section>

      {/* Additional Links */}
      <section className="border-b border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-20">
          <div className="grid sm:grid-cols-3 gap-8">
            <div>
              <h3 className="font-medium text-[15px] mb-2">GitHub Releases</h3>
              <p className="text-sm text-muted-foreground mb-3">
                Download older versions or view checksums for all release
                assets.
              </p>
              <Link
                href="https://github.com/OWNER/omni/releases"
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-primary hover:underline"
              >
                View all releases
              </Link>
            </div>
            <div>
              <h3 className="font-medium text-[15px] mb-2">Changelog</h3>
              <p className="text-sm text-muted-foreground mb-3">
                See what changed in each version, including new features, bug
                fixes, and improvements.
              </p>
              <Link
                href="/docs/changelog"
                className="text-sm text-primary hover:underline"
              >
                Read changelog
              </Link>
            </div>
            <div>
              <h3 className="font-medium text-[15px] mb-2">
                Build from Source
              </h3>
              <p className="text-sm text-muted-foreground mb-3">
                Omni is open source. Clone the repo and build it yourself.
              </p>
              <Link
                href="/docs/building"
                className="text-sm text-primary hover:underline"
              >
                Build instructions
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
              Extend your agent
            </h2>
            <p className="mt-3 text-muted-foreground leading-relaxed">
              After installing Omni, browse the marketplace for extensions that
              add new tools, channels, and capabilities to your AI agent.
            </p>
            <div className="mt-8 flex items-center gap-3">
              <Link href="/extensions">
                <Button size="xl">
                  Browse extensions
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </Link>
              <Link href="/docs/getting-started">
                <Button size="xl" variant="outline">
                  Get started
                </Button>
              </Link>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
