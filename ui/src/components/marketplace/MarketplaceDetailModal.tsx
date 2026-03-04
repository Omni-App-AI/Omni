import { useEffect, useRef } from "react";
import {
  X,
  Download,
  Star,
  ExternalLink,
  Shield,
  Wrench,
  Loader2,
  Check,
  ArrowUp,
  FileCode,
  ShieldCheck,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-shell";
import type { MarketplaceDetailDto, ExtensionUpdateDto } from "../../lib/tauri-commands";
import { TrustBadge } from "./TrustBadge";

interface MarketplaceDetailModalProps {
  detail: MarketplaceDetailDto | null;
  loading: boolean;
  isInstalled: boolean;
  isInstalling: boolean;
  updateInfo?: ExtensionUpdateDto;
  onClose: () => void;
  onInstall: (id: string) => void;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  if (bytes >= 1_024) return `${(bytes / 1_024).toFixed(1)} KB`;
  return `${bytes} B`;
}

function formatDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

const SCAN_STATUS_LABELS: Record<string, { label: string; color: string }> = {
  passed: { label: "Passed", color: "#22c55e" },
  clean: { label: "Clean", color: "#22c55e" },
  pending: { label: "Pending", color: "#eab308" },
  scanning: { label: "Scanning", color: "#3b82f6" },
  failed: { label: "Failed", color: "#ef4444" },
  flagged: { label: "Flagged", color: "#f97316" },
};

export function MarketplaceDetailModal({
  detail,
  loading,
  isInstalled,
  isInstalling,
  updateInfo,
  onClose,
  onInstall,
}: MarketplaceDetailModalProps) {
  const backdropRef = useRef<HTMLDivElement>(null);
  const hasUpdate = updateInfo?.has_update ?? false;

  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleEsc);
    return () => window.removeEventListener("keydown", handleEsc);
  }, [onClose]);

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === backdropRef.current) onClose();
  };

  return (
    <div
      ref={backdropRef}
      onClick={handleBackdropClick}
      className="fixed inset-0 z-50 flex items-center justify-center p-8"
      style={{
        backgroundColor: "rgba(0, 0, 0, 0.6)",
        backdropFilter: "blur(4px)",
      }}
    >
      <div
        className="relative rounded-xl w-full max-w-3xl max-h-[85vh] overflow-hidden flex flex-col"
        style={{
          backgroundColor: "var(--bg-primary)",
          border: "1px solid var(--border)",
        }}
      >
        {/* Close button */}
        <button
          onClick={onClose}
          className="absolute top-4 right-4 z-10 p-1.5 rounded-md hover:opacity-80 transition-opacity"
          style={{
            backgroundColor: "var(--bg-secondary)",
            color: "var(--text-muted)",
          }}
        >
          <X size={18} />
        </button>

        {loading || !detail ? (
          <div className="overflow-y-auto flex-1">
            {/* Skeleton header */}
            <div
              className="p-6 border-b flex items-start gap-4"
              style={{ borderColor: "var(--border)" }}
            >
              <div
                className="w-16 h-16 rounded-xl flex-shrink-0 animate-pulse"
                style={{ backgroundColor: "var(--bg-secondary)" }}
              />
              <div className="flex-1 space-y-3">
                <div
                  className="h-6 w-48 rounded animate-pulse"
                  style={{ backgroundColor: "var(--bg-secondary)" }}
                />
                <div
                  className="h-4 w-32 rounded animate-pulse"
                  style={{ backgroundColor: "var(--bg-secondary)" }}
                />
                <div
                  className="h-4 w-full rounded animate-pulse"
                  style={{ backgroundColor: "var(--bg-secondary)" }}
                />
              </div>
            </div>
            <div className="flex flex-col md:flex-row">
              {/* Skeleton main content */}
              <div className="flex-1 p-6 space-y-4">
                {[...Array(4)].map((_, i) => (
                  <div
                    key={i}
                    className="h-4 rounded animate-pulse"
                    style={{
                      backgroundColor: "var(--bg-secondary)",
                      width: `${85 - i * 15}%`,
                    }}
                  />
                ))}
                <div className="pt-4 space-y-2">
                  {[...Array(3)].map((_, i) => (
                    <div
                      key={i}
                      className="h-8 w-20 rounded-md inline-block mr-2 animate-pulse"
                      style={{ backgroundColor: "var(--bg-secondary)" }}
                    />
                  ))}
                </div>
              </div>
              {/* Skeleton sidebar */}
              <div
                className="w-full md:w-64 flex-shrink-0 p-6 border-t md:border-t-0 md:border-l space-y-4"
                style={{ borderColor: "var(--border)" }}
              >
                <div
                  className="h-10 rounded-md animate-pulse"
                  style={{ backgroundColor: "var(--bg-secondary)" }}
                />
                {[...Array(4)].map((_, i) => (
                  <div key={i} className="flex justify-between">
                    <div
                      className="h-4 w-16 rounded animate-pulse"
                      style={{ backgroundColor: "var(--bg-secondary)" }}
                    />
                    <div
                      className="h-4 w-12 rounded animate-pulse"
                      style={{ backgroundColor: "var(--bg-secondary)" }}
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        ) : (
          <div className="overflow-y-auto flex-1">
            {/* Header */}
            <div
              className="p-6 border-b flex items-start gap-4"
              style={{ borderColor: "var(--border)" }}
            >
              {detail.icon_url ? (
                <img
                  src={detail.icon_url}
                  alt=""
                  className="w-16 h-16 rounded-xl object-cover flex-shrink-0"
                />
              ) : (
                <div
                  className="w-16 h-16 rounded-xl flex items-center justify-center text-white font-bold text-xl flex-shrink-0"
                  style={{
                    background: `linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 60%, #8b5cf6))`,
                  }}
                >
                  {detail.name.charAt(0).toUpperCase()}
                </div>
              )}

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  <h2
                    className="text-xl font-bold"
                    style={{ color: "var(--text-primary)" }}
                  >
                    {detail.name}
                  </h2>
                  <span
                    className="text-sm px-2 py-0.5 rounded-md"
                    style={{
                      backgroundColor: "var(--bg-secondary)",
                      color: "var(--text-muted)",
                    }}
                  >
                    v{detail.latest_version}
                  </span>
                  <TrustBadge level={detail.trust_level} />
                </div>

                <p className="text-sm mt-1" style={{ color: "var(--text-secondary)" }}>
                  by {detail.publisher_name}
                  {detail.publisher_verified && (
                    <ShieldCheck
                      size={14}
                      className="inline ml-1"
                      style={{ color: "#3b82f6" }}
                    />
                  )}
                </p>

                <p
                  className="text-sm mt-2 leading-relaxed"
                  style={{ color: "var(--text-secondary)" }}
                >
                  {detail.short_description}
                </p>
              </div>
            </div>

            <div className="flex flex-col md:flex-row">
              {/* Main content */}
              <div className="flex-1 p-6 space-y-6 min-w-0">
                {/* Description */}
                {detail.description && (
                  <div>
                    <h3
                      className="text-sm font-semibold mb-2"
                      style={{ color: "var(--text-primary)" }}
                    >
                      About
                    </h3>
                    <p
                      className="text-sm leading-relaxed whitespace-pre-wrap"
                      style={{ color: "var(--text-secondary)" }}
                    >
                      {detail.description}
                    </p>
                  </div>
                )}

                {/* Tools */}
                {detail.tools.length > 0 && (
                  <div>
                    <h3
                      className="text-sm font-semibold mb-2 flex items-center gap-1.5"
                      style={{ color: "var(--text-primary)" }}
                    >
                      <Wrench size={14} />
                      Tools ({detail.tools.length})
                    </h3>
                    <div className="flex flex-wrap gap-1.5">
                      {detail.tools.map((tool) => (
                        <span
                          key={tool}
                          className="text-xs px-2 py-1 rounded-md font-mono"
                          style={{
                            backgroundColor: "var(--bg-secondary)",
                            color: "var(--text-secondary)",
                            border: "1px solid var(--border)",
                          }}
                        >
                          {tool}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {/* Permissions */}
                {detail.permissions.length > 0 && (
                  <div>
                    <h3
                      className="text-sm font-semibold mb-2 flex items-center gap-1.5"
                      style={{ color: "var(--text-primary)" }}
                    >
                      <Shield size={14} />
                      Permissions ({detail.permissions.length})
                    </h3>
                    <div className="flex flex-wrap gap-1.5">
                      {detail.permissions.map((perm) => (
                        <span
                          key={perm}
                          className="text-xs px-2 py-1 rounded-md"
                          style={{
                            backgroundColor:
                              "color-mix(in srgb, var(--warning) 10%, transparent)",
                            color: "var(--warning)",
                            border:
                              "1px solid color-mix(in srgb, var(--warning) 25%, transparent)",
                          }}
                        >
                          {perm}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {/* Changelog */}
                {detail.changelog && (
                  <div>
                    <h3
                      className="text-sm font-semibold mb-2"
                      style={{ color: "var(--text-primary)" }}
                    >
                      Changelog
                    </h3>
                    <p
                      className="text-sm leading-relaxed whitespace-pre-wrap"
                      style={{ color: "var(--text-secondary)" }}
                    >
                      {detail.changelog}
                    </p>
                  </div>
                )}
              </div>

              {/* Sidebar */}
              <div
                className="w-full md:w-64 flex-shrink-0 p-6 border-t md:border-t-0 md:border-l space-y-5"
                style={{ borderColor: "var(--border)" }}
              >
                {/* Install button */}
                <div>
                  {isInstalling ? (
                    <button
                      disabled
                      className="w-full py-2 rounded-md text-sm font-medium flex items-center justify-center gap-2 opacity-70"
                      style={{
                        backgroundColor: "var(--accent)",
                        color: "#fff",
                      }}
                    >
                      <Loader2 size={16} className="animate-spin" />
                      Installing...
                    </button>
                  ) : isInstalled && !hasUpdate ? (
                    <button
                      disabled
                      className="w-full py-2 rounded-md text-sm font-medium flex items-center justify-center gap-2"
                      style={{
                        backgroundColor:
                          "color-mix(in srgb, var(--success) 15%, transparent)",
                        color: "var(--success)",
                        border:
                          "1px solid color-mix(in srgb, var(--success) 30%, transparent)",
                      }}
                    >
                      <Check size={16} />
                      Installed
                    </button>
                  ) : hasUpdate ? (
                    <button
                      onClick={() => onInstall(detail.id)}
                      className="w-full py-2 rounded-md text-sm font-medium flex items-center justify-center gap-2 hover:opacity-90 transition-opacity"
                      style={{
                        backgroundColor:
                          "color-mix(in srgb, var(--warning) 15%, transparent)",
                        color: "var(--warning)",
                        border:
                          "1px solid color-mix(in srgb, var(--warning) 30%, transparent)",
                      }}
                    >
                      <ArrowUp size={16} />
                      Update to v{updateInfo?.latest_version}
                    </button>
                  ) : (
                    <button
                      onClick={() => onInstall(detail.id)}
                      className="w-full py-2 rounded-md text-sm font-medium hover:opacity-90 transition-opacity"
                      style={{
                        backgroundColor: "var(--accent)",
                        color: "#fff",
                      }}
                    >
                      Install Extension
                    </button>
                  )}
                </div>

                {/* Stats */}
                <div className="space-y-3">
                  <div className="flex items-center justify-between text-sm">
                    <span style={{ color: "var(--text-muted)" }}>Downloads</span>
                    <span
                      className="flex items-center gap-1"
                      style={{ color: "var(--text-primary)" }}
                    >
                      <Download size={14} />
                      {formatDownloads(detail.total_downloads)}
                    </span>
                  </div>

                  <div className="flex items-center justify-between text-sm">
                    <span style={{ color: "var(--text-muted)" }}>Rating</span>
                    <span
                      className="flex items-center gap-1"
                      style={{ color: "var(--text-primary)" }}
                    >
                      <Star size={14} fill="#eab308" stroke="#eab308" />
                      {detail.average_rating.toFixed(1)} ({detail.review_count})
                    </span>
                  </div>

                  {detail.scan_status && (
                    <div className="flex items-center justify-between text-sm">
                      <span style={{ color: "var(--text-muted)" }}>Security</span>
                      <span
                        className="flex items-center gap-1"
                        style={{
                          color:
                            SCAN_STATUS_LABELS[detail.scan_status]?.color ??
                            "var(--text-muted)",
                        }}
                      >
                        <ShieldCheck size={14} />
                        {SCAN_STATUS_LABELS[detail.scan_status]?.label ??
                          detail.scan_status}
                      </span>
                    </div>
                  )}

                  {detail.wasm_size_bytes != null && (
                    <div className="flex items-center justify-between text-sm">
                      <span style={{ color: "var(--text-muted)" }}>Size</span>
                      <span
                        className="flex items-center gap-1"
                        style={{ color: "var(--text-primary)" }}
                      >
                        <FileCode size={14} />
                        {formatBytes(detail.wasm_size_bytes)}
                      </span>
                    </div>
                  )}

                  {detail.license && (
                    <div className="flex items-center justify-between text-sm">
                      <span style={{ color: "var(--text-muted)" }}>License</span>
                      <span style={{ color: "var(--text-primary)" }}>
                        {detail.license}
                      </span>
                    </div>
                  )}

                  {detail.min_omni_version && (
                    <div className="flex items-center justify-between text-sm">
                      <span style={{ color: "var(--text-muted)" }}>Min Omni</span>
                      <span style={{ color: "var(--text-primary)" }}>
                        v{detail.min_omni_version}
                      </span>
                    </div>
                  )}
                </div>

                {/* Links */}
                <div className="space-y-2 pt-2 border-t" style={{ borderColor: "var(--border)" }}>
                  <button
                    onClick={(e) => { e.stopPropagation(); open(`https://omniapp.org/extensions/${detail.id}`); }}
                    className="flex items-center gap-2 text-sm hover:opacity-80 transition-opacity cursor-pointer bg-transparent border-none p-0"
                    style={{ color: "var(--accent)" }}
                  >
                    <ExternalLink size={14} />
                    View on Website
                  </button>
                  {detail.homepage && (
                    <button
                      onClick={(e) => { e.stopPropagation(); open(detail.homepage!); }}
                      className="flex items-center gap-2 text-sm hover:opacity-80 transition-opacity cursor-pointer bg-transparent border-none p-0"
                      style={{ color: "var(--accent)" }}
                    >
                      <ExternalLink size={14} />
                      Homepage
                    </button>
                  )}
                  {detail.repository && (
                    <button
                      onClick={(e) => { e.stopPropagation(); open(detail.repository!); }}
                      className="flex items-center gap-2 text-sm hover:opacity-80 transition-opacity cursor-pointer bg-transparent border-none p-0"
                      style={{ color: "var(--accent)" }}
                    >
                      <ExternalLink size={14} />
                      Source Code
                    </button>
                  )}
                </div>

                {/* Categories */}
                {detail.categories.length > 0 && (
                  <div className="pt-2 border-t" style={{ borderColor: "var(--border)" }}>
                    <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                      Categories
                    </span>
                    <div className="flex flex-wrap gap-1 mt-1">
                      {detail.categories.map((cat) => (
                        <span
                          key={cat}
                          className="text-xs px-2 py-0.5 rounded-full"
                          style={{
                            backgroundColor: "var(--bg-secondary)",
                            color: "var(--text-secondary)",
                            border: "1px solid var(--border)",
                          }}
                        >
                          {cat}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
