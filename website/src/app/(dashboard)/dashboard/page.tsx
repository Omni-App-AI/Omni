import type { Metadata } from "next";
import Link from "next/link";
import { Package, Download, Star, Plus, ArrowRight } from "lucide-react";
import { createClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { formatNumber } from "@/lib/utils";
import type { Extension } from "@/lib/supabase/types";

export const metadata: Metadata = {
  title: "Dashboard",
  description:
    "Your extension publisher dashboard. View stats, manage extensions, and track downloads.",
};

export default async function DashboardPage() {
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) return null;

  const { data } = await supabase
    .from("extensions")
    .select("id, name, total_downloads, average_rating, review_count, latest_version, trust_level, icon_url")
    .eq("publisher_id", user.id)
    .order("total_downloads", { ascending: false });

  const extensions = data as Pick<Extension, "id" | "name" | "total_downloads" | "average_rating" | "review_count" | "latest_version" | "trust_level" | "icon_url">[] | null;

  const totalDownloads = extensions?.reduce((sum, ext) => sum + ext.total_downloads, 0) || 0;
  const avgRating = extensions?.length
    ? extensions.reduce((sum, ext) => sum + ext.average_rating, 0) / extensions.length
    : 0;

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
                Overview
              </p>
              <h1 className="text-3xl font-bold tracking-tight">
                Welcome back
              </h1>
              <p className="text-muted-foreground mt-1">
                Manage your extensions and track performance.
              </p>
            </div>
            <Link href="/dashboard/extensions/new">
              <Button className="gap-2">
                <Plus className="h-4 w-4" />
                Publish Extension
              </Button>
            </Link>
          </div>
        </div>
      </section>

      {/* Stats strip */}
      <section className="border-b border-border/50">
        <div className="px-8 lg:px-12 py-6">
          <div className="grid grid-cols-3 gap-8">
            <div>
              <p className="text-2xl font-bold">{extensions?.length || 0}</p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Extensions
              </p>
            </div>
            <div>
              <p className="text-2xl font-bold">{formatNumber(totalDownloads)}</p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Total Downloads
              </p>
            </div>
            <div>
              <p className="text-2xl font-bold">{avgRating > 0 ? avgRating.toFixed(1) : "—"}</p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Avg. Rating
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Extensions list */}
      <section className="px-8 lg:px-12 py-8">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-lg font-semibold">Your Extensions</h2>
          {extensions && extensions.length > 0 && (
            <Link href="/dashboard/extensions" className="text-[13px] text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1">
              View all <ArrowRight className="h-3 w-3" />
            </Link>
          )}
        </div>

        {!extensions || extensions.length === 0 ? (
          <div className="border border-dashed border-border/50 rounded-lg p-12 text-center">
            <Package className="h-10 w-10 text-muted-foreground/40 mx-auto mb-4" />
            <p className="text-sm text-muted-foreground mb-1">No extensions published yet.</p>
            <p className="text-xs text-muted-foreground/60 mb-6">
              Build with the Omni SDK and publish your first extension.
            </p>
            <div className="flex justify-center gap-3">
              <Link href="/dashboard/extensions/new">
                <Button size="sm">Publish Extension</Button>
              </Link>
              <Link href="/docs/sdk">
                <Button variant="outline" size="sm">Read SDK Docs</Button>
              </Link>
            </div>
          </div>
        ) : (
          <div className="border border-border/50 rounded-lg divide-y divide-border/50">
            {extensions.map((ext) => (
              <Link
                key={ext.id}
                href={`/dashboard/extensions/${ext.id}`}
                className="flex items-center gap-4 px-5 py-4 hover:bg-secondary/30 transition-colors first:rounded-t-lg last:rounded-b-lg"
              >
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10">
                  {ext.icon_url ? (
                    <img src={ext.icon_url} alt={ext.name} className="h-6 w-6 rounded" />
                  ) : (
                    <Package className="h-5 w-5 text-primary" />
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{ext.name}</p>
                  <p className="text-xs text-muted-foreground font-mono">
                    v{ext.latest_version || "0.0.0"}
                  </p>
                </div>
                <div className="flex items-center gap-5 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1.5">
                    <Download className="h-3.5 w-3.5" />
                    {formatNumber(ext.total_downloads)}
                  </span>
                  {ext.average_rating > 0 && (
                    <span className="flex items-center gap-1.5">
                      <Star className="h-3.5 w-3.5 fill-warning text-warning" />
                      {ext.average_rating.toFixed(1)}
                    </span>
                  )}
                  <ArrowRight className="h-3.5 w-3.5 text-muted-foreground/40" />
                </div>
              </Link>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
