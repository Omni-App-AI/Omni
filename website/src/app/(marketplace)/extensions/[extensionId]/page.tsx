import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import {
  Download,
  Star,
  ExternalLink,
  GitBranch,
  Calendar,
  Package,
  Shield,
  Clock,
  ChevronRight,
  MessageSquare,
  Tag,
  Wrench,
} from "lucide-react";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Avatar } from "@/components/ui/avatar";
import { TrustBadge } from "@/components/marketplace/TrustBadge";
import { PermissionBadges } from "@/components/marketplace/PermissionBadges";
import { ReviewSection } from "@/components/marketplace/ReviewSection";
import { ExtensionModActions, ExtensionModerationBanner } from "@/components/marketplace/ExtensionModActions";
import { InstallButton } from "@/components/marketplace/InstallButton";
import { ScanStatus } from "@/components/dashboard/ScanStatus";
import { formatNumber, formatBytes } from "@/lib/utils";
import { JsonLd } from "@/components/seo/JsonLd";
import type { Extension, ExtensionVersion, Review, Profile } from "@/lib/supabase/types";

interface Props {
  params: Promise<{ extensionId: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { extensionId } = await params;
  const supabase = await createClient();

  const { data: extData } = await supabase
    .from("extensions")
    .select("name, short_description, icon_url, banner_url")
    .eq("id", extensionId)
    .single();

  const ext = extData as { name: string; short_description: string; icon_url: string | null; banner_url: string | null } | null;
  if (!ext) return { title: "Extension Not Found" };

  const ogImage = ext.banner_url || ext.icon_url || "/og-image.png";

  return {
    title: ext.name,
    description: ext.short_description,
    openGraph: {
      title: `${ext.name} — Omni Extension`,
      description: ext.short_description,
      images: [ogImage],
      url: `/extensions/${extensionId}`,
    },
    alternates: {
      canonical: `/extensions/${extensionId}`,
    },
  };
}

export default async function ExtensionDetailPage({ params }: Props) {
  const { extensionId } = await params;
  const supabase = await createClient();

  const { data: extensionData } = await supabase
    .from("extensions")
    .select("*, publisher:profiles(*)")
    .eq("id", extensionId)
    .single();

  const extension = extensionData as (Extension & { publisher: Profile }) | null;
  if (!extension) notFound();

  const { data: latestVersionData } = await supabase
    .from("extension_versions")
    .select("*, scan_results(*)")
    .eq("extension_id", extensionId)
    .eq("published", true)
    .order("created_at", { ascending: false })
    .limit(1)
    .single();

  const latestVersion = latestVersionData as (ExtensionVersion & { scan_results: unknown[] }) | null;

  const { data: versionsData } = await supabase
    .from("extension_versions")
    .select("id, version, created_at, scan_status, scan_score, changelog")
    .eq("extension_id", extensionId)
    .eq("published", true)
    .order("created_at", { ascending: false })
    .limit(10);

  const versions = versionsData as Pick<ExtensionVersion, "id" | "version" | "created_at" | "scan_status" | "scan_score" | "changelog">[] | null;

  const { data: reviewsData } = await supabase
    .from("reviews")
    .select("*, user:profiles(username, display_name, avatar_url)")
    .eq("extension_id", extensionId)
    .order("created_at", { ascending: false })
    .limit(10);

  const reviews = reviewsData as (Review & { user: Pick<Profile, "username" | "avatar_url"> & { display_name: string } })[] | null;

  // Check if current user is a moderator
  let isModerator = false;
  try {
    const { data: { user } } = await supabase.auth.getUser();
    if (user) {
      const service = createServiceClient();
      const { data: profile } = await service
        .from("profiles")
        .select("is_moderator")
        .eq("id", user.id)
        .single();
      isModerator = !!(profile as any)?.is_moderator;
    }
  } catch {
    // Not logged in or error -- default to false
  }

  const moderationStatus = (extension as any).moderation_status || "active";
  const moderationNote = (extension as any).moderation_note || null;
  const moderatedAt = (extension as any).moderated_at || null;
  const bannerUrl = (extension as any).banner_url as string | null;
  const screenshots = ((extension as any).screenshots as string[]) || [];

  const publisher = extension.publisher as {
    username: string;
    display_name: string;
    avatar_url: string | null;
    verified_publisher: boolean;
  };

  const permissions = (latestVersion?.permissions as Array<{
    capability: string;
    scope?: Record<string, unknown>;
    reason?: string;
  }>) || [];

  const tools = (latestVersion?.tools as Array<{
    name: string;
    description: string;
  }>) || [];

  const baseUrl = process.env.NEXT_PUBLIC_APP_URL || "https://www.omniapp.org";

  return (
    <div>
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "SoftwareApplication",
          name: extension.name,
          description: extension.short_description,
          applicationCategory: "DeveloperApplication",
          operatingSystem: "Cross-platform (WASM)",
          ...(extension.icon_url ? { image: extension.icon_url } : {}),
          author: {
            "@type": "Person",
            name: publisher.display_name,
            url: `${baseUrl}/publishers/${publisher.username}`,
          },
          offers: {
            "@type": "Offer",
            price: "0",
            priceCurrency: "USD",
          },
          ...(extension.average_rating > 0
            ? {
                aggregateRating: {
                  "@type": "AggregateRating",
                  ratingValue: extension.average_rating,
                  reviewCount: extension.review_count,
                  bestRating: 5,
                  worstRating: 1,
                },
              }
            : {}),
        }}
      />
      <JsonLd
        data={{
          "@context": "https://schema.org",
          "@type": "BreadcrumbList",
          itemListElement: [
            {
              "@type": "ListItem",
              position: 1,
              name: "Extensions",
              item: `${baseUrl}/extensions`,
            },
            {
              "@type": "ListItem",
              position: 2,
              name: extension.name,
            },
          ],
        }}
      />

