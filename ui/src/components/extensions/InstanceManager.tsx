import { useEffect, useState, useCallback } from "react";
import { Power, PowerOff, Trash2, Plus } from "lucide-react";
import { useInstanceStore } from "../../stores/instanceStore";
import { useOmniEvent } from "../../hooks/useOmniEvents";

interface InstanceManagerProps {
  extensionId: string;
}

export function InstanceManager({ extensionId }: InstanceManagerProps) {
  const {
    instances,
    loading,
    loadInstances,
    createInstance,
    deleteInstance,
    activateInstance,
    deactivateInstance,
    toggleEnabled,
  } = useInstanceStore();

  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDisplayName, setNewDisplayName] = useState("");
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    loadInstances(extensionId);
  }, [extensionId, loadInstances]);

  // Refresh when backend emits instance lifecycle events
  const handleInstanceEvent = useCallback(() => {
    loadInstances(extensionId);
  }, [extensionId, loadInstances]);
  useOmniEvent("omni:extension-instance-created", handleInstanceEvent);
  useOmniEvent("omni:extension-instance-deleted", handleInstanceEvent);

  const extInstances = instances[extensionId] ?? [];

  const handleCreate = async () => {
    if (!newName.trim()) return;
    setBusy("create");
    try {
      await createInstance(
        extensionId,
        newName.trim(),
        newDisplayName.trim() || undefined,
      );
      setNewName("");
      setNewDisplayName("");
      setShowCreate(false);
    } finally {
      setBusy(null);
    }
  };

  const handleDelete = async (instanceId: string) => {
    setBusy(instanceId);
    try {
      await deleteInstance(instanceId);
    } finally {
      setBusy(null);
    }
  };

  const handleToggleActive = async (instanceId: string, active: boolean) => {
    setBusy(instanceId);
    try {
      if (active) {
        await deactivateInstance(instanceId);
      } else {
        await activateInstance(instanceId);
      }
    } finally {
      setBusy(null);
    }
  };

  const handleToggleEnabled = async (instanceId: string, enabled: boolean) => {
    setBusy(instanceId);
    try {
      await toggleEnabled(instanceId, !enabled);
    } finally {
      setBusy(null);
    }
  };

  return (
    <div
      className="rounded p-3 flex flex-col gap-2"
      style={{
        backgroundColor: "var(--bg-primary)",
        border: "1px solid var(--border)",
      }}
    >
      <div className="flex items-center justify-between">
        <span
          className="text-xs font-medium"
          style={{ color: "var(--text-secondary)" }}
        >
          Instances ({extInstances.length})
        </span>
        <button
          onClick={() => setShowCreate(!showCreate)}
          className="flex items-center gap-1 text-xs px-2 py-0.5 rounded transition-opacity hover:opacity-80"
          style={{ color: "var(--accent)" }}
        >
          <Plus size={10} /> New Instance
        </button>
      </div>

      {/* Create form */}
      {showCreate && (
        <div
          className="flex flex-col gap-2 p-2 rounded"
          style={{
            backgroundColor: "var(--bg-secondary)",
            border: "1px solid var(--border)",
          }}
        >
          <input
            type="text"
            placeholder="Instance name (e.g., support-bot)"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            className="px-2 py-1 rounded text-xs"
            style={{
              backgroundColor: "var(--bg-primary)",
              color: "var(--text-primary)",
              border: "1px solid var(--border)",
            }}
            onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          />
          <input
            type="text"
            placeholder="Display name (optional)"
            value={newDisplayName}
            onChange={(e) => setNewDisplayName(e.target.value)}
            className="px-2 py-1 rounded text-xs"
            style={{
              backgroundColor: "var(--bg-primary)",
              color: "var(--text-primary)",
              border: "1px solid var(--border)",
            }}
            onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          />
          <div className="flex gap-2">
            <button
              onClick={handleCreate}
              disabled={!newName.trim() || busy === "create"}
              className="px-2 py-1 rounded text-xs font-medium text-white"
              style={{
                backgroundColor: "var(--accent)",
                opacity: !newName.trim() || busy === "create" ? 0.5 : 1,
              }}
            >
              Create
            </button>
            <button
              onClick={() => setShowCreate(false)}
              className="px-2 py-1 rounded text-xs"
              style={{ color: "var(--text-muted)" }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Instance list */}
      {loading && extInstances.length === 0 && (
        <span className="text-xs" style={{ color: "var(--text-muted)" }}>
          Loading...
        </span>
      )}

      {extInstances.map((inst) => {
        const isBusy = busy === inst.instance_id;
        const isDefault = inst.instance_name === "default";
        const label =
          inst.display_name || inst.instance_name;

        return (
          <div
            key={inst.instance_id}
            className="flex items-center gap-2 p-2 rounded"
            style={{
              backgroundColor: "var(--bg-secondary)",
              border: "1px solid var(--border)",
              opacity: isBusy ? 0.5 : 1,
              pointerEvents: isBusy ? "none" : "auto",
            }}
          >
            {/* Name + status */}
            <div className="flex flex-col gap-0.5 flex-1 min-w-0">
              <span
                className="text-xs font-medium truncate"
                style={{ color: "var(--text-primary)" }}
              >
                {label}
              </span>
              <span
                className="text-[10px] font-mono truncate"
                style={{ color: "var(--text-muted)" }}
              >
                {inst.instance_id}
              </span>
            </div>

            {/* Status badge */}
            <span
              className="px-1.5 py-0.5 rounded text-[10px] font-medium text-white shrink-0"
              style={{
                backgroundColor: !inst.enabled
                  ? "var(--text-muted)"
                  : inst.active
                    ? "var(--success)"
                    : "var(--warning, #f59e0b)",
              }}
            >
              {!inst.enabled ? "Off" : inst.active ? "Running" : "Stopped"}
            </span>

            {/* Enable/Disable */}
            <button
              onClick={() => handleToggleEnabled(inst.instance_id, inst.enabled)}
              className="text-[10px] px-1.5 py-0.5 rounded"
              style={{
                color: "var(--text-secondary)",
                border: "1px solid var(--border)",
              }}
              title={inst.enabled ? "Disable instance" : "Enable instance"}
            >
              {inst.enabled ? "Disable" : "Enable"}
            </button>

            {/* Start/Stop */}
            {inst.enabled && (
              <button
                onClick={() =>
                  handleToggleActive(inst.instance_id, inst.active)
                }
                className="p-1 rounded transition-opacity hover:opacity-80"
                style={{
                  color: inst.active ? "var(--warning, #f59e0b)" : "var(--success)",
                }}
                title={inst.active ? "Stop instance" : "Start instance"}
              >
                {inst.active ? <PowerOff size={12} /> : <Power size={12} />}
              </button>
            )}

            {/* Delete (not for default) */}
            {!isDefault && (
              <button
                onClick={() => handleDelete(inst.instance_id)}
                className="p-1 rounded transition-opacity hover:opacity-80"
                style={{ color: "var(--danger)" }}
                title="Delete instance"
              >
                <Trash2 size={12} />
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
