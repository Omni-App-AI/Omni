import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import { Download, Star, BarChart3, Clock, ExternalLink } from "lucide-react";
import { createClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { TrustBadge } from "@/components/marketplace/TrustBadge";
import { ScanStatus } from "@/components/dashboard/ScanStatus";
import { VersionUpload } from "@/components/dashboard/VersionUpload";
import { ExtensionSettings } from "@/components/dashboard/ExtensionSettings";
import { formatNumber } from "@/lib/utils";
import type { Extension, ExtensionVersion } from "@/lib/supabase/types";

export const metadata: Metadata = {
  title: "Manage Extension",
  description:
    "Manage extension settings, upload new versions, and view scan results.",
};

export default async function ManageExtensionPage({
  params,
}: {
  params: Promise<{ extensionId: string }>;
}) {
  const { extensionId } = await params;
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) return null;

  const { data: extensionData } = await supabase
    .from("extensions")
    .select("*")
    .eq("id", extensionId)
    .eq("publisher_id", user.id)
    .single();

  const extension = extensionData as Extension | null;
  if (!extension) notFound();

  const { data: versionsData } = await supabase
    .from("extension_versions")
    .select("*, scan_results(*)")
    .eq("extension_id", extensionId)
    .order("created_at", { ascending: false });

  const versions = versionsData as (ExtensionVersion & { scan_results: unknown[] })[] | null;

  return (
    <div>
      {/* Header */}
      <section className="relative overflow-hidden border-b border-border/50">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative px-8 lg:px-12 pt-10 pb-8">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
                Manage Extension
              </p>
              <div className="flex items-center gap-3">
                <h1 className="text-3xl font-bold tracking-tight">{extension.name}</h1>
                <TrustBadge level={extension.trust_level as "verified" | "community" | "unverified"} showLabel />
              </div>
              <p className="text-sm text-muted-foreground mt-1 font-mono">
                {extension.id}
              </p>
            </div>
            <div className="flex gap-2">
              <Link href={`/dashboard/extensions/${extensionId}/analytics`}>
                <Button variant="outline" size="sm" className="gap-2">
                  <BarChart3 className="h-4 w-4" />
                  Analytics
                </Button>
              </Link>
              <Link href={`/extensions/${extensionId}`}>
                <Button variant="outline" size="sm" className="gap-2">
                  <ExternalLink className="h-4 w-4" />
                  Public Page
                </Button>
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Stats strip */}
      <section className="border-b border-border/50">
        <div className="px-8 lg:px-12 py-6">
          <div className="grid grid-cols-3 gap-8">
            <div>
              <p className="text-2xl font-bold">{formatNumber(extension.total_downloads)}</p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Downloads
              </p>
            </div>
            <div>
              <p className="text-2xl font-bold">
                {extension.average_rating > 0 ? extension.average_rating.toFixed(1) : "—"}
                {extension.review_count > 0 && (
                  <span className="text-sm font-normal text-muted-foreground ml-1">
                    ({extension.review_count})
                  </span>
                )}
              </p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Rating
              </p>
            </div>
            <div>
              <p className="text-2xl font-bold font-mono">v{extension.latest_version || "—"}</p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Latest Version
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Extension Settings */}
      <section className="px-8 lg:px-12 py-8 border-b border-border/50">
        <h2 className="text-lg font-semibold mb-4">Extension Settings</h2>
        <div className="border border-border/50 rounded-lg p-6">
          <ExtensionSettings extension={extension} />
        </div>
      </section>

      {/* Upload + Versions */}
      <section className="px-8 lg:px-12 py-8 space-y-8">
        {/* Upload new version */}
        <div>
          <h2 className="text-lg font-semibold mb-4">Upload New Version</h2>
          <div className="border border-border/50 rounded-lg p-6">
            <VersionUpload extensionId={extensionId} />
          </div>
        </div>

        {/* Version list */}
        <div>
          <h2 className="text-lg font-semibold mb-4">Version History</h2>
          {!versions || versions.length === 0 ? (
            <div className="border border-dashed border-border/50 rounded-lg p-8 text-center">
              <p className="text-sm text-muted-foreground">No versions uploaded yet.</p>
            </div>
          ) : (
            <div className="border border-border/50 rounded-lg divide-y divide-border/50">
              {versions.map((v) => {
                const scanResults = v.scan_results as unknown[];
                const latestScan = scanResults?.[0] as Record<string, unknown> | undefined;

                return (
                  <div
                    key={v.id}
                    className="flex items-start justify-between px-5 py-4"
                  >
                    <div className="space-y-1">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-sm font-medium">v{v.version}</span>
                        {v.scan_status === "passed" && <Badge variant="success">Passed</Badge>}
                        {v.scan_status === "pending" && <Badge variant="secondary">Pending</Badge>}
                        {v.scan_status === "scanning" && <Badge variant="secondary">Scanning</Badge>}
                        {v.scan_status === "failed" && <Badge variant="destructive">Failed</Badge>}
                        {v.scan_status === "flagged" && <Badge variant="warning">Flagged</Badge>}
                        {v.published && <Badge variant="outline">Published</Badge>}
                      </div>
                      {v.changelog && (
                        <p className="text-[13px] text-muted-foreground">{v.changelog}</p>
                      )}
                    </div>
                    <div className="text-right shrink-0 ml-4">
                      <p className="text-xs text-muted-foreground font-mono">
                        {new Date(v.created_at).toLocaleDateString()}
                      </p>
                      {latestScan && (
                        <div className="mt-1">
                          <ScanStatus scanResult={latestScan} compact />
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