      {/* Hero header with gradient background */}
      <div className="relative border-b border-border/40">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid opacity-30" />

        <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 pt-8 pb-10">
          {/* Breadcrumb */}
          <nav className="flex items-center gap-1.5 text-sm text-muted-foreground mb-8">
            <Link href="/extensions" className="hover:text-foreground transition-colors">Extensions</Link>
            {extension.categories[0] && (
              <>
                <ChevronRight className="h-3 w-3" />
                <Link
                  href={`/extensions?category=${extension.categories[0]}`}
                  className="hover:text-foreground transition-colors capitalize"
                >
                  {extension.categories[0]}
                </Link>
              </>
            )}
            <ChevronRight className="h-3 w-3" />
            <span className="text-foreground">{extension.name}</span>
          </nav>

          {/* Moderation banner */}
          {moderationStatus !== "active" && (
            <div className="mb-6">
              <ExtensionModerationBanner status={moderationStatus} />
            </div>
          )}

          {/* Banner image */}
          {bannerUrl && (
            <div className="mb-8 rounded-xl overflow-hidden border border-border/30 shadow-[0_0_40px_-12px_rgba(124,107,245,0.1)]">
              <img
                src={bannerUrl}
                alt={`${extension.name} banner`}
                className="w-full h-auto aspect-[3/1] object-cover"
              />
            </div>
          )}

          {/* Extension header */}
          <div className="flex items-start gap-5">
            <div className="relative flex h-[72px] w-[72px] shrink-0 items-center justify-center rounded-2xl bg-card border border-border/60 shadow-[0_0_30px_-6px_rgba(124,107,245,0.2)]">
              {extension.icon_url ? (
                <img src={extension.icon_url} alt={extension.name} className="h-[72px] w-[72px] rounded-2xl object-cover" />
              ) : (
                <Package className="h-9 w-9 text-primary" />
              )}
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-3 flex-wrap">
                <h1 className="text-2xl md:text-3xl font-bold">{extension.name}</h1>
                <TrustBadge level={extension.trust_level} showLabel />
              </div>
              <p className="mt-1.5 text-muted-foreground max-w-2xl">{extension.short_description}</p>
              <div className="mt-3 flex items-center gap-3">
                <Link
                  href={`/publishers/${publisher.username}`}
                  className="flex items-center gap-2 hover:text-foreground transition-colors text-sm text-muted-foreground"
                >
                  <Avatar src={publisher.avatar_url} fallback={publisher.display_name} size="sm" />
                  <span>{publisher.display_name}</span>
                </Link>
              </div>
            </div>
          </div>

