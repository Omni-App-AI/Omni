import { useEffect, useCallback, useRef } from "react";
import { Store, RefreshCw } from "lucide-react";
import { useMarketplaceStore } from "../../stores/marketplaceStore";
import { useExtensionStore } from "../../stores/extensionStore";
import { MarketplaceSearchBar } from "./MarketplaceSearchBar";
import { MarketplaceCategoryBar } from "./MarketplaceCategoryBar";
import { MarketplaceExtensionGrid } from "./MarketplaceExtensionGrid";
import { MarketplaceDetailModal } from "./MarketplaceDetailModal";
import { MarketplacePagination } from "./MarketplacePagination";

export function MarketplacePage() {
  const {
    extensions,
    categories,
    selectedDetail,
    loading,
    loadingDetail,
    total,
    page,
    totalPages,
    filters,
    error,
    installingId,
    updates,
    checkingUpdates,
    search,
    setQuery,
    setCategory,
    setSort,
    setTrust,
    loadCategories,
    loadDetail,
    clearDetail,
    installFromMarketplace,
    checkForUpdates,
    goToPage,
    refresh,
  } = useMarketplaceStore();

  const { extensions: installedExtensions, loadExtensions } = useExtensionStore();

  const installedIds = new Set(installedExtensions.map((e) => e.id));
  const updateMap = new Map(updates.map((u) => [u.extension_id, u]));

  useEffect(() => {
    console.log("[MarketplacePage] mount — firing loadExtensions, loadCategories, search, checkForUpdates");
    loadExtensions();
    loadCategories();
    search(1);
    checkForUpdates();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Single click = check updates + cached refresh, double click = hard refresh (bypass cache)
  const clickTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isRefreshing = loading || checkingUpdates;

  const handleRefreshClick = useCallback(() => {
    if (isRefreshing) return;
    if (clickTimer.current) {
      // Double click detected -- hard refresh (bypass cache)
      clearTimeout(clickTimer.current);
      clickTimer.current = null;
      refresh();
    } else {
      // Wait to see if a second click comes
      clickTimer.current = setTimeout(() => {
        clickTimer.current = null;
        // Single click -- check updates (uses cache for listings)
        checkForUpdates();
        search();
      }, 300);
    }
  }, [isRefreshing, refresh, checkForUpdates, search]);

  const handleInstall = async (extensionId: string) => {
    try {
      await installFromMarketplace(extensionId);
      await loadExtensions();
    } catch {
      // Error handled by store
    }
  };

  return (
    <div
      className="flex flex-col gap-5 p-6 flex-1 overflow-y-auto"
      style={{
        backgroundColor: "var(--bg-primary)",
        color: "var(--text-primary)",
      }}
    >
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Store size={24} style={{ color: "var(--accent)" }} />
          <div>
            <h2 className="text-xl font-bold">Marketplace</h2>
            <p className="text-sm" style={{ color: "var(--text-muted)" }}>
              Discover and install extensions
            </p>
          </div>
        </div>
        <button
          onClick={handleRefreshClick}
          disabled={isRefreshing}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition-opacity hover:opacity-80 disabled:opacity-50"
          style={{
            backgroundColor: "var(--bg-secondary)",
            border: "1px solid var(--border)",
            color: "var(--text-secondary)",
          }}
          title="Click to refresh, double-click to bypass cache"
        >
          <RefreshCw
            size={14}
            className={isRefreshing ? "animate-spin" : ""}
          />
          Refresh
        </button>
      </div>

      {/* Search + Filters */}
      <MarketplaceSearchBar
        query={filters.query}
        sort={filters.sort}
        trust={filters.trust}
        onQueryChange={setQuery}
        onQuerySubmit={() => search(1)}
        onSortChange={setSort}
        onTrustChange={setTrust}
      />

      {/* Category bar */}
      <MarketplaceCategoryBar
        categories={categories}
        selected={filters.category}
        onSelect={setCategory}
      />

      {/* Error — connection failures get a friendly message */}
      {error && (
        <div
          className="rounded-lg px-4 py-3 text-sm"
          style={{
            backgroundColor:
              "color-mix(in srgb, var(--warning, #f59e0b) 15%, transparent)",
            color: "var(--warning, #f59e0b)",
            border: "1px solid color-mix(in srgb, var(--warning, #f59e0b) 30%, transparent)",
          }}
        >
          {error.includes("error sending request") || error.includes("connection")
            ? "Marketplace is currently unavailable. Check your internet connection or try again later."
            : error}
        </div>
      )}

      {/* Result count */}
      {!loading && !error && (
        <p className="text-sm" style={{ color: "var(--text-muted)" }}>
          {total} extension{total !== 1 ? "s" : ""} found
        </p>
      )}

      {/* Extension Grid */}
      <MarketplaceExtensionGrid
        extensions={extensions}
        loading={loading}
        offline={!!error && extensions.length === 0}
        installedIds={installedIds}
        installingId={installingId}
        updateMap={updateMap}
        onViewDetail={loadDetail}
        onInstall={handleInstall}
      />

      {/* Pagination */}
      {totalPages > 1 && (
        <MarketplacePagination
          page={page}
          totalPages={totalPages}
          onPageChange={goToPage}
        />
      )}

      {/* Detail Modal */}
      {(selectedDetail || loadingDetail) && (
        <MarketplaceDetailModal
          detail={selectedDetail}
          loading={loadingDetail}
          isInstalled={
            selectedDetail ? installedIds.has(selectedDetail.id) : false
          }
          isInstalling={selectedDetail?.id === installingId}
          updateInfo={
            selectedDetail ? updateMap.get(selectedDetail.id) : undefined
          }
          onClose={clearDetail}
          onInstall={handleInstall}
        />
      )}
    </div>
  );
}
