import { Search } from "lucide-react";

interface MarketplaceSearchBarProps {
  query: string;
  sort: string;
  trust: string | null;
  onQueryChange: (q: string) => void;
  onQuerySubmit: () => void;
  onSortChange: (sort: string) => void;
  onTrustChange: (trust: string | null) => void;
}

const SORT_OPTIONS = [
  { value: "downloads", label: "Most Popular" },
  { value: "rating", label: "Top Rated" },
  { value: "newest", label: "Newest" },
  { value: "updated", label: "Recently Updated" },
  { value: "name", label: "Name A-Z" },
];

const TRUST_OPTIONS = [
  { value: null, label: "All Trust Levels" },
  { value: "verified", label: "Verified" },
  { value: "community", label: "Community" },
  { value: "unverified", label: "Unverified" },
];

export function MarketplaceSearchBar({
  query,
  sort,
  trust,
  onQueryChange,
  onQuerySubmit,
  onSortChange,
  onTrustChange,
}: MarketplaceSearchBarProps) {
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      onQuerySubmit();
    }
  };

  const selectStyle: React.CSSProperties = {
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border)",
    color: "var(--text-primary)",
    borderRadius: "6px",
    padding: "6px 8px",
    fontSize: "13px",
    outline: "none",
  };

  return (
    <div className="flex items-center gap-3 flex-wrap">
      <div
        className="flex items-center gap-2 flex-1 min-w-[200px] rounded-md px-3 py-2"
        style={{
          backgroundColor: "var(--bg-secondary)",
          border: "1px solid var(--border)",
        }}
      >
        <Search size={16} style={{ color: "var(--text-muted)", flexShrink: 0 }} />
        <input
          type="text"
          placeholder="Search extensions..."
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={handleKeyDown}
          className="bg-transparent border-none outline-none text-sm flex-1"
          style={{ color: "var(--text-primary)" }}
        />
      </div>

      <select
        value={sort}
        onChange={(e) => onSortChange(e.target.value)}
        style={selectStyle}
      >
        {SORT_OPTIONS.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>

      <select
        value={trust ?? ""}
        onChange={(e) => onTrustChange(e.target.value || null)}
        style={selectStyle}
      >
        {TRUST_OPTIONS.map((opt) => (
          <option key={opt.value ?? "all"} value={opt.value ?? ""}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}