          {/* Stats pills */}
          <div className="mt-6 flex flex-wrap gap-3">
            <div className="flex items-center gap-1.5 bg-card/80 border border-border/40 rounded-full px-3.5 py-1.5 text-sm">
              <Download className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="font-medium">{formatNumber(extension.total_downloads)}</span>
              <span className="text-muted-foreground">downloads</span>
            </div>
            {extension.average_rating > 0 && (
              <div className="flex items-center gap-1.5 bg-card/80 border border-border/40 rounded-full px-3.5 py-1.5 text-sm">
                <Star className="h-3.5 w-3.5 fill-warning text-warning" />
                <span className="font-medium">{extension.average_rating.toFixed(1)}</span>
                <span className="text-muted-foreground">({extension.review_count} reviews)</span>
              </div>
            )}
            {latestVersion && (
              <div className="flex items-center gap-1.5 bg-card/80 border border-border/40 rounded-full px-3.5 py-1.5 text-sm">
                <GitBranch className="h-3.5 w-3.5 text-muted-foreground" />
                <span className="font-mono font-medium">v{latestVersion.version}</span>
              </div>
            )}
            {latestVersion && (
              <div className="flex items-center gap-1.5 bg-card/80 border border-border/40 rounded-full px-3.5 py-1.5 text-sm">
                <Calendar className="h-3.5 w-3.5 text-muted-foreground" />
                <span className="text-muted-foreground">
                  {new Date(latestVersion.created_at).toLocaleDateString()}
                </span>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Main content area */}
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-8">
        <div className="grid lg:grid-cols-3 gap-8">
          {/* Main content column */}
          <div className="lg:col-span-2 space-y-8">
            {/* About */}
            <section>
              <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
                About
              </h2>
              <div className="rounded-xl border border-border/50 bg-card/50 p-6">
                <div className="prose prose-invert prose-sm max-w-none">
                  <p className="whitespace-pre-wrap text-muted-foreground leading-relaxed">{extension.description}</p>
                </div>
              </div>
            </section>

            {/* Screenshots */}
            {screenshots.length > 0 && (
              <section>
                <h2 className="text-lg font-semibold mb-4">
                  Screenshots
                </h2>
                <div className="flex gap-4 overflow-x-auto pb-4 snap-x snap-mandatory -mx-1 px-1">
                  {screenshots.map((url, i) => (
                    <div
                      key={i}
                      className="shrink-0 snap-start rounded-xl overflow-hidden border border-border/50 shadow-[0_0_20px_-6px_rgba(0,0,0,0.3)]"
                    >
                      <img
                        src={url}
                        alt={`Screenshot ${i + 1}`}
                        className="w-[360px] sm:w-[480px] h-auto aspect-[16/10] object-cover"
                      />
                    </div>
                  ))}
                </div>
              </section>
            )}

            {/* Discussions */}
            <Link
              href={`/extensions/${extensionId}/discussions`}
              className="group flex items-center justify-between rounded-xl border border-border/50 bg-card/50 p-5 hover:border-primary/30 hover:bg-card/80 transition-all duration-200"
            >
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                  <MessageSquare className="h-5 w-5 text-primary" />
                </div>
                <div>
                  <span className="text-sm font-medium group-hover:text-primary transition-colors">Discussions</span>
                  <p className="text-xs text-muted-foreground">Ask questions, report issues, and share feedback</p>
                </div>
              </div>
              <ChevronRight className="h-4 w-4 text-muted-foreground group-hover:text-primary transition-colors" />
            </Link>

            {/* Tools */}
            {tools.length > 0 && (
              <section>
                <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
                  <Wrench className="h-5 w-5 text-muted-foreground" />
                  Tools ({tools.length})
                </h2>
                <div className="rounded-xl border border-border/50 bg-card/50 divide-y divide-border/40">
                  {tools.map((tool) => (
                    <div key={tool.name} className="flex items-start gap-3 p-4">
                      <code className="text-sm bg-primary/10 text-primary px-2.5 py-1 rounded-lg font-mono shrink-0">
                        {tool.name}
                      </code>
                      <span className="text-sm text-muted-foreground pt-0.5">{tool.description}</span>
                    </div>
                  ))}
                </div>
              </section>
            )}

            {/* Security scan */}
            {latestVersion?.scan_results && (latestVersion.scan_results as unknown[]).length > 0 && (
              <section>
                <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
                  <Shield className="h-5 w-5 text-muted-foreground" />
                  Security Scan
                </h2>
                <div className="rounded-xl border border-border/50 bg-card/50 p-6">
                  <ScanStatus
                    scanResult={(latestVersion.scan_results as unknown[])[0] as Record<string, unknown>}
                  />
                </div>
              </section>
            )}

            {/* Version history */}
            {versions && versions.length > 0 && (
              <section>
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-lg font-semibold flex items-center gap-2">
                    <Clock className="h-5 w-5 text-muted-foreground" />
                    Version History
                  </h2>
                  <Link href={`/extensions/${extensionId}/versions`}>
                    <Button variant="ghost" size="sm" className="text-muted-foreground hover:text-foreground">
                      View all
                    </Button>
                  </Link>
                </div>
                <div className="rounded-xl border border-border/50 bg-card/50 divide-y divide-border/40">
                  {versions.map((v) => (
                    <div key={v.id} className="flex items-start justify-between p-4">
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-mono font-medium text-sm">v{v.version}</span>
                          {v.scan_status === "passed" && (
                            <Badge variant="success" className="text-[10px]">Verified</Badge>
                          )}
                        </div>
                        {v.changelog && (
                          <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                            {v.changelog}
                          </p>
                        )}
                      </div>
                      <span className="text-xs text-muted-foreground shrink-0 pt-0.5">
                        {new Date(v.created_at).toLocaleDateString()}
                      </span>
                    </div>
                  ))}
                </div>
              </section>
            )}

            {/* Reviews */}
            <ReviewSection
              extensionId={extensionId}
              reviews={reviews || []}
              averageRating={extension.average_rating}
              reviewCount={extension.review_count}
            />
          </div>

          {/* Sidebar */}
          <div className="space-y-5">
            <div className="lg:sticky lg:top-20 space-y-5">
              {/* Moderator actions */}
              {isModerator && (
                <ExtensionModActions
                  extensionId={extensionId}
                  moderationStatus={moderationStatus}
                  moderationNote={moderationNote}
                  moderatedAt={moderatedAt}
                />
              )}

              {/* Install card */}
              <div className="rounded-xl border border-border/50 bg-card p-5 space-y-4 shadow-[0_0_30px_-8px_rgba(124,107,245,0.08)]">
                <InstallButton
                  extensionId={extensionId}
                  hasPassedScan={latestVersion?.scan_status === "passed"}
                />
                <div className="border-t border-border/40 pt-3 text-xs text-muted-foreground space-y-1.5">
                  <div className="flex items-center justify-between">
                    <span>ID</span>
                    <code className="bg-secondary px-1.5 py-0.5 rounded text-[11px] font-mono">{extension.id}</code>
                  </div>
                  {latestVersion && (
                    <div className="flex items-center justify-between">
                      <span>Size</span>
                      <span>{formatBytes(latestVersion.wasm_size_bytes)}</span>
                    </div>
                  )}
                  {extension.license && (
                    <div className="flex items-center justify-between">
                      <span>License</span>
                      <span>{extension.license}</span>
                    </div>
                  )}
                </div>
              </div>

              {/* Permissions */}
              <div className="rounded-xl border border-border/50 bg-card p-5">
                <h3 className="text-sm font-semibold flex items-center gap-2 mb-3">
                  <Shield className="h-4 w-4 text-muted-foreground" />
                  Permissions ({permissions.length})
                </h3>
                <PermissionBadges permissions={permissions} showReasons />
              </div>

              {/* Links */}
              {(extension.homepage || extension.repository) && (
                <div className="rounded-xl border border-border/50 bg-card p-5">
                  <h3 className="text-sm font-semibold mb-3">Links</h3>
                  <div className="space-y-2">
                    {extension.homepage && (
                      <a
                        href={extension.homepage}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="flex items-center gap-2 text-sm text-muted-foreground hover:text-primary transition-colors"
                      >
                        <ExternalLink className="h-4 w-4" />
                        Homepage
                      </a>
                    )}
                    {extension.repository && (
                      <a
                        href={extension.repository}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="flex items-center gap-2 text-sm text-muted-foreground hover:text-primary transition-colors"
                      >
                        <GitBranch className="h-4 w-4" />
                        Source Code
                      </a>
                    )}
                  </div>
                </div>
              )}

              {/* Tags */}
              {extension.tags.length > 0 && (
                <div className="rounded-xl border border-border/50 bg-card p-5">
                  <h3 className="text-sm font-semibold flex items-center gap-2 mb-3">
                    <Tag className="h-4 w-4 text-muted-foreground" />
                    Tags
                  </h3>
                  <div className="flex flex-wrap gap-1.5">
                    {extension.tags.map((tag) => (
                      <Badge key={tag} variant="secondary" className="text-xs">{tag}</Badge>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
