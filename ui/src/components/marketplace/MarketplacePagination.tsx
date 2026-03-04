import { ChevronLeft, ChevronRight } from "lucide-react";

interface MarketplacePaginationProps {
  page: number;
  totalPages: number;
  onPageChange: (page: number) => void;
}

export function MarketplacePagination({
  page,
  totalPages,
  onPageChange,
}: MarketplacePaginationProps) {
  const btnStyle: React.CSSProperties = {
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border)",
    color: "var(--text-secondary)",
    borderRadius: "6px",
    padding: "6px 12px",
    fontSize: "13px",
    cursor: "pointer",
    transition: "opacity 0.15s",
  };

  const disabledStyle: React.CSSProperties = {
    ...btnStyle,
    opacity: 0.4,
    cursor: "not-allowed",
  };

  return (
    <div className="flex items-center justify-center gap-4 pt-2">
      <button
        onClick={() => onPageChange(page - 1)}
        disabled={page <= 1}
        style={page <= 1 ? disabledStyle : btnStyle}
        className="flex items-center gap-1 hover:opacity-80"
      >
        <ChevronLeft size={16} />
        Previous
      </button>

      <span className="text-sm" style={{ color: "var(--text-muted)" }}>
        Page {page} of {totalPages}
      </span>

      <button
        onClick={() => onPageChange(page + 1)}
        disabled={page >= totalPages}
        style={page >= totalPages ? disabledStyle : btnStyle}
        className="flex items-center gap-1 hover:opacity-80"
      >
        Next
        <ChevronRight size={16} />
      </button>
    </div>
  );
}
