import { Loader2 } from "lucide-react";
import type { MarketplaceExtensionDto, ExtensionUpdateDto } from "../../lib/tauri-commands";
import { MarketplaceExtensionCard } from "./MarketplaceExtensionCard";

interface MarketplaceExtensionGridProps {
  extensions: MarketplaceExtensionDto[];
  loading: boolean;
  offline?: boolean;
  installedIds: Set<string>;
  installingId: string | null;
  updateMap: Map<string, ExtensionUpdateDto>;
  onViewDetail: (id: string) => void;
  onInstall: (id: string) => void;
}

export function MarketplaceExtensionGrid({
  extensions,
  loading,
  offline,
  installedIds,
  installingId,
  updateMap,
  onViewDetail,
  onInstall,
}: MarketplaceExtensionGridProps) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2
          size={32}
          className="animate-spin"
          style={{ color: "var(--accent)" }}
        />
      </div>
    );
  }

  if (offline) {
    return (
      <div className="flex flex-col items-center justify-center py-16 gap-2">
        <p className="text-sm" style={{ color: "var(--text-muted)" }}>
          Could not connect to the marketplace.
        </p>
        <p className="text-xs" style={{ color: "var(--text-muted)" }}>
          Check your connection and try refreshing.
        </p>
      </div>
    );
  }

  if (extensions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 gap-2">
        <p className="text-sm" style={{ color: "var(--text-muted)" }}>
          No extensions found.
        </p>
        <p className="text-xs" style={{ color: "var(--text-muted)" }}>
          Try adjusting your search or filters.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {extensions.map((ext) => (
        <MarketplaceExtensionCard
          key={ext.id}
          extension={ext}
          isInstalled={installedIds.has(ext.id)}
          isInstalling={installingId === ext.id}
          updateInfo={updateMap.get(ext.id)}
          onViewDetail={onViewDetail}
          onInstall={onInstall}
        />
      ))}
    </div>
  );
}
