import type { Metadata } from "next";
import Link from "next/link";
import { Plus, Package, Download, Star, ExternalLink, ArrowRight } from "lucide-react";
import { createClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { TrustBadge } from "@/components/marketplace/TrustBadge";
import { formatNumber } from "@/lib/utils";
import type { Extension } from "@/lib/supabase/types";

export const metadata: Metadata = {
  title: "My Extensions",
  description: "Manage your published Omni extensions.",
};

export default async function MyExtensionsPage() {
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) return null;

  const { data: extensions } = await supabase
    .from("extensions")
    .select("*")
    .eq("publisher_id", user.id)
    .order("updated_at", { ascending: false }) as { data: Extension[] | null };


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
                Extensions
              </p>
              <h1 className="text-3xl font-bold tracking-tight">
                My Extensions
              </h1>
              <p className="text-muted-foreground mt-1">
                Manage your published extensions.
              </p>
            </div>
            <Link href="/dashboard/extensions/new">
              <Button className="gap-2">
                <Plus className="h-4 w-4" />
                New Extension
              </Button>
            </Link>
          </div>
        </div>
      </section>

      {/* Content */}
      <section className="px-8 lg:px-12 py-8">
        {!extensions || extensions.length === 0 ? (
          <div className="border border-dashed border-border/50 rounded-lg p-12 text-center">
            <Package className="h-10 w-10 text-muted-foreground/40 mx-auto mb-4" />
            <p className="text-lg font-semibold mb-1">No extensions yet</p>
            <p className="text-sm text-muted-foreground mb-6 max-w-md mx-auto">
              Build your first extension with the Omni SDK and publish it to the marketplace.
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
          <div className="space-y-3">
            {extensions.map((ext) => (
              <div
                key={ext.id}
                className="border border-border/50 rounded-lg p-5 hover:border-primary/20 transition-colors"
              >
                <div className="flex items-center gap-4">
                  <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg bg-primary/10">
                    {ext.icon_url ? (
                      <img src={ext.icon_url} alt={ext.name} className="h-7 w-7 rounded-lg" />
                    ) : (
                      <Package className="h-6 w-6 text-primary" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-0.5">
                      <h3 className="text-sm font-semibold">{ext.name}</h3>
                      <TrustBadge level={ext.trust_level as "verified" | "community" | "unverified"} />
                      {!ext.published && <Badge variant="outline">Draft</Badge>}
                    </div>
                    <p className="text-[13px] text-muted-foreground truncate">{ext.short_description}</p>
                    <div className="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
                      <span className="flex items-center gap-1.5">
                        <Download className="h-3 w-3" />
                        {formatNumber(ext.total_downloads)}
                      </span>
                      {ext.average_rating > 0 && (
                        <span className="flex items-center gap-1.5">
                          <Star className="h-3 w-3 fill-warning text-warning" />
                          {ext.average_rating.toFixed(1)} ({ext.review_count})
                        </span>
                      )}
                      {ext.latest_version && (
                        <span className="font-mono">v{ext.latest_version}</span>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Link href={`/extensions/${ext.id}`}>
                      <Button variant="ghost" size="icon" className="text-muted-foreground hover:text-foreground">
                        <ExternalLink className="h-4 w-4" />
                      </Button>
                    </Link>
                    <Link href={`/dashboard/extensions/${ext.id}`}>
                      <Button variant="outline" size="sm" className="gap-1.5">
                        Manage
                        <ArrowRight className="h-3 w-3" />
                      </Button>
                    </Link>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
