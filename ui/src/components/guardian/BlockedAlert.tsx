import { useState } from "react";
import { guardianOverride } from "../../lib/tauri-commands";

interface BlockedAlertProps {
  layer: string;
  reason: string;
  contentPreview: string;
  scanId: string;
  onOverride: (scanId: string) => void;
}

export function BlockedAlert({ layer, reason, contentPreview, scanId, onOverride }: BlockedAlertProps) {
  const [confirming, setConfirming] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleOverride = async () => {
    setLoading(true);
    setError(null);
    try {
      await guardianOverride(scanId);
      onOverride(scanId);
      setConfirming(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="rounded-lg p-4 flex flex-col gap-2"
      style={{
        backgroundColor: "var(--bg-secondary)",
        borderLeft: "4px solid var(--danger)",
        borderTop: "1px solid var(--danger)",
        borderRight: "1px solid var(--danger)",
        borderBottom: "1px solid var(--danger)",
      }}
    >
      <div className="flex items-center gap-2">
        <svg
          className="w-5 h-5 flex-shrink-0"
          style={{ color: "var(--danger)" }}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2}
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M12 9v2m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <span className="font-semibold text-sm" style={{ color: "var(--danger)" }}>
          Content Blocked
        </span>
        <span
          className="ml-auto px-2 py-0.5 rounded text-xs font-medium text-white"
          style={{ backgroundColor: "var(--danger)" }}
        >
          {layer}
        </span>
      </div>

      <p className="text-sm" style={{ color: "var(--text-primary)" }}>
        {reason}
      </p>

      {contentPreview && (
        <pre
          className="p-2 rounded text-xs font-mono overflow-x-auto overflow-y-auto whitespace-pre-wrap"
          style={{ backgroundColor: "var(--bg-primary)", color: "var(--text-muted)", maxHeight: "120px" }}
        >
          {contentPreview}
        </pre>
      )}

      {error && (
        <p className="text-xs" style={{ color: "var(--danger)" }}>
          Override failed: {error}
        </p>
      )}

      {!confirming ? (
        <button
          onClick={() => setConfirming(true)}
          className="self-end px-3 py-1 rounded text-xs font-medium transition-opacity hover:opacity-90"
          style={{ backgroundColor: "var(--warning)", color: "white" }}
        >
          Override
        </button>
      ) : (
        <div
          className="flex items-center justify-between p-3 rounded mt-1"
          style={{ backgroundColor: "var(--bg-primary)", border: "1px solid var(--danger)" }}
        >
          <span className="text-sm" style={{ color: "var(--danger)" }}>
            I understand the risk — proceed anyway
          </span>
          <div className="flex gap-2">
            <button
              onClick={() => { setConfirming(false); setError(null); }}
              disabled={loading}
              className="px-3 py-1 rounded text-xs"
              style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-primary)" }}
            >
              Cancel
            </button>
            <button
              onClick={handleOverride}
              disabled={loading}
              className="px-3 py-1 rounded text-xs font-medium text-white transition-opacity hover:opacity-90"
              style={{ backgroundColor: "var(--danger)", opacity: loading ? 0.6 : 1 }}
            >
              {loading ? "Overriding..." : "Confirm Override"}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
