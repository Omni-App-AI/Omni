import { useState, useRef, useEffect } from "react";
import { Plus, X, ChevronDown } from "lucide-react";
import type { BindingDto, ExtensionDto, ExtensionInstanceDto } from "../../lib/tauri-commands";

interface InlineBindingEditorProps {
  channelId: string;
  bindings: BindingDto[];
  extensions: ExtensionDto[];
  /** All extension instances -- used for instance-level binding display. */
  instances?: ExtensionInstanceDto[];
  onAddBinding: (
    channelInstance: string,
    extensionId: string,
    peerFilter?: string,
    groupFilter?: string,
    priority?: number,
  ) => Promise<string | void>;
  onRemoveBinding: (bindingId: string) => Promise<void>;
}

/**
 * Resolve a binding's extension_id (which may be an instance_id like "ext::name")
 * to a human-readable label.
 */
function getBindingLabel(
  extensions: ExtensionDto[],
  instances: ExtensionInstanceDto[] | undefined,
  extensionId: string,
): string {
  // If it's an instance_id (contains "::"), look up instance first
  if (extensionId.includes("::")) {
    const inst = instances?.find((i) => i.instance_id === extensionId);
    if (inst) {
      const ext = extensions.find((e) => e.id === inst.extension_id);
      const extName = ext?.name ?? inst.extension_id;
      const label = inst.display_name || inst.instance_name;
      return label === "default" ? extName : `${extName} (${label})`;
    }
  }
  // Fall back to extension name
  const ext = extensions.find((e) => e.id === extensionId);
  return ext?.name ?? extensionId;
}

