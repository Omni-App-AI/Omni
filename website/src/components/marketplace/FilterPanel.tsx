"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { Select } from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { CATEGORIES, SORT_OPTIONS } from "@/lib/constants";
import { cn } from "@/lib/utils";

export function FilterPanel() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const currentCategory = searchParams.get("category") || "";
  const currentSort = searchParams.get("sort") || "downloads";
  const currentTrust = searchParams.get("trust") || "";

  const updateParam = (key: string, value: string) => {
    const params = new URLSearchParams(searchParams.toString());
    if (value) {
      params.set(key, value);
    } else {
      params.delete(key);
    }
    params.delete("page");
    router.push(`/extensions?${params.toString()}`);
  };

  return (
    <div className="space-y-6">
      {/* Sort */}
      <div>
        <label className="text-sm font-medium mb-2 block">Sort by</label>
        <Select
          value={currentSort}
          onChange={(e) => updateParam("sort", e.target.value)}
        >
          {SORT_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </Select>
      </div>

      {/* Trust level */}
      <div>
        <label className="text-sm font-medium mb-2 block">Trust level</label>
        <div className="flex flex-wrap gap-2">
          {["", "verified", "community", "unverified"].map((level) => (
            <Badge
              key={level}
              variant={currentTrust === level ? "default" : "outline"}
              className="cursor-pointer"
              onClick={() => updateParam("trust", level)}
            >
              {level || "All"}
            </Badge>
          ))}
        </div>
      </div>

      {/* Categories */}
      <div>
        <label className="text-sm font-medium mb-2 block">Category</label>
        <div className="space-y-1">
          <button
            onClick={() => updateParam("category", "")}
            className={cn(
              "w-full text-left px-3 py-1.5 rounded-md text-sm transition-colors",
              !currentCategory
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground hover:bg-secondary",
            )}
          >
            All Categories
          </button>
          {CATEGORIES.map((cat) => (
            <button
              key={cat.id}
              onClick={() => updateParam("category", cat.id)}
              className={cn(
                "w-full text-left px-3 py-1.5 rounded-md text-sm transition-colors",
                currentCategory === cat.id
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:text-foreground hover:bg-secondary",
              )}
            >
              {cat.name}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
