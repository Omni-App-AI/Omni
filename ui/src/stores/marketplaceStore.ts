import { create } from "zustand";
import {
  marketplaceSearch,
  marketplaceGetDetail,
  marketplaceGetCategories,
  marketplaceInstall,
  marketplaceCheckUpdates,
  type MarketplaceExtensionDto,
  type MarketplaceDetailDto,
  type MarketplaceCategoryDto,
  type ExtensionUpdateDto,
} from "../lib/tauri-commands";

interface MarketplaceFilters {
  query: string;
  category: string | null;
  sort: string;
  trust: string | null;
}

interface MarketplaceState {
  extensions: MarketplaceExtensionDto[];
  categories: MarketplaceCategoryDto[];
  selectedDetail: MarketplaceDetailDto | null;
  updates: ExtensionUpdateDto[];

  total: number;
  page: number;
  limit: number;
  totalPages: number;

  filters: MarketplaceFilters;

  loading: boolean;
  loadingDetail: boolean;
  loadingCategories: boolean;
  installingId: string | null;
  checkingUpdates: boolean;
  error: string | null;

  search: (page?: number, forceRefresh?: boolean) => Promise<void>;
  setQuery: (query: string) => void;
  setCategory: (category: string | null) => void;
  setSort: (sort: string) => void;
  setTrust: (trust: string | null) => void;
  loadCategories: (forceRefresh?: boolean) => Promise<void>;
  loadDetail: (extensionId: string, forceRefresh?: boolean) => Promise<void>;
  clearDetail: () => void;
  installFromMarketplace: (extensionId: string) => Promise<string>;
  checkForUpdates: () => Promise<void>;
  goToPage: (page: number) => Promise<void>;
  /** Force-refresh all marketplace data (bypasses 30-min cache). */
  refresh: () => Promise<void>;
}

export const useMarketplaceStore = create<MarketplaceState>((set, get) => ({
  extensions: [],
  categories: [],
  selectedDetail: null,
  updates: [],
  total: 0,
  page: 1,
  limit: 24,
  totalPages: 0,
  filters: {
    query: "",
    category: null,
    sort: "downloads",
    trust: null,
  },
  loading: false,
  loadingDetail: false,
  loadingCategories: false,
  installingId: null,
  checkingUpdates: false,
  error: null,

  search: async (page?: number, forceRefresh?: boolean) => {
    const { filters, limit } = get();
    const targetPage = page ?? 1;
    console.log("[marketplace] search START", { filters, targetPage, limit, forceRefresh });
    set({ loading: true, error: null, page: targetPage });
    try {
      const result = await marketplaceSearch(
        filters.query || undefined,
        filters.category ?? undefined,
        filters.sort,
        filters.trust ?? undefined,
        targetPage,
        limit,
        forceRefresh,
      );
      console.log("[marketplace] search OK", result.extensions.length, "results, total:", result.total);
      set({
        extensions: result.extensions,
        total: result.total,
        page: result.page,
        totalPages: result.total_pages,
        loading: false,
      });
    } catch (e) {
      console.error("[marketplace] search FAILED", e);
      set({ loading: false, error: String(e) });
    }
  },

  setQuery: (query: string) => {
    set((s) => ({ filters: { ...s.filters, query } }));
  },

  setCategory: (category: string | null) => {
    set((s) => ({ filters: { ...s.filters, category } }));
    get().search(1);
  },

  setSort: (sort: string) => {
    set((s) => ({ filters: { ...s.filters, sort } }));
    get().search(1);
  },

  setTrust: (trust: string | null) => {
    set((s) => ({ filters: { ...s.filters, trust } }));
    get().search(1);
  },

  loadCategories: async (forceRefresh?: boolean) => {
    console.log("[marketplace] loadCategories START");
    set({ loadingCategories: true });
    try {
      const categories = await marketplaceGetCategories(forceRefresh);
      console.log("[marketplace] loadCategories OK", categories.length, "categories");
      set({ categories, loadingCategories: false });
    } catch (e) {
      console.error("[marketplace] loadCategories FAILED", e);
      set({ loadingCategories: false });
    }
  },

  loadDetail: async (extensionId: string, forceRefresh?: boolean) => {
    set({ loadingDetail: true, selectedDetail: null });
    try {
      const detail = await marketplaceGetDetail(extensionId, forceRefresh);
      set({ selectedDetail: detail, loadingDetail: false });
    } catch (e) {
      set({ loadingDetail: false, error: String(e) });
    }
  },

  clearDetail: () => set({ selectedDetail: null }),

  installFromMarketplace: async (extensionId: string) => {
    set({ installingId: extensionId });
    try {
      const id = await marketplaceInstall(extensionId);
      set({ installingId: null });
      return id;
    } catch (e) {
      set({ installingId: null });
      throw e;
    }
  },

  checkForUpdates: async () => {
    console.log("[marketplace] checkForUpdates START");
    set({ checkingUpdates: true });
    try {
      const updates = await marketplaceCheckUpdates();
      console.log("[marketplace] checkForUpdates OK", updates.length, "updates");
      set({ updates, checkingUpdates: false });
    } catch (e) {
      console.error("[marketplace] checkForUpdates FAILED", e);
      set({ checkingUpdates: false });
    }
  },

  goToPage: async (page: number) => {
    get().search(page);
  },

  refresh: async () => {
    const { page } = get();
    await Promise.all([
      get().search(page, true),
      get().loadCategories(true),
      get().checkForUpdates(),
    ]);
  },
}));
