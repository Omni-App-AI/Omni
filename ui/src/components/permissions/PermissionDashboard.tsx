import { useEffect, useState } from "react";
import { killSwitch, getAuditLog, type AuditEntryDto } from "../../lib/tauri-commands";
import { formatTimestamp } from "../../lib/formatters";

export function PermissionDashboard() {
  const [auditLog, setAuditLog] = useState<AuditEntryDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [killSwitchResult, setKillSwitchResult] = useState<number | null>(null);

  useEffect(() => {
    getAuditLog()
      .then(setAuditLog)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const handleKillSwitch = async () => {
    try {
      const revoked = await killSwitch();
      setKillSwitchResult(revoked);
      // Refresh audit log after kill switch
      const updated = await getAuditLog();
      setAuditLog(updated);
    } catch (err) {
      console.error("Kill switch failed:", err);
    }
  };

  return (
    <div className="flex flex-col gap-6 p-6" style={{ backgroundColor: "var(--bg-primary)", color: "var(--text-primary)" }}>
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Permission Dashboard</h1>
        <button
          onClick={handleKillSwitch}
          className="px-4 py-2 rounded font-semibold text-white transition-opacity hover:opacity-90"
          style={{ backgroundColor: "var(--danger)" }}
        >
          Kill Switch
        </button>
      </div>

      {killSwitchResult !== null && (
        <div
          className="px-4 py-2 rounded text-sm"
          style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--warning)", color: "var(--warning)" }}
        >
          Kill switch activated: {killSwitchResult} permission(s) revoked.
        </div>
      )}

      <div
        className="rounded-lg overflow-hidden"
        style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border)" }}
      >
        <h2 className="text-lg font-semibold px-4 py-3" style={{ borderBottom: "1px solid var(--border)" }}>
          Audit Log
        </h2>

        {loading ? (
          <div className="p-4" style={{ color: "var(--text-muted)" }}>Loading audit log...</div>
        ) : auditLog.length === 0 ? (
          <div className="p-4" style={{ color: "var(--text-muted)" }}>No audit entries found.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-secondary)" }}>
                  <th className="text-left px-4 py-2 font-medium">Timestamp</th>
                  <th className="text-left px-4 py-2 font-medium">Event Type</th>
                  <th className="text-left px-4 py-2 font-medium">Extension</th>
                  <th className="text-left px-4 py-2 font-medium">Capability</th>
                  <th className="text-left px-4 py-2 font-medium">Decision</th>
                </tr>
              </thead>
              <tbody>
                {auditLog.map((entry, i) => (
                  <tr
                    key={`${entry.timestamp}-${i}`}
                    className="transition-colors"
                    style={{ borderTop: "1px solid var(--border)" }}
                    onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--bg-hover)")}
                    onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
                  >
                    <td className="px-4 py-2" style={{ color: "var(--text-muted)" }}>
                      {formatTimestamp(entry.timestamp)}
                    </td>
                    <td className="px-4 py-2">{entry.event_type}</td>
                    <td className="px-4 py-2 font-mono text-xs">{entry.extension_id}</td>
                    <td className="px-4 py-2">{entry.capability}</td>
                    <td className="px-4 py-2">
                      <span
                        className="px-2 py-0.5 rounded text-xs font-medium"
                        style={{
                          backgroundColor: entry.decision.toLowerCase().includes("allow")
                            ? "var(--success)"
                            : "var(--danger)",
                          color: "white",
                        }}
                      >
                        {entry.decision}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
