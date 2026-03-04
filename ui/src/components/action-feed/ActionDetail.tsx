import { useState } from "react";

interface ActionDetailProps {
  payload: Record<string, unknown>;
}

export function ActionDetail({ payload }: ActionDetailProps) {
  const [copied, setCopied] = useState(false);
  const formatted = JSON.stringify(payload, null, 2);

  const handleCopy = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(formatted);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      console.error("Failed to copy to clipboard");
    }
  };

  return (
    <div className="rounded-md border border-[var(--border)] bg-[var(--bg-secondary)] overflow-hidden">
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-[var(--border)]">
        <span className="text-[10px] font-medium text-[var(--text-muted)] uppercase tracking-wider">
          Payload
        </span>
        <button
          onClick={handleCopy}
          className="text-[10px] px-2 py-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)] transition-colors"
        >
          {copied ? "Copied!" : "Copy"}
        </button>
      </div>
      <pre className="p-3 text-xs text-[var(--text-secondary)] overflow-x-auto whitespace-pre-wrap break-words">
        {formatted}
      </pre>
    </div>
  );
}
