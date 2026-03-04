import { useState } from "react";
import { Trash2, ChevronDown, ChevronUp, Layers } from "lucide-react";
import type { ExtensionDto } from "../../lib/tauri-commands";
import { useExtensionStore } from "../../stores/extensionStore";
import { InstanceManager } from "./InstanceManager";

interface ExtensionCardProps {
  extension: ExtensionDto;
}

export function ExtensionCard({ extension }: ExtensionCardProps) {
  const { uninstall, toggleEnabled } = useExtensionStore();
  const [expanded, setExpanded] = useState(false);
  const [busy, setBusy] = useState(false);

  const handleToggleEnabled = async () => {
    setBusy(true);
    try {
      await toggleEnabled(extension.id, !extension.enabled);
    } finally {
      setBusy(false);
    }
  };

  const handleUninstall = async () => {
    setBusy(true);
    try {
      await uninstall(extension.id);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="rounded-lg p-4 flex flex-col gap-3 transition-colors"
      style={{
        backgroundColor: "var(--bg-secondary)",
        border: "1px solid var(--border)",
        opacity: busy ? 0.6 : 1,
        pointerEvents: busy ? "none" : "auto",
      }}
    >
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="flex flex-col gap-0.5">
          <h3 className="font-semibold" style={{ color: "var(--text-primary)" }}>
            {extension.name}
          </h3>
          <span className="text-xs font-mono" style={{ color: "var(--text-muted)" }}>
            v{extension.version}
          </span>
        </div>
        <div className="flex items-center gap-2">
          {/* Instance count badge */}
          <span
            className="flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium"
            style={{
              backgroundColor: "var(--bg-primary)",
              color: "var(--text-secondary)",
              border: "1px solid var(--border)",
            }}
            title={`${extension.instance_count} instance${extension.instance_count !== 1 ? "s" : ""}`}
          >
            <Layers size={10} />
            {extension.instance_count}
          </span>
          {/* Enabled/Disabled indicator */}
          <span
            className="px-2 py-0.5 rounded text-xs font-medium text-white"
            style={{
              backgroundColor: extension.enabled
                ? extension.active
                  ? "var(--success)"
                  : "var(--warning, #f59e0b)"
                : "var(--text-muted)",
            }}
          >
            {!extension.enabled
              ? "Disabled"
              : extension.active
                ? "Active"
                : "Inactive"}
          </span>
        </div>
      </div>

      {/* Author */}
      {extension.author && (
        <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
          {extension.author}
        </p>
      )}

      {/* Description */}
      {extension.description && (
        <p className="text-sm leading-relaxed" style={{ color: "var(--text-primary)" }}>
          {extension.description}
        </p>
      )}

      {/* Stats row */}
      <div
        className="flex items-center gap-3 pt-2"
        style={{ borderTop: "1px solid var(--border)" }}
      >
        <span className="text-xs" style={{ color: "var(--text-muted)" }}>
          {extension.tools.length} tool{extension.tools.length !== 1 ? "s" : ""}
        </span>
        <span className="text-xs" style={{ color: "var(--text-muted)" }}>
          {extension.permissions.length} permission
          {extension.permissions.length !== 1 ? "s" : ""}
        </span>
        <button
          onClick={() => setExpanded(!expanded)}
          className="ml-auto flex items-center gap-1 text-xs transition-colors"
          style={{ color: "var(--accent)" }}
        >
          {expanded ? "Less" : "Details"}
          {expanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
        </button>
      </div>

      {/* Expanded details */}
      {expanded && (
        <div className="flex flex-col gap-3 pt-1">
          {/* Instance Manager */}
          <InstanceManager extensionId={extension.id} />

          {extension.tools.length > 0 && (
            <div>
              <span className="text-xs font-medium" style={{ color: "var(--text-secondary)" }}>
                Tools:
              </span>
              <div className="flex flex-wrap gap-1 mt-1">
                {extension.tools.map((tool) => (
                  <span
                    key={tool}
                    className="px-1.5 py-0.5 rounded text-xs font-mono"
                    style={{
                      backgroundColor: "var(--bg-primary)",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {tool}
                  </span>
                ))}
              </div>
            </div>
          )}
          {extension.permissions.length > 0 && (
            <div>
              <span className="text-xs font-medium" style={{ color: "var(--text-secondary)" }}>
                Permissions:
              </span>
              <div className="flex flex-wrap gap-1 mt-1">
                {extension.permissions.map((perm) => (
                  <span
                    key={perm}
                    className="px-1.5 py-0.5 rounded text-xs font-mono"
                    style={{
                      backgroundColor: "var(--bg-primary)",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {perm}
                  </span>
                ))}
              </div>
            </div>
          )}
          <p className="text-xs font-mono" style={{ color: "var(--text-muted)" }}>
            ID: {extension.id}
          </p>
        </div>
      )}

      {/* Action buttons */}
      <div className="flex items-center gap-2 pt-1">
        {/* Enable/Disable toggle (extension-level) */}
        <button
          onClick={handleToggleEnabled}
          className="flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition-opacity hover:opacity-80"
          style={{
            backgroundColor: extension.enabled ? "var(--bg-primary)" : "var(--accent)",
            color: extension.enabled ? "var(--text-secondary)" : "white",
            border: extension.enabled ? "1px solid var(--border)" : "none",
          }}
          title={extension.enabled ? "Disable all instances" : "Enable all instances"}
        >
          {extension.enabled ? "Disable" : "Enable"}
        </button>

        {/* Uninstall */}
        <button
          onClick={handleUninstall}
          className="flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium ml-auto transition-opacity hover:opacity-80"
          style={{ color: "var(--danger)" }}
          title="Uninstall extension and all instances"
        >
          <Trash2 size={12} /> Uninstall
        </button>
      </div>
    </div>
  );
}
