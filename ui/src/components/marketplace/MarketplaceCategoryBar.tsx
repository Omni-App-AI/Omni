import type { MarketplaceCategoryDto } from "../../lib/tauri-commands";

interface MarketplaceCategoryBarProps {
  categories: MarketplaceCategoryDto[];
  selected: string | null;
  onSelect: (category: string | null) => void;
}

export function MarketplaceCategoryBar({
  categories,
  selected,
  onSelect,
}: MarketplaceCategoryBarProps) {
  const pillBase: React.CSSProperties = {
    padding: "4px 12px",
    borderRadius: "9999px",
    fontSize: "13px",
    fontWeight: 500,
    whiteSpace: "nowrap",
    cursor: "pointer",
    transition: "all 0.15s ease",
    border: "1px solid var(--border)",
  };

  const activePill: React.CSSProperties = {
    ...pillBase,
    backgroundColor: "var(--accent)",
    color: "#fff",
    borderColor: "var(--accent)",
  };

  const inactivePill: React.CSSProperties = {
    ...pillBase,
    backgroundColor: "var(--bg-secondary)",
    color: "var(--text-secondary)",
  };

  return (
    <div
      className="flex items-center gap-2 overflow-x-auto pb-1"
      style={{ scrollbarWidth: "thin" }}
    >
      <button
        onClick={() => onSelect(null)}
        style={selected === null ? activePill : inactivePill}
      >
        All
      </button>
      {categories.map((cat) => (
        <button
          key={cat.id}
          onClick={() => onSelect(cat.id)}
          style={selected === cat.id ? activePill : inactivePill}
        >
          {cat.name}
          <span
            className="ml-1.5 text-xs opacity-70"
          >
            {cat.count}
          </span>
        </button>
      ))}
    </div>
  );
}
