import { useCallback, useEffect, useRef, useState } from "react";
import {
  getGuardianMetrics,
  getPendingBlocks,
  type GuardianMetricsDto,
  type PendingBlockDto,
} from "../../lib/tauri-commands";
import { useOmniEvent } from "../../hooks/useOmniEvents";
import { ScanResultCard } from "./ScanResultCard";
import { SensitivitySlider } from "./SensitivitySlider";

export function GuardianMonitor() {
  const [metrics, setMetrics] = useState<GuardianMetricsDto | null>(null);
  const [pendingBlocks, setPendingBlocks] = useState<PendingBlockDto[]>([]);
  const [error, setError] = useState<string | null>(null);
  const staleRef = useRef(false);

  const fetchData = useCallback(async () => {
    try {
      const [m, blocks] = await Promise.all([getGuardianMetrics(), getPendingBlocks()]);
      if (!staleRef.current) {
        setMetrics(m);
        setPendingBlocks(blocks);
        setError(null);
      }
    } catch (err) {
      if (!staleRef.current) {
        setError(String(err));
      }
    }
  }, []);

  useEffect(() => {
    staleRef.current = false;
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => {
      staleRef.current = true;
      clearInterval(interval);
    };
  }, [fetchData]);

  // React to real-time guardian events for instant UI updates
  useOmniEvent("omni:guardian-blocked", useCallback(() => {
    fetchData();
  }, [fetchData]));

  const blockRate =
    metrics && metrics.total_scans_db > 0
      ? ((metrics.total_blocked_db / metrics.total_scans_db) * 100).toFixed(1)
      : "0.0";

  const layerData = metrics
    ? [
        { label: "Signature", value: metrics.signature_blocks },
        { label: "Heuristic", value: metrics.heuristic_blocks },
        { label: "ML", value: metrics.ml_blocks },
        { label: "Policy", value: metrics.policy_blocks },
      ]
    : [];

  const maxLayerValue = Math.max(...layerData.map((d) => d.value), 1);

  const handleOverride = (scanId: string) => {
    setPendingBlocks((prev) => prev.filter((b) => b.scan_id !== scanId));
    // Refetch to ensure consistency with backend
    fetchData();
  };

  return (
    <div className="flex flex-col gap-6 p-6" style={{ backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}>
      <h1 className="text-2xl font-bold">Guardian Monitor</h1>

      {error && (
        <div className="px-4 py-2 rounded text-sm" style={{ backgroundColor: "var(--danger)", color: "white" }}>
          {error}
        </div>
      )}

      {/* Metric Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard label="Total Scans" value={metrics?.total_scans_db ?? 0} />
        <MetricCard label="Total Blocks" value={metrics?.total_blocked_db ?? 0} />
        <MetricCard label="Block Rate (%)" value={blockRate} />
        <MetricCard label="Avg Scan Time (ms)" value={metrics?.avg_scan_ms.toFixed(2) ?? "0.00"} />
      </div>

      {/* Per-Layer Breakdown */}
      <div
        className="rounded-lg p-4"
        style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border)" }}
      >
        <h2 className="text-lg font-semibold mb-4">Per-Layer Breakdown</h2>
        <div className="flex flex-col gap-3">
          {layerData.map((layer) => (
            <div key={layer.label} className="flex items-center gap-3">
              <span className="w-24 text-sm text-right" style={{ color: "var(--text-secondary)" }}>
                {layer.label}
              </span>
              <div className="flex-1 h-6 rounded overflow-hidden" style={{ backgroundColor: "var(--bg-hover)" }}>
                <div
                  className="h-full rounded transition-all duration-300"
                  style={{
                    width: `${(layer.value / maxLayerValue) * 100}%`,
                    backgroundColor: "var(--accent)",
                    minWidth: layer.value > 0 ? "2px" : "0",
                  }}
                />
              </div>
              <span className="w-12 text-sm font-mono" style={{ color: "var(--text-muted)" }}>
                {layer.value}
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Pending Blocks */}
      {pendingBlocks.length > 0 && (
        <div className="flex flex-col gap-3">
          <h2 className="text-lg font-semibold">Pending Blocks</h2>
          {pendingBlocks.map((block) => (
            <ScanResultCard
              key={block.scan_id}
              scanType={block.scan_type}
              layer={block.layer}
              reason={block.reason}
              confidence={block.confidence}
              contentPreview={block.content_preview}
              scanId={block.scan_id}
              onOverride={handleOverride}
            />
          ))}
        </div>
      )}

      {/* Sensitivity Slider */}
      <SensitivitySlider />
    </div>
  );
}

function MetricCard({ label, value }: { label: string; value: string | number }) {
  return (
    <div
      className="rounded-lg p-4 flex flex-col gap-1"
      style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border)" }}
    >
      <span className="text-xs font-medium uppercase tracking-wide" style={{ color: "var(--text-muted)" }}>
        {label}
      </span>
      <span className="text-2xl font-bold">{value}</span>
    </div>
  );
}
