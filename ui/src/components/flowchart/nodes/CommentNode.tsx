import type { NodeProps } from "@xyflow/react";
import { StickyNote } from "lucide-react";

export function CommentNode(props: NodeProps) {
  const { data, selected } = props;
  const label = (data as Record<string, unknown>).label as string;
  const config = (data as Record<string, unknown>).config as Record<string, unknown> | undefined;
  const text = (config?.text as string) || "";

  return (
    <div
      className={`rounded-lg border-2 border-dashed min-w-[140px] max-w-[220px] ${
        selected ? "ring-2 ring-[var(--accent)]" : ""
      }`}
      style={{ borderColor: "#eab308", background: "color-mix(in srgb, #eab308 10%, var(--bg-primary))" }}
    >
      <div className="flex items-center gap-1.5 px-2 py-1.5 border-b border-dashed" style={{ borderColor: "#eab30833" }}>
        <StickyNote size={14} style={{ color: "#eab308", flexShrink: 0 }} />
        <span className="text-xs font-medium text-[var(--text-primary)] truncate">
          {label}
        </span>
      </div>
      {text && (
        <div className="px-2 py-1.5 text-[10px] text-[var(--text-muted)] whitespace-pre-wrap break-words">
          {text}
        </div>
      )}
    </div>
  );
}
