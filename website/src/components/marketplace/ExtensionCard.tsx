import Link from "next/link";
import { Download, Star, Package } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { TrustBadge } from "@/components/marketplace/TrustBadge";
import { formatNumber } from "@/lib/utils";
import type { ExtensionWithPublisher } from "@/lib/supabase/types";

interface ExtensionCardProps {
  extension: ExtensionWithPublisher;
}

export function ExtensionCard({ extension }: ExtensionCardProps) {
  return (
    <Link href={`/extensions/${extension.id}`}>
      <div className="group relative h-full rounded-xl border border-border/60 bg-card p-5 transition-all duration-300 hover:border-primary/30 hover:shadow-[0_0_30px_-8px_rgba(124,107,245,0.15)] cursor-pointer shine">
        {/* Icon + Name row */}
        <div className="flex items-start gap-3.5 mb-3">
          <div className="relative flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-primary/10 transition-all duration-300 group-hover:bg-primary/15 group-hover:shadow-[0_0_20px_-4px_rgba(124,107,245,0.3)]">
            {extension.icon_url ? (
              <img
                src={extension.icon_url}
                alt={extension.name}
                className="h-12 w-12 rounded-xl object-cover"
              />
            ) : (
              <Package className="h-6 w-6 text-primary" />
            )}
          </div>
          <div className="flex-1 min-w-0 pt-0.5">
            <div className="flex items-center justify-between gap-2">
              <h3 className="font-semibold text-[15px] truncate group-hover:text-primary transition-colors duration-200">
                {extension.name}
              </h3>
              <TrustBadge level={extension.trust_level} />
            </div>
            <p className="text-xs text-muted-foreground truncate mt-0.5">
              {extension.publisher?.display_name || extension.publisher?.username}
            </p>
          </div>
        </div>

        {/* Description */}
        <p className="text-sm text-muted-foreground/80 leading-relaxed line-clamp-2 mb-4">
          {extension.short_description}
        </p>

        {/* Divider */}
        <div className="border-t border-border/40 pt-3">
          {/* Stats + Version row */}
          <div className="flex items-center gap-4 text-xs text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <Download className="h-3.5 w-3.5" />
              {formatNumber(extension.total_downloads)}
            </span>
            {extension.average_rating > 0 && (
              <span className="flex items-center gap-1.5">
                <Star className="h-3.5 w-3.5 fill-warning text-warning" />
                {extension.average_rating.toFixed(1)}
              </span>
            )}
            {extension.latest_version && (
              <span className="ml-auto font-mono text-[11px] text-muted-foreground/60">
                v{extension.latest_version}
              </span>
            )}
          </div>

          {/* Categories */}
          {extension.categories.length > 0 && (
            <div className="mt-2.5 flex flex-wrap gap-1.5">
              {extension.categories.slice(0, 2).map((cat) => (
                <Badge key={cat} variant="secondary" className="text-[10px] px-1.5 py-0">
                  {cat}
                </Badge>
              ))}
            </div>
          )}
        </div>
      </div>
    </Link>
  );
}
