import { useState } from "react";
import { updateSettings } from "../../lib/tauri-commands";
import { useSettingsStore } from "../../stores/settingsStore";
import { useUpdateStore } from "../../stores/updateStore";

const LOG_LEVELS = ["trace", "debug", "info", "warn", "error"] as const;
type LogLevel = (typeof LOG_LEVELS)[number];

const CONFIG_PATH =
  typeof window !== "undefined" && navigator.platform.startsWith("Win")
    ? "%APPDATA%\\omni\\config.toml"
    : "~/.config/omni/config.toml";

export function AdvancedSettings() {
  const [logLevel, setLogLevel] = useState<LogLevel>("info");
  const [showConfirm, setShowConfirm] = useState(false);

  const autoUpdate = useSettingsStore((s) => s.autoUpdate);
  const setAutoUpdate = useSettingsStore((s) => s.setAutoUpdate);

  const updateStatus = useUpdateStore((s) => s.status);
  const updateVersion = useUpdateStore((s) => s.version);
  const lastChecked = useUpdateStore((s) => s.lastChecked);
  const checkForUpdate = useUpdateStore((s) => s.checkForUpdate);

  const handleLogLevelChange = async (level: LogLevel) => {
    setLogLevel(level);
    try {
      await updateSettings({ logLevel: level });
    } catch (err) {
      console.error("Failed to update log level:", err);
    }
  };

  const handleReset = async () => {
    if (!showConfirm) {
      setShowConfirm(true);
      return;
    }

    try {
      await updateSettings({ reset: true });
      setLogLevel("info");
      setShowConfirm(false);
    } catch (err) {
      console.error("Failed to reset settings:", err);
    }
  };

  const formatLastChecked = () => {
    if (!lastChecked) return "Never";
    const diff = Date.now() - lastChecked;
    if (diff < 60_000) return "Just now";
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    return `${Math.floor(diff / 3_600_000)}h ago`;
  };

  return (
    <div className="space-y-6">
      {/* Auto-Update */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-sm font-medium text-[var(--text-primary)]">
            Automatic Updates
          </h3>
          <button
            role="switch"
            aria-checked={autoUpdate}
            onClick={() => setAutoUpdate(!autoUpdate)}
            className={`relative w-9 h-5 rounded-full transition-colors ${
              autoUpdate ? "bg-[var(--accent)]" : "bg-[var(--border)]"
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                autoUpdate ? "translate-x-4" : ""
              }`}
            />
          </button>
        </div>
        <p className="text-xs text-[var(--text-muted)] mb-4">
          Automatically check for updates on startup and every 6 hours.
        </p>
        <div className="flex items-center gap-3">
          <button
            onClick={checkForUpdate}
            disabled={updateStatus === "checking" || updateStatus === "downloading"}
            className="px-3 py-1.5 text-xs font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {updateStatus === "checking" ? "Checking..." : "Check Now"}
          </button>
          <span className="text-xs text-[var(--text-muted)]">
            {updateStatus === "available"
              ? `v${updateVersion} available`
              : `Last checked: ${formatLastChecked()}`}
          </span>
        </div>
      </div>

      {/* Config File Path */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <h3 className="text-sm font-medium text-[var(--text-primary)] mb-2">
          Configuration File
        </h3>
        <p className="text-xs text-[var(--text-muted)] mb-2">
          Read-only path to the active configuration file.
        </p>
        <div className="px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] font-mono text-xs text-[var(--text-secondary)] select-all">
          {CONFIG_PATH}
        </div>
      </div>

      {/* Log Level */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <h3 className="text-sm font-medium text-[var(--text-primary)] mb-2">
          Log Level
        </h3>
        <p className="text-xs text-[var(--text-muted)] mb-3">
          Controls the verbosity of application logging.
        </p>
        <select
          value={logLevel}
          onChange={(e) => handleLogLevelChange(e.target.value as LogLevel)}
          className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
        >
          {LOG_LEVELS.map((level) => (
            <option key={level} value={level}>
              {level.charAt(0).toUpperCase() + level.slice(1)}
            </option>
          ))}
        </select>
      </div>

      {/* Reset to Defaults */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <h3 className="text-sm font-medium text-[var(--text-primary)] mb-2">
          Reset Settings
        </h3>
        <p className="text-xs text-[var(--text-muted)] mb-4">
          Restore all settings to their default values. This action cannot be
          undone.
        </p>
        {showConfirm ? (
          <div className="flex items-center gap-3">
            <span className="text-xs text-[var(--text-secondary)]">
              Are you sure?
            </span>
            <button
              onClick={handleReset}
              className="px-3 py-1.5 text-xs font-medium rounded-md bg-red-500/20 text-red-400 hover:bg-red-500/30 transition-colors"
            >
              Confirm Reset
            </button>
            <button
              onClick={() => setShowConfirm(false)}
              className="px-3 py-1.5 text-xs font-medium rounded-md text-[var(--text-muted)] hover:bg-[var(--bg-hover)] transition-colors"
            >
              Cancel
            </button>
          </div>
        ) : (
          <button
            onClick={handleReset}
            className="px-4 py-2 text-sm font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors"
          >
            Reset to Defaults
          </button>
        )}
      </div>
    </div>
  );
}
