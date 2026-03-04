import { useState, useEffect, useMemo } from "react";
import {
  X,
  ArrowLeft,
  ArrowRight,
  Loader2,
  Check,
  Search,
  AlertCircle,
} from "lucide-react";
import { ChannelIcon } from "./ChannelIcon";
import {
  CHANNEL_META,
  CHANNEL_CATEGORIES,
  getChannelMeta,
  type ChannelCategory,
} from "./channelMeta";
import {
  listExtensions,
  listExtensionInstances,
  type ExtensionDto,
  type ExtensionInstanceDto,
} from "../../lib/tauri-commands";

interface AddChannelWizardProps {
  preselectedType?: string | null;
  onClose: () => void;
  onComplete: (result: {
    channelType: string;
    instanceId: string;
    credentialType: string;
    credentials: Record<string, string>;
    extensionId: string | null;
  }) => Promise<void>;
}

type Step = 1 | 2 | 3;

export function AddChannelWizard({
  preselectedType,
  onClose,
  onComplete,
}: AddChannelWizardProps) {
  // --- State ---
  const [step, setStep] = useState<Step>(preselectedType ? 2 : 1);
  const [selectedType, setSelectedType] = useState<string>(preselectedType ?? "");
  const [searchQuery, setSearchQuery] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<ChannelCategory | "all">(
    "all",
  );

  // Step 2
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [instanceId, setInstanceId] = useState("default");
  const [showInstanceName, setShowInstanceName] = useState(false);

  // Step 3
  const [extensions, setExtensions] = useState<ExtensionDto[]>([]);
  const [instances, setInstances] = useState<ExtensionInstanceDto[]>([]);
  const [bindingMode, setBindingMode] = useState<"all" | "specific">("all");
  const [selectedExtension, setSelectedExtension] = useState("");

  // Execution
  const [executing, setExecuting] = useState(false);
  const [executionStep, setExecutionStep] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Load extensions and instances on mount
  useEffect(() => {
    listExtensions()
      .then(setExtensions)
      .catch(() => {});
    listExtensionInstances()
      .then(setInstances)
      .catch(() => {});
  }, []);

  const meta = selectedType ? getChannelMeta(selectedType) : null;

  // --- Step 1: Filter channels ---
  const filteredChannels = useMemo(() => {
    const entries = Object.entries(CHANNEL_META);
    return entries.filter(([, m]) => {
      if (categoryFilter !== "all" && m.category !== categoryFilter) return false;
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        return (
          m.displayName.toLowerCase().includes(q) ||
          m.description.toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [searchQuery, categoryFilter]);

  // --- Step 2: Validation ---
  const requiredFieldsFilled = meta
    ? meta.fields
        .filter((f) => f.required)
        .every((f) => (credentials[f.key] ?? "").trim().length > 0)
    : false;

  const canProceedStep2 =
    meta?.authMode === "qr_code" || meta?.authMode === "none" || requiredFieldsFilled;

  // --- Handlers ---
  const selectChannel = (type: string) => {
    setSelectedType(type);
    setCredentials({});
    setStep(2);
  };

  const handleFinish = async () => {
    setExecuting(true);
    setError(null);
    try {
      setExecutionStep("Creating channel...");
      await onComplete({
        channelType: selectedType,
        instanceId: showInstanceName ? instanceId : "default",
        credentialType: meta?.credentialType ?? "",
        credentials,
        extensionId:
          bindingMode === "specific" && selectedExtension
            ? selectedExtension
            : null,
      });
      // onComplete handles closing
    } catch (e) {
      setError(String(e));
      setExecuting(false);
      setExecutionStep(null);
    }
  };

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-40"
        style={{ backgroundColor: "rgba(0, 0, 0, 0.5)" }}
        onClick={onClose}
      />

      {/* Modal */}
      <div
        className="fixed inset-4 md:inset-auto md:top-1/2 md:left-1/2 md:-translate-x-1/2 md:-translate-y-1/2 md:w-[640px] md:max-h-[80vh] z-50 rounded-xl shadow-2xl flex flex-col overflow-hidden"
        style={{
          backgroundColor: "var(--bg-primary)",
          border: "1px solid var(--border)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between p-4"
          style={{ borderBottom: "1px solid var(--border)" }}
        >
          <div className="flex items-center gap-3">
            {step > 1 && !executing && (
              <button
                onClick={() => setStep((step - 1) as Step)}
                className="p-1 rounded hover:opacity-70 transition-opacity"
                style={{ color: "var(--text-muted)" }}
              >
                <ArrowLeft size={18} />
              </button>
            )}
            <span
              className="text-sm font-semibold"
              style={{ color: "var(--text-primary)" }}
            >
              {step === 1
                ? "Choose a Platform"
                : step === 2
                  ? `${meta?.displayName ?? ""} Setup`
                  : "Route Messages"}
            </span>
          </div>

          <div className="flex items-center gap-3">
            {/* Step indicator */}
            <div className="flex items-center gap-1">
              {[1, 2, 3].map((s) => (
                <div
                  key={s}
                  className="w-2 h-2 rounded-full transition-colors"
                  style={{
                    backgroundColor:
                      s <= step ? "var(--accent)" : "var(--border)",
                  }}
                />
              ))}
            </div>
            <button
              onClick={onClose}
              className="p-1 rounded hover:opacity-70 transition-opacity"
              style={{ color: "var(--text-muted)" }}
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-4">
          {/* STEP 1: Choose Platform */}
          {step === 1 && (
            <div className="flex flex-col gap-4">
              {/* Search */}
              <div
                className="flex items-center gap-2 px-3 py-2 rounded-lg"
                style={{
                  backgroundColor: "var(--bg-secondary)",
                  border: "1px solid var(--border)",
                }}
              >
                <Search
                  size={14}
                  style={{ color: "var(--text-muted)" }}
                />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search platforms..."
                  className="flex-1 bg-transparent text-sm outline-none"
                  style={{ color: "var(--text-primary)" }}
                  autoFocus
                />
              </div>

              {/* Category tabs */}
              <div className="flex flex-wrap gap-1.5">
                <button
                  onClick={() => setCategoryFilter("all")}
                  className="px-2.5 py-1 rounded-full text-xs font-medium transition-colors"
                  style={{
                    backgroundColor:
                      categoryFilter === "all"
                        ? "var(--accent)"
                        : "var(--bg-secondary)",
                    color: categoryFilter === "all" ? "white" : "var(--text-secondary)",
                  }}
                >
                  All
                </button>
                {CHANNEL_CATEGORIES.map((cat) => (
                  <button
                    key={cat.key}
                    onClick={() => setCategoryFilter(cat.key)}
                    className="px-2.5 py-1 rounded-full text-xs font-medium transition-colors"
                    style={{
                      backgroundColor:
                        categoryFilter === cat.key
                          ? "var(--accent)"
                          : "var(--bg-secondary)",
                      color:
                        categoryFilter === cat.key
                          ? "white"
                          : "var(--text-secondary)",
                    }}
                  >
                    {cat.label}
                  </button>
                ))}
              </div>

              {/* Channel grid */}
              <div className="grid grid-cols-2 gap-2">
                {filteredChannels.map(([type, m]) => (
                  <button
                    key={type}
                    onClick={() => selectChannel(type)}
                    className="flex items-center gap-3 p-3 rounded-lg text-left transition-colors"
                    style={{
                      backgroundColor: "var(--bg-secondary)",
                      border: "1px solid var(--border)",
                    }}
                    onMouseEnter={(e) =>
                      (e.currentTarget.style.borderColor = "var(--accent)")
                    }
                    onMouseLeave={(e) =>
                      (e.currentTarget.style.borderColor = "var(--border)")
                    }
                  >
                    <div
                      className="w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0"
                      style={{
                        backgroundColor:
                          "color-mix(in srgb, var(--accent) 15%, transparent)",
                      }}
                    >
                      <ChannelIcon
                        iconName={m.icon}
                        size={18}
                        style={{ color: "var(--accent)" }}
                      />
                    </div>
                    <div className="min-w-0">
                      <div
                        className="text-sm font-medium truncate"
                        style={{ color: "var(--text-primary)" }}
                      >
                        {m.displayName}
                      </div>
                      <div
                        className="text-xs truncate"
                        style={{ color: "var(--text-muted)" }}
                      >
                        {m.description}
                      </div>
                    </div>
                  </button>
                ))}
              </div>

              {filteredChannels.length === 0 && (
                <div
                  className="text-center py-8 text-sm"
                  style={{ color: "var(--text-muted)" }}
                >
                  No platforms match your search.
                </div>
              )}
            </div>
          )}

          {/* STEP 2: Enter Credentials */}
          {step === 2 && meta && (
            <div className="flex flex-col gap-4">
              {meta.authMode === "qr_code" ? (
                <div
                  className="rounded-lg p-4 text-center"
                  style={{
                    backgroundColor: "var(--bg-secondary)",
                    border: "1px solid var(--border)",
                  }}
                >
                  <p
                    className="text-sm font-medium"
                    style={{ color: "var(--text-primary)" }}
                  >
                    QR Code Authentication
                  </p>
                  <p
                    className="text-xs mt-2 leading-relaxed"
                    style={{ color: "var(--text-muted)" }}
                  >
                    After setup completes, a QR code will appear. Scan it with
                    your {meta.displayName} app to link your account.
                  </p>
                </div>
              ) : meta.authMode === "none" ? (
                <div
                  className="rounded-lg p-4 text-center"
                  style={{
                    backgroundColor: "var(--bg-secondary)",
                    border: "1px solid var(--border)",
                  }}
                >
                  <p
                    className="text-sm font-medium"
                    style={{ color: "var(--text-primary)" }}
                  >
                    No credentials needed
                  </p>
                  <p
                    className="text-xs mt-2"
                    style={{ color: "var(--text-muted)" }}
                  >
                    This channel can be connected without any authentication.
                  </p>
                </div>
              ) : null}

              {/* Credential fields */}
              {meta.fields.length > 0 && (
                <div className="flex flex-col gap-3">
                  {meta.fields.map((field) => (
                    <div key={field.key} className="flex flex-col gap-1">
                      <label
                        className="text-xs font-medium"
                        style={{ color: "var(--text-muted)" }}
                      >
                        {field.label}
                        {!field.required && (
                          <span className="ml-1 opacity-60">(optional)</span>
                        )}
                      </label>
                      {field.type === "textarea" ? (
                        <textarea
                          value={credentials[field.key] ?? ""}
                          onChange={(e) =>
                            setCredentials((prev) => ({
                              ...prev,
                              [field.key]: e.target.value,
                            }))
                          }
                          placeholder={field.placeholder}
                          rows={4}
                          className="px-3 py-2 rounded text-sm resize-y"
                          style={{
                            backgroundColor: "var(--bg-secondary)",
                            color: "var(--text-primary)",
                            border: "1px solid var(--border)",
                          }}
                        />
                      ) : (
                        <input
                          type={field.type}
                          value={credentials[field.key] ?? ""}
                          onChange={(e) =>
                            setCredentials((prev) => ({
                              ...prev,
                              [field.key]: e.target.value,
                            }))
                          }
                          placeholder={field.placeholder}
                          className="px-3 py-2 rounded text-sm"
                          style={{
                            backgroundColor: "var(--bg-secondary)",
                            color: "var(--text-primary)",
                            border: "1px solid var(--border)",
                          }}
                        />
                      )}
                      {field.helpText && (
                        <span
                          className="text-xs"
                          style={{ color: "var(--text-muted)" }}
                        >
                          {field.helpText}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {/* Instance name (collapsed by default) */}
              <div
                className="pt-2"
                style={{ borderTop: "1px solid var(--border)" }}
              >
                {showInstanceName ? (
                  <div className="flex flex-col gap-1">
                    <label
                      className="text-xs font-medium"
                      style={{ color: "var(--text-muted)" }}
                    >
                      Instance Name
                    </label>
                    <input
                      type="text"
                      value={instanceId}
                      onChange={(e) =>
                        setInstanceId(
                          e.target.value
                            .toLowerCase()
                            .replace(/[^a-z0-9-]/g, "-"),
                        )
                      }
                      placeholder="e.g. production, bot-2"
                      className="px-3 py-2 rounded text-sm"
                      style={{
                        backgroundColor: "var(--bg-secondary)",
                        color: "var(--text-primary)",
                        border: "1px solid var(--border)",
                      }}
                    />
                    <span
                      className="text-xs"
                      style={{ color: "var(--text-muted)" }}
                    >
                      Used to distinguish multiple {meta.displayName} accounts.
                    </span>
                  </div>
                ) : (
                  <button
                    onClick={() => setShowInstanceName(true)}
                    className="text-xs transition-colors hover:underline"
                    style={{ color: "var(--text-muted)" }}
                  >
                    Running multiple accounts? Give this one a name
                  </button>
                )}
              </div>
            </div>
          )}

          {/* STEP 3: Connect to Extension */}
          {step === 3 && !executing && (
            <div className="flex flex-col gap-4">
              <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
                Choose which AI extension instance should handle messages from{" "}
                {meta?.displayName}.
              </p>

              <div className="flex flex-col gap-2">
                {/* Option: Any */}
                <label
                  className="flex items-start gap-3 p-3 rounded-lg cursor-pointer transition-colors"
                  style={{
                    backgroundColor:
                      bindingMode === "all"
                        ? "color-mix(in srgb, var(--accent) 10%, transparent)"
                        : "var(--bg-secondary)",
                    border: `1px solid ${bindingMode === "all" ? "var(--accent)" : "var(--border)"}`,
                  }}
                >
                  <input
                    type="radio"
                    name="bindingMode"
                    value="all"
                    checked={bindingMode === "all"}
                    onChange={() => setBindingMode("all")}
                    className="mt-0.5"
                  />
                  <div>
                    <div
                      className="text-sm font-medium"
                      style={{ color: "var(--text-primary)" }}
                    >
                      Any extension can use this channel
                    </div>
                    <div
                      className="text-xs mt-0.5"
                      style={{ color: "var(--text-muted)" }}
                    >
                      No restrictions — all extensions can send and receive
                      through this channel.
                    </div>
                  </div>
                </label>

                {/* Option: Specific */}
                <label
                  className="flex items-start gap-3 p-3 rounded-lg cursor-pointer transition-colors"
                  style={{
                    backgroundColor:
                      bindingMode === "specific"
                        ? "color-mix(in srgb, var(--accent) 10%, transparent)"
                        : "var(--bg-secondary)",
                    border: `1px solid ${bindingMode === "specific" ? "var(--accent)" : "var(--border)"}`,
                  }}
                >
                  <input
                    type="radio"
                    name="bindingMode"
                    value="specific"
                    checked={bindingMode === "specific"}
                    onChange={() => setBindingMode("specific")}
                    className="mt-0.5"
                  />
                  <div className="flex-1">
                    <div
                      className="text-sm font-medium"
                      style={{ color: "var(--text-primary)" }}
                    >
                      Only a specific extension instance
                    </div>
                    <div
                      className="text-xs mt-0.5"
                      style={{ color: "var(--text-muted)" }}
                    >
                      Only this instance can send messages through this
                      channel.
                    </div>

                    {bindingMode === "specific" && (
                      <div className="mt-2">
                        {extensions.length === 0 ? (
                          <div
                            className="text-xs italic"
                            style={{ color: "var(--text-muted)" }}
                          >
                            No extensions installed. Install one from the
                            Extensions page first.
                          </div>
                        ) : (
                          <select
                            value={selectedExtension}
                            onChange={(e) =>
                              setSelectedExtension(e.target.value)
                            }
                            className="w-full px-3 py-2 rounded text-sm"
                            style={{
                              backgroundColor: "var(--bg-primary)",
                              color: "var(--text-primary)",
                              border: "1px solid var(--border)",
                            }}
                          >
                            <option value="">Select an instance...</option>
                            {instances.length > 0
                              ? extensions.map((ext) => {
                                  const extInstances = instances.filter(
                                    (i) => i.extension_id === ext.id,
                                  );
                                  if (extInstances.length === 0) return null;
                                  return (
                                    <optgroup key={ext.id} label={ext.name}>
                                      {extInstances.map((inst) => {
                                        const label =
                                          inst.instance_name === "default"
                                            ? ext.name
                                            : `${ext.name} (${inst.display_name || inst.instance_name})`;
                                        return (
                                          <option
                                            key={inst.instance_id}
                                            value={inst.instance_id}
                                          >
                                            {label}
                                          </option>
                                        );
                                      })}
                                    </optgroup>
                                  );
                                })
                              : extensions.map((ext) => (
                                  <option key={ext.id} value={ext.id}>
                                    {ext.name}
                                  </option>
                                ))}
                          </select>
                        )}
                      </div>
                    )}
                  </div>
                </label>
              </div>
            </div>
          )}

          {/* Execution progress */}
          {executing && (
            <div className="flex flex-col items-center justify-center py-8 gap-4">
              <Loader2
                size={32}
                className="animate-spin"
                style={{ color: "var(--accent)" }}
              />
              <p className="text-sm" style={{ color: "var(--text-primary)" }}>
                {executionStep ?? "Setting up..."}
              </p>
            </div>
          )}

          {/* Error */}
          {error && (
            <div
              className="rounded-lg p-3 mt-2 flex items-start gap-2 text-xs"
              style={{
                backgroundColor: "color-mix(in srgb, var(--error) 15%, transparent)",
                color: "var(--error)",
              }}
            >
              <AlertCircle size={14} className="flex-shrink-0 mt-0.5" />
              <div>
                <div className="font-medium">Setup failed</div>
                <div className="mt-1">{error}</div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        {!executing && (
          <div
            className="flex items-center justify-between p-4"
            style={{ borderTop: "1px solid var(--border)" }}
          >
            <button
              onClick={onClose}
              className="px-4 py-2 rounded text-sm font-medium transition-colors"
              style={{
                backgroundColor: "var(--bg-secondary)",
                color: "var(--text-secondary)",
              }}
            >
              Cancel
            </button>

            {step === 2 && (
              <button
                onClick={() => setStep(3)}
                disabled={!canProceedStep2}
                className="flex items-center gap-2 px-4 py-2 rounded text-sm font-medium transition-colors disabled:opacity-50"
                style={{ backgroundColor: "var(--accent)", color: "white" }}
              >
                Next
                <ArrowRight size={14} />
              </button>
            )}

            {step === 3 && (
              <button
                onClick={handleFinish}
                disabled={
                  bindingMode === "specific" &&
                  !selectedExtension &&
                  extensions.length > 0
                }
                className="flex items-center gap-2 px-4 py-2 rounded text-sm font-medium transition-colors disabled:opacity-50"
                style={{ backgroundColor: "var(--accent)", color: "white" }}
              >
                <Check size={14} />
                Finish Setup
              </button>
            )}
          </div>
        )}
      </div>
    </>
  );
}
