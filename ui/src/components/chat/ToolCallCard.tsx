import { useState } from "react";
import { ChevronDown, ChevronRight, Loader2, Check, AlertCircle, Clock } from "lucide-react";
import { DiffViewer } from "./DiffViewer";

interface ToolCallCardProps {
  name: string;
  arguments: string;
  result?: string;
  status: "pending" | "running" | "completed" | "error";
}

const statusConfig = {
  pending: {
    icon: Clock,
    label: "Pending",
    color: "text-[var(--text-muted)]",
    bg: "bg-[var(--bg-secondary)]",
  },
  running: {
    icon: Loader2,
    label: "Running",
    color: "text-[var(--accent)]",
    bg: "bg-[var(--accent)]/10",
  },
  completed: {
    icon: Check,
    label: "Completed",
    color: "text-green-400",
    bg: "bg-green-400/10",
  },
  error: {
    icon: AlertCircle,
    label: "Error",
    color: "text-red-400",
    bg: "bg-red-400/10",
  },
} as const;

function isDiffContent(text: string): boolean {
  const lines = text.split("\n");
  // Check for unified diff markers: must have at least one --- and +++ pair, or @@ hunk headers
  const hasMinusMinus = lines.some((l) => l.startsWith("--- "));
  const hasPlusPlus = lines.some((l) => l.startsWith("+++ "));
  const hasHunkHeader = lines.some((l) => l.startsWith("@@ "));
  // Also accept "diff --git" style headers
  const hasDiffHeader = lines.some((l) => l.startsWith("diff --git ") || l.startsWith("diff -"));

  return (hasMinusMinus && hasPlusPlus && hasHunkHeader) || (hasDiffHeader && hasHunkHeader);
}

/** Try to extract diff content from a JSON-structured tool result. */
function extractDiffFromJson(text: string): string | null {
  try {
    const parsed = JSON.parse(text);
    if (typeof parsed === "object" && parsed !== null) {
      // Check common field names for diff content
      for (const key of ["diff", "patch", "changes", "output"]) {
        if (typeof parsed[key] === "string" && isDiffContent(parsed[key])) {
          return parsed[key];
        }
      }
    }
  } catch {
    // Not JSON -- that's fine
  }
  return null;
}

export function ToolCallCard({ name, arguments: args, result, status }: ToolCallCardProps) {
  const [expanded, setExpanded] = useState(false);
  const config = statusConfig[status];
  const StatusIcon = config.icon;

  const formatJson = (raw: string): string => {
    try {
      return JSON.stringify(JSON.parse(raw), null, 2);
    } catch {
      return raw;
    }
  };

  const renderResult = (raw: string) => {
    const formatted = formatJson(raw);

    // Check if the entire result is a diff
    if (isDiffContent(formatted)) {
      return <DiffViewer diff={formatted} />;
    }

    // Check if result is JSON wrapping a diff field
    const nestedDiff = extractDiffFromJson(raw);
    if (nestedDiff) {
      return <DiffViewer diff={nestedDiff} />;
    }

    return (
      <pre
        className={`text-xs font-mono rounded p-2 overflow-x-auto max-h-48 overflow-y-auto ${
          status === "error"
            ? "bg-red-400/5 text-red-300"
            : "bg-[var(--bg-primary)] text-[var(--text-secondary)]"
        }`}
      >
        {formatted}
      </pre>
    );
  };

  return (
    <div className="mx-4 mb-3 rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] overflow-hidden">
      <button
        onClick={() => setExpanded((prev) => !prev)}
        className="flex w-full items-center gap-2 px-3 py-2 text-sm hover:bg-[var(--bg-hover)] transition-colors"
      >
        {expanded ? (
          <ChevronDown size={14} className="text-[var(--text-muted)] shrink-0" />
        ) : (
          <ChevronRight size={14} className="text-[var(--text-muted)] shrink-0" />
        )}

        <span className="font-mono font-medium text-[var(--text-primary)] truncate">
          {name}
        </span>

        <span className="ml-auto flex items-center gap-1.5 shrink-0">
          <StatusIcon
            size={14}
            className={`${config.color} ${status === "running" ? "animate-spin" : ""}`}
          />
          <span className={`text-xs ${config.color}`}>{config.label}</span>
        </span>
      </button>

      {expanded && (
        <div className="border-t border-[var(--border)] px-3 py-2 space-y-2">
          <div>
            <p className="text-xs font-semibold text-[var(--text-muted)] mb-1">Arguments</p>
            <pre className="text-xs font-mono bg-[var(--bg-primary)] rounded p-2 overflow-x-auto text-[var(--text-secondary)] max-h-48 overflow-y-auto">
              {formatJson(args)}
            </pre>
          </div>

          {result !== undefined && (
            <div>
              <p className="text-xs font-semibold text-[var(--text-muted)] mb-1">Result</p>
              {renderResult(result)}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
