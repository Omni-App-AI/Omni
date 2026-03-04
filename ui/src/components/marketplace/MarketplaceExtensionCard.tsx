import { Download, Star, Loader2, Check, ArrowUp } from "lucide-react";
import type { MarketplaceExtensionDto, ExtensionUpdateDto } from "../../lib/tauri-commands";
import { TrustBadge } from "./TrustBadge";

interface MarketplaceExtensionCardProps {
  extension: MarketplaceExtensionDto;
  isInstalled: boolean;
  isInstalling: boolean;
  updateInfo?: ExtensionUpdateDto;
  onViewDetail: (id: string) => void;
  onInstall: (id: string) => void;
}

function formatDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function RatingStars({ rating }: { rating: number }) {
  const full = Math.floor(rating);
  const hasHalf = rating - full >= 0.25 && rating - full < 0.75;
  const roundedUp = rating - full >= 0.75;
  return (
    <span className="inline-flex items-center gap-0.5">
      {[...Array(5)].map((_, i) => {
        const isFull = i < full || (i === full && roundedUp);
        const isHalf = i === full && hasHalf;
        return (
          <span key={i} className="relative inline-block" style={{ width: 12, height: 12 }}>
            {/* Background (empty) star */}
            <Star size={12} fill="none" stroke="var(--text-muted)" />
            {/* Filled overlay — full or clipped to half */}
            {(isFull || isHalf) && (
              <span
                className="absolute inset-0 overflow-hidden"
                style={{ width: isHalf ? "50%" : "100%" }}
              >
                <Star size={12} fill="#eab308" stroke="#eab308" />
              </span>
            )}
          </span>
        );
      })}
      <span className="text-xs ml-1" style={{ color: "var(--text-muted)" }}>
        {rating.toFixed(1)}
      </span>
    </span>
  );
}

export function MarketplaceExtensionCard({
  extension,
  isInstalled,
  isInstalling,
  updateInfo,
  onViewDetail,
  onInstall,
}: MarketplaceExtensionCardProps) {
  const hasUpdate = updateInfo?.has_update ?? false;

  return (
    <div
      className="rounded-lg p-4 flex flex-col gap-3 cursor-pointer transition-all hover:translate-y-[-1px]"
      style={{
        backgroundColor: "var(--bg-secondary)",
        border: "1px solid var(--border)",
      }}
      onClick={() => onViewDetail(extension.id)}
    >
      {/* Header */}
      <div className="flex items-start gap-3">
        {/* Icon */}
        {extension.icon_url ? (
          <img
            src={extension.icon_url}
            alt=""
            className="w-10 h-10 rounded-lg object-cover flex-shrink-0"
          />
        ) : (
          <div
            className="w-10 h-10 rounded-lg flex items-center justify-center text-white font-bold text-sm flex-shrink-0"
            style={{
              background: `linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 60%, #8b5cf6))`,
            }}
          >
            {extension.name.charAt(0).toUpperCase()}
          </div>
        )}

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <h3
              className="font-semibold text-sm truncate"
              style={{ color: "var(--text-primary)" }}
            >
              {extension.name}
            </h3>
            <TrustBadge level={extension.trust_level} size={12} />
          </div>
          <p className="text-xs mt-0.5" style={{ color: "var(--text-muted)" }}>
            {extension.publisher_name}
          </p>
        </div>
      </div>

      {/* Description */}
      <p
        className="text-xs leading-relaxed line-clamp-2"
        style={{ color: "var(--text-secondary)" }}
      >
        {extension.short_description || "No description available."}
      </p>

      {/* Stats */}
      <div className="flex items-center gap-3 text-xs" style={{ color: "var(--text-muted)" }}>
        <span className="inline-flex items-center gap-1">
          <Download size={12} />
          {formatDownloads(extension.total_downloads)}
        </span>
        <RatingStars rating={extension.average_rating} />
        <span className="ml-auto" style={{ color: "var(--text-muted)" }}>
          v{extension.latest_version}
        </span>
      </div>

      {/* Install button */}
      <div className="mt-auto pt-1">
        {isInstalling ? (
          <button
            disabled
            className="w-full py-1.5 rounded-md text-xs font-medium flex items-center justify-center gap-1.5 opacity-70"
            style={{
              backgroundColor: "var(--accent)",
              color: "#fff",
            }}
          >
            <Loader2 size={14} className="animate-spin" />
            Installing...
          </button>
        ) : isInstalled && !hasUpdate ? (
          <button
            disabled
            className="w-full py-1.5 rounded-md text-xs font-medium flex items-center justify-center gap-1.5"
            style={{
              backgroundColor: "color-mix(in srgb, var(--success) 15%, transparent)",
              color: "var(--success)",
              border: "1px solid color-mix(in srgb, var(--success) 30%, transparent)",
            }}
          >
            <Check size={14} />
            Installed
          </button>
        ) : hasUpdate ? (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onInstall(extension.id);
            }}
            className="w-full py-1.5 rounded-md text-xs font-medium flex items-center justify-center gap-1.5 hover:opacity-90 transition-opacity"
            style={{
              backgroundColor: "color-mix(in srgb, var(--warning) 15%, transparent)",
              color: "var(--warning)",
              border: "1px solid color-mix(in srgb, var(--warning) 30%, transparent)",
            }}
          >
            <ArrowUp size={14} />
            Update to v{updateInfo?.latest_version}
          </button>
        ) : (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onInstall(extension.id);
            }}
            className="w-full py-1.5 rounded-md text-xs font-medium hover:opacity-90 transition-opacity"
            style={{
              backgroundColor: "var(--accent)",
              color: "#fff",
            }}
          >
            Install
          </button>
        )}
      </div>
    </div>
  );
}
