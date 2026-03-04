import { useState } from "react";
import type { OmniEventEntry } from "../../stores/eventStore";
import { formatTimestamp } from "../../lib/formatters";
import { ActionDetail } from "./ActionDetail";

interface ActionEntryProps {
  event: OmniEventEntry;
}

const TYPE_COLORS: Record<string, string> = {
  "permission.request": "bg-yellow-500/20 text-yellow-400",
  "permission.grant": "bg-green-500/20 text-green-400",
  "permission.deny": "bg-red-500/20 text-red-400",
  "extension.install": "bg-blue-500/20 text-blue-400",
  "extension.uninstall": "bg-orange-500/20 text-orange-400",
  "guardian.block": "bg-red-500/20 text-red-400",
  "guardian.allow": "bg-green-500/20 text-green-400",
  "chat.message": "bg-purple-500/20 text-purple-400",
};

const DEFAULT_TYPE_COLOR = "bg-[var(--accent)]/20 text-[var(--accent)]";

function getBadgeColor(eventType: string): string {
  return TYPE_COLORS[eventType] ?? DEFAULT_TYPE_COLOR;
}

export function ActionEntry({ event }: ActionEntryProps) {
  const [expanded, setExpanded] = useState(false);

  const summary = JSON.stringify(event.payload).slice(0, 100);

  return (
    <div
      className="px-4 py-2.5 hover:bg-[var(--bg-hover)] transition-colors cursor-pointer"
      onClick={() => setExpanded((prev) => !prev)}
    >
      <div className="flex items-start gap-2">
        <span className="text-xs text-[var(--text-muted)] whitespace-nowrap pt-0.5">
          {formatTimestamp(event.timestamp)}
        </span>

        <span
          className={`text-[10px] font-medium px-1.5 py-0.5 rounded-full whitespace-nowrap ${getBadgeColor(event.eventType)}`}
        >
          {event.eventType}
        </span>

        <span className="text-xs text-[var(--text-secondary)] truncate">
          {summary}
        </span>
      </div>

      {expanded && (
        <div className="mt-2 ml-1">
          <ActionDetail payload={event.payload} />
        </div>
      )}
    </div>
  );
}
