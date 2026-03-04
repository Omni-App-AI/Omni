import type { Metadata } from "next";
import { Suspense } from "react";
import { createClient } from "@/lib/supabase/server";
import { SearchBar } from "@/components/marketplace/SearchBar";
import { FilterPanel } from "@/components/marketplace/FilterPanel";
import { ExtensionGrid } from "@/components/marketplace/ExtensionGrid";
import { Button } from "@/components/ui/button";
import Link from "next/link";
import { ChevronLeft, ChevronRight, Blocks } from "lucide-react";
import type { ExtensionWithPublisher } from "@/lib/supabase/types";

export const metadata: Metadata = {
  title: "Browse AI Agent Extensions & WASM Tools",
  description:
    "Discover and install verified WASM extensions for the Omni AI agent. Browse sandboxed AI tools, automation bots, developer utilities, and more — all scanned by a 4-layer security pipeline.",
  openGraph: {
    title: "Browse Omni Extensions — AI Agent Tools & WASM Plugins",
    description:
      "Discover and install verified WASM extensions for the Omni AI agent. Browse sandboxed AI tools, automation bots, and developer utilities — all security-scanned.",
    url: "/extensions",
  },
  alternates: { canonical: "/extensions" },
};

const PAGE_SIZE = 18;

export default async function ExtensionsPage({
  searchParams,
}: {
  searchParams: Promise<{
    q?: string;
    category?: string;
    sort?: string;
    trust?: string;
    page?: string;
  }>;
}) {
  const params = await searchParams;
  const page = Math.max(1, parseInt(params.page || "1", 10));
  const offset = (page - 1) * PAGE_SIZE;

  const supabase = await createClient();

  let query = supabase
    .from("extensions")
    .select("*, publisher:profiles(*)", { count: "exact" })
    .eq("published", true);

  // Search
  if (params.q) {
    query = query.textSearch("fts", params.q, { type: "websearch" });
  }

  // Category filter
  if (params.category) {
    query = query.contains("categories", [params.category]);
  }

  // Trust filter
  if (params.trust) {
    query = query.eq("trust_level", params.trust);
  }

  // Sort
  switch (params.sort) {
    case "rating":
      query = query.order("average_rating", { ascending: false });
      break;
    case "newest":
      query = query.order("created_at", { ascending: false });
      break;
    case "updated":
      query = query.order("updated_at", { ascending: false });
      break;
    case "name":
      query = query.order("name", { ascending: true });
      break;
    case "downloads":
    default:
      query = query.order("total_downloads", { ascending: false });
      break;
  }

  query = query.range(offset, offset + PAGE_SIZE - 1);

  const { data: extensions, count } = await query;
  const totalPages = Math.ceil((count || 0) / PAGE_SIZE);

  return (
    <div>
      {/* Hero header with gradient */}
      <div className="relative border-b border-border/40">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid opacity-40" />
        <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-12 md:py-16">
          <div className="flex items-center gap-3 mb-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-primary/10">
              <Blocks className="h-5 w-5 text-primary" />
            </div>
            <h1 className="text-3xl md:text-4xl font-bold">Extensions</h1>
          </div>
          <p className="text-muted-foreground text-lg max-w-2xl">
            Discover {count || 0} extensions to enhance your Omni experience
          </p>

          {/* Search bar integrated into hero */}
          <div className="mt-6 max-w-2xl">
            <Suspense fallback={<div className="h-12 bg-secondary/50 rounded-xl animate-pulse" />}>
              <SearchBar />
            </Suspense>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-8">
        <div className="flex flex-col lg:flex-row gap-8">
          {/* Sidebar filters */}
          <aside className="lg:w-56 shrink-0">
            <div className="lg:sticky lg:top-20">
              <Suspense fallback={null}>
                <FilterPanel />
              </Suspense>
            </div>
          </aside>

          {/* Results */}
          <div className="flex-1">
            <ExtensionGrid
              extensions={(extensions as ExtensionWithPublisher[]) || []}
              emptyMessage="No extensions match your search. Try different keywords or filters."
            />

            {/* Pagination */}
            {totalPages > 1 && (
              <div className="mt-10 flex items-center justify-center gap-2">
                {page > 1 && (
                  <Link
                    href={`/extensions?${new URLSearchParams({ ...params, page: String(page - 1) }).toString()}`}
                  >
                    <Button variant="outline" size="sm" className="gap-1">
                      <ChevronLeft className="h-4 w-4" /> Previous
                    </Button>
                  </Link>
                )}
                <span className="text-sm text-muted-foreground px-4">
                  Page {page} of {totalPages}
                </span>
                {page < totalPages && (
                  <Link
                    href={`/extensions?${new URLSearchParams({ ...params, page: String(page + 1) }).toString()}`}
                  >
                    <Button variant="outline" size="sm" className="gap-1">
                      Next <ChevronRight className="h-4 w-4" />
                    </Button>
                  </Link>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