export function InlineBindingEditor({
  channelId,
  bindings,
  extensions,
  instances,
  onAddBinding,
  onRemoveBinding,
}: InlineBindingEditorProps) {
  const [showDropdown, setShowDropdown] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [peerFilter, setPeerFilter] = useState("");
  const [groupFilter, setGroupFilter] = useState("");
  const [priority, setPriority] = useState("0");
  const [removing, setRemoving] = useState<string | null>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close dropdown on outside click
  useEffect(() => {
    if (!showDropdown) return;
    const handler = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
        setShowAdvanced(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showDropdown]);

  const boundIds = new Set(bindings.map((b) => b.extension_id));

  // Build available items: show instances if available, otherwise extensions
  const availableItems: { id: string; name: string; subtitle: string }[] = [];
  if (instances && instances.length > 0) {
    for (const inst of instances) {
      if (boundIds.has(inst.instance_id)) continue;
      const ext = extensions.find((e) => e.id === inst.extension_id);
      const extName = ext?.name ?? inst.extension_id;
      const label = inst.display_name || inst.instance_name;
      availableItems.push({
        id: inst.instance_id,
        name: label === "default" ? extName : `${extName} (${label})`,
        subtitle: inst.instance_id,
      });
    }
  } else {
    for (const ext of extensions) {
      if (boundIds.has(ext.id)) continue;
      availableItems.push({
        id: ext.id,
        name: ext.name,
        subtitle: ext.id,
      });
    }
  }

  const handleAdd = async (extensionId: string) => {
    await onAddBinding(
      channelId,
      extensionId,
      showAdvanced && peerFilter ? peerFilter : undefined,
      showAdvanced && groupFilter ? groupFilter : undefined,
      showAdvanced ? parseInt(priority) || 0 : undefined,
    );
    setShowDropdown(false);
    setShowAdvanced(false);
    setPeerFilter("");
    setGroupFilter("");
    setPriority("0");
  };

  const handleRemove = async (bindingId: string) => {
    setRemoving(null);
    await onRemoveBinding(bindingId);
  };

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <span className="text-xs font-medium" style={{ color: "var(--text-muted)" }}>
        Extensions:
      </span>

      {bindings.length === 0 && (
        <span className="text-xs italic" style={{ color: "var(--text-muted)" }}>
          All extensions
        </span>
      )}

      {bindings.map((b) => (
        <span
          key={b.id}
          className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
          style={{
            backgroundColor: "color-mix(in srgb, var(--accent) 15%, transparent)",
            color: "var(--accent)",
          }}
        >
          {getBindingLabel(extensions, instances, b.extension_id)}
          {removing === b.id ? (
            <span className="flex items-center gap-1 ml-1">
              <button
                onClick={() => handleRemove(b.id)}
                className="text-xs underline"
                style={{ color: "var(--error)" }}
              >
                remove
              </button>
              <button
                onClick={() => setRemoving(null)}
                className="text-xs underline"
                style={{ color: "var(--text-muted)" }}
              >
                cancel
              </button>
            </span>
          ) : (
            <button
              onClick={() => setRemoving(b.id)}
              className="hover:opacity-70 transition-opacity"
            >
              <X size={12} />
            </button>
          )}
        </span>
      ))}

      {/* Add button */}
      <div className="relative" ref={dropdownRef}>
        <button
          onClick={() => setShowDropdown(!showDropdown)}
          className="inline-flex items-center justify-center w-5 h-5 rounded-full transition-colors"
          style={{
            backgroundColor: "var(--bg-primary)",
            color: "var(--text-muted)",
            border: "1px solid var(--border)",
          }}
          title="Assign extension"
        >
          <Plus size={12} />
        </button>

        {showDropdown && (
          <div
            className="absolute left-0 top-7 z-50 w-64 rounded-lg shadow-lg overflow-hidden"
            style={{
              backgroundColor: "var(--bg-secondary)",
              border: "1px solid var(--border)",
            }}
          >
            <div
              className="px-3 py-2 text-xs font-semibold"
              style={{
                color: "var(--text-muted)",
                borderBottom: "1px solid var(--border)",
              }}
            >
              Assign an extension
            </div>

            {availableItems.length === 0 ? (
              <div
                className="px-3 py-4 text-xs text-center"
                style={{ color: "var(--text-muted)" }}
              >
                {extensions.length === 0
                  ? "No extensions installed."
                  : "All instances already assigned."}
              </div>
            ) : (
              <div className="max-h-48 overflow-y-auto">
                {availableItems.map((item) => (
                  <button
                    key={item.id}
                    onClick={() => handleAdd(item.id)}
                    className="w-full text-left px-3 py-2 text-xs transition-colors hover:bg-[var(--bg-hover)]"
                    style={{ color: "var(--text-primary)" }}
                  >
                    <div className="font-medium">{item.name}</div>
                    <div style={{ color: "var(--text-muted)" }}>{item.subtitle}</div>
                  </button>
                ))}
              </div>
            )}

            {/* Advanced options */}
            <div style={{ borderTop: "1px solid var(--border)" }}>
              <button
                onClick={() => setShowAdvanced(!showAdvanced)}
                className="w-full flex items-center justify-between px-3 py-2 text-xs transition-colors hover:bg-[var(--bg-hover)]"
                style={{ color: "var(--text-muted)" }}
              >
                Advanced options
                <ChevronDown
                  size={12}
                  style={{
                    transform: showAdvanced ? "rotate(180deg)" : undefined,
                    transition: "transform 0.2s",
                  }}
                />
              </button>

              {showAdvanced && (
                <div className="px-3 pb-3 flex flex-col gap-2">
                  <div className="flex flex-col gap-1">
                    <label
                      className="text-xs"
                      style={{ color: "var(--text-muted)" }}
                    >
                      Peer filter (glob)
                    </label>
                    <input
                      type="text"
                      value={peerFilter}
                      onChange={(e) => setPeerFilter(e.target.value)}
                      placeholder="e.g. admin-*"
                      className="px-2 py-1 rounded text-xs"
                      style={{
                        backgroundColor: "var(--bg-primary)",
                        color: "var(--text-primary)",
                        border: "1px solid var(--border)",
                      }}
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <label
                      className="text-xs"
                      style={{ color: "var(--text-muted)" }}
                    >
                      Group filter (glob)
                    </label>
                    <input
                      type="text"
                      value={groupFilter}
                      onChange={(e) => setGroupFilter(e.target.value)}
                      placeholder="e.g. support-*"
                      className="px-2 py-1 rounded text-xs"
                      style={{
                        backgroundColor: "var(--bg-primary)",
                        color: "var(--text-primary)",
                        border: "1px solid var(--border)",
                      }}
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <label
                      className="text-xs"
                      style={{ color: "var(--text-muted)" }}
                    >
                      Priority
                    </label>
                    <input
                      type="number"
                      value={priority}
                      onChange={(e) => setPriority(e.target.value)}
                      className="px-2 py-1 rounded text-xs w-20"
                      style={{
                        backgroundColor: "var(--bg-primary)",
                        color: "var(--text-primary)",
                        border: "1px solid var(--border)",
                      }}
                    />
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
