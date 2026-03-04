import { useState } from "react";
import { guardianOverride } from "../../lib/tauri-commands";

interface ScanResultCardProps {
  scanType: string;
  layer: string;
  reason: string;
  confidence: number;
  contentPreview: string;
  scanId: string;
  onOverride?: (scanId: string) => void;
}

export function ScanResultCard({
  scanType,
  layer,
  reason,
  confidence,
  contentPreview,
  scanId,
  onOverride,
}: ScanResultCardProps) {
  const [confirming, setConfirming] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleOverride = async () => {
    setLoading(true);
    setError(null);
    try {
      await guardianOverride(scanId);
      onOverride?.(scanId);
      setConfirming(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="rounded-lg p-4 flex flex-col gap-3"
      style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border)" }}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span
            className="px-2 py-0.5 rounded text-xs font-medium text-white"
            style={{ backgroundColor: "var(--accent)" }}
          >
            {layer}
          </span>
          <span className="text-sm" style={{ color: "var(--text-secondary)" }}>
            {scanType}
          </span>
        </div>
        <span className="text-sm font-mono" style={{ color: "var(--text-muted)" }}>
          {scanId.slice(0, 8)}
        </span>
      </div>

      <div>
        <span className="text-sm font-medium" style={{ color: "var(--text-secondary)" }}>
          Reason
        </span>
        <p className="text-sm mt-0.5" style={{ color: "var(--text-primary)" }}>
          {reason}
        </p>
      </div>

      <div>
        <span className="text-sm font-medium" style={{ color: "var(--text-secondary)" }}>
          Confidence
        </span>
        <div className="flex items-center gap-2 mt-1">
          <div className="flex-1 h-2 rounded overflow-hidden" style={{ backgroundColor: "var(--bg-hover)" }}>
            <div
              className="h-full rounded transition-all"
              style={{
                width: `${confidence * 100}%`,
                backgroundColor:
                  confidence > 0.8 ? "var(--danger)" : confidence > 0.5 ? "var(--warning)" : "var(--success)",
              }}
            />
          </div>
          <span className="text-xs font-mono" style={{ color: "var(--text-muted)" }}>
            {(confidence * 100).toFixed(0)}%
          </span>
        </div>
      </div>

      <div>
        <span className="text-sm font-medium" style={{ color: "var(--text-secondary)" }}>
          Content Preview
        </span>
        <pre
          className="mt-0.5 p-2 rounded text-xs font-mono overflow-x-auto overflow-y-auto whitespace-pre-wrap"
          style={{ backgroundColor: "var(--bg-primary)", color: "var(--text-muted)", maxHeight: "120px" }}
        >
          {contentPreview}
        </pre>
      </div>

      {error && (
        <p className="text-xs" style={{ color: "var(--danger)" }}>
          Override failed: {error}
        </p>
      )}

      {onOverride && !confirming && (
        <button
          onClick={() => setConfirming(true)}
          className="self-end px-3 py-1.5 rounded text-sm font-medium transition-opacity hover:opacity-90"
          style={{ backgroundColor: "var(--warning)", color: "white" }}
        >
          Override
        </button>
      )}

      {confirming && (
        <div
          className="flex items-center justify-between p-3 rounded"
          style={{ backgroundColor: "var(--bg-primary)", border: "1px solid var(--danger)" }}
        >
          <span className="text-sm" style={{ color: "var(--danger)" }}>
            I understand the risk — proceed anyway
          </span>
          <div className="flex gap-2">
            <button
              onClick={() => { setConfirming(false); setError(null); }}
              disabled={loading}
              className="px-3 py-1 rounded text-sm"
              style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-primary)" }}
            >
              Cancel
            </button>
            <button
              onClick={handleOverride}
              disabled={loading}
              className="px-3 py-1 rounded text-sm font-medium text-white transition-opacity hover:opacity-90"
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
