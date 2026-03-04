import { useState } from "react";
import { updateSettings } from "../../lib/tauri-commands";

export function PrivacySettings() {
  const [telemetryEnabled, setTelemetryEnabled] = useState(false);
  const [saving, setSaving] = useState(false);

  const handleToggle = async () => {
    const next = !telemetryEnabled;
    setTelemetryEnabled(next);
    setSaving(true);
    try {
      await updateSettings({ telemetry: next });
    } catch (err) {
      console.error("Failed to update telemetry setting:", err);
      setTelemetryEnabled(!next); // revert on failure
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <h3 className="text-sm font-medium text-[var(--text-primary)] mb-4">
          Privacy
        </h3>

        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-[var(--text-primary)]">
              Anonymous Telemetry
            </p>
            <p className="text-xs text-[var(--text-muted)] mt-0.5">
              Help improve Omni by sending anonymous usage data.
            </p>
          </div>

          <button
            onClick={handleToggle}
            disabled={saving}
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
              telemetryEnabled
                ? "bg-[var(--accent)]"
                : "bg-[var(--border)]"
            }`}
            role="switch"
            aria-checked={telemetryEnabled}
          >
            <span
              className={`inline-block h-4 w-4 rounded-full bg-white transition-transform ${
                telemetryEnabled ? "translate-x-6" : "translate-x-1"
              }`}
            />
          </button>
        </div>
      </div>
    </div>
  );
}
