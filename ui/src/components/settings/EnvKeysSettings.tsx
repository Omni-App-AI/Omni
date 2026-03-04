import { useState, useEffect, useCallback } from "react";
import {
  envVarsList,
  envVarsSet,
  envVarsDelete,
  type EnvVarEntryDto,
} from "../../lib/tauri-commands";

// Well-known environment variables with descriptions
const WELL_KNOWN_KEYS = [
  {
    key: "BRAVE_API_KEY",
    label: "Brave Search API Key",
    description:
      "Web search via Brave Search API. Free tier: 2,000 searches/month.",
    link: "https://brave.com/search/api/",
  },
  {
    key: "TAVILY_API_KEY",
    label: "Tavily Search API Key",
    description:
      "AI-optimized web search with clean, structured results. Free tier: 1,000 searches/month.",
    link: "https://tavily.com/",
  },
  {
    key: "SERPER_API_KEY",
    label: "Serper.dev API Key",
    description:
      "Google Search results via API. Free tier: 2,500 searches/month.",
    link: "https://serper.dev/",
  },
  {
    key: "SEARXNG_URL",
    label: "SearXNG Instance URL",
    description:
      "Self-hosted meta-search engine. No API key needed — just provide the instance URL (e.g. https://searx.be).",
    link: "https://searx.space/",
  },
  {
    key: "OMNI_MEMORY_DIR",
    label: "Memory Storage Path",
    description:
      "Custom directory for the agent's persistent memory. Leave empty for default.",
  },
];

export function EnvKeysSettings() {
  const [envVars, setEnvVars] = useState<EnvVarEntryDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddForm, setShowAddForm] = useState(false);

  const loadVars = useCallback(async () => {
    try {
      const vars = await envVarsList();
      setEnvVars(vars);
    } catch (err) {
      console.error("Failed to load env vars:", err);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    loadVars();
  }, [loadVars]);

  const isKeyConfigured = (key: string) =>
    envVars.some((v) => v.key === key);

  const getMaskedValue = (key: string) =>
    envVars.find((v) => v.key === key)?.masked_value ?? "";

  // Custom keys = configured keys that aren't in WELL_KNOWN_KEYS
  const wellKnownKeySet = new Set(WELL_KNOWN_KEYS.map((k) => k.key));
  const customVars = envVars.filter((v) => !wellKnownKeySet.has(v.key));

  return (
    <div className="space-y-6">
      {/* Well-Known Keys */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <h3 className="text-sm font-medium text-[var(--text-primary)] mb-1">
          API Keys & Service Configuration
        </h3>
        <p className="text-xs text-[var(--text-muted)] mb-4">
          Environment variables used by Omni's built-in tools. Values are
          injected at startup and take effect immediately when set.
        </p>

        <div className="space-y-3">
          {WELL_KNOWN_KEYS.map((wk) => (
            <WellKnownKeyCard
              key={wk.key}
              envKey={wk.key}
              label={wk.label}
              description={wk.description}
              link={wk.link}
              configured={isKeyConfigured(wk.key)}
              maskedValue={getMaskedValue(wk.key)}
              onSave={async (value) => {
                await envVarsSet(wk.key, value);
                await loadVars();
              }}
              onDelete={async () => {
                await envVarsDelete(wk.key);
                await loadVars();
              }}
            />
          ))}
        </div>
      </div>

      {/* Custom Keys */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h3 className="text-sm font-medium text-[var(--text-primary)]">
              Custom Environment Variables
            </h3>
            <p className="text-xs text-[var(--text-muted)] mt-0.5">
              Add arbitrary key-value pairs available to all tools and
              extensions.
            </p>
          </div>
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className="px-3 py-1.5 text-xs font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
          >
            {showAddForm ? "Cancel" : "+ Add Variable"}
          </button>
        </div>

        {showAddForm && (
          <AddEnvVarForm
            existingKeys={envVars.map((v) => v.key)}
            onAdd={async (key, value) => {
              await envVarsSet(key, value);
              await loadVars();
              setShowAddForm(false);
            }}
            onCancel={() => setShowAddForm(false)}
          />
        )}

        {loading && customVars.length === 0 ? (
          <p className="text-sm text-[var(--text-muted)]">Loading...</p>
        ) : customVars.length === 0 && !showAddForm ? (
          <p className="text-sm text-[var(--text-muted)]">
            No custom variables configured.
          </p>
        ) : (
          <div className="space-y-2 mt-3">
            {customVars.map((v) => (
              <CustomVarRow
                key={v.key}
                envKey={v.key}
                maskedValue={v.masked_value}
                onUpdate={async (value) => {
                  await envVarsSet(v.key, value);
                  await loadVars();
                }}
                onDelete={async () => {
                  await envVarsDelete(v.key);
                  await loadVars();
                }}
              />
            ))}
          </div>
        )}
      </div>

      {/* Info */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <h3 className="text-sm font-medium text-[var(--text-primary)] mb-2">
          How It Works
        </h3>
        <ul className="text-xs text-[var(--text-muted)] space-y-1.5 list-disc list-inside">
          <li>
            Variables are stored in your{" "}
            <code className="font-mono text-[var(--text-secondary)]">
              config.toml
            </code>{" "}
            under the{" "}
            <code className="font-mono text-[var(--text-secondary)]">
              [env_vars]
            </code>{" "}
            section.
          </li>
          <li>
            They are injected into the process at startup and immediately when
            set via this UI.
          </li>
          <li>
            Tools like{" "}
            <code className="font-mono text-[var(--text-secondary)]">
              web_search
            </code>{" "}
            automatically check for keys like{" "}
            <code className="font-mono text-[var(--text-secondary)]">
              BRAVE_API_KEY
            </code>
            .
          </li>
        </ul>
      </div>
    </div>
  );
}

// ─── Well-Known Key Card ──────────────────────────────────────────────

function WellKnownKeyCard({
  envKey,
  label,
  description,
  link,
  configured,
  maskedValue,
  onSave,
  onDelete,
}: {
  envKey: string;
  label: string;
  description: string;
  link?: string;
  configured: boolean;
  maskedValue: string;
  onSave: (value: string) => Promise<void>;
  onDelete: () => Promise<void>;
}) {
  const [value, setValue] = useState("");
  const [showValue, setShowValue] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);

  const handleSave = async () => {
    if (!value.trim()) return;
    setSaving(true);
    setError("");
    try {
      await onSave(value.trim());
      setValue("");
      setShowValue(false);
    } catch (e) {
      setError(String(e));
    }
    setSaving(false);
  };

  const handleDelete = async () => {
    if (!confirmDelete) {
      setConfirmDelete(true);
      return;
    }
    try {
      await onDelete();
      setConfirmDelete(false);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="p-4 rounded-lg border border-[var(--border)] bg-[var(--bg-primary)]">
      <div className="flex items-start justify-between mb-2">
        <div>
          <div className="flex items-center gap-2">
            <span
              className={`w-2 h-2 rounded-full ${configured ? "bg-green-400" : "bg-yellow-400"}`}
            />
            <span className="text-sm font-medium text-[var(--text-primary)]">
              {label}
            </span>
            <code className="text-[11px] font-mono px-1.5 py-0.5 rounded bg-[var(--bg-secondary)] text-[var(--text-muted)]">
              {envKey}
            </code>
          </div>
          <p className="text-xs text-[var(--text-muted)] mt-1 ml-4">
            {description}
            {link && (
              <>
                {" "}
                <a
                  href={link}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[var(--accent)] hover:underline"
                >
                  Get key →
                </a>
              </>
            )}
          </p>
        </div>
        {configured && (
          <span className="text-[11px] px-2 py-0.5 rounded-full bg-green-500/10 text-green-400 whitespace-nowrap">
            Configured
          </span>
        )}
      </div>

      {configured && (
        <div className="flex items-center gap-2 mb-3 ml-4">
          <span className="text-xs text-[var(--text-muted)]">Current:</span>
          <code className="text-xs font-mono text-[var(--text-secondary)]">
            {maskedValue}
          </code>
        </div>
      )}

      <div className="flex gap-2 ml-4">
        <div className="relative flex-1">
          <input
            type={showValue ? "text" : "password"}
            value={value}
            onChange={(e) => {
              setValue(e.target.value);
              setError("");
            }}
            placeholder={
              configured ? "Enter new value to replace..." : "Enter value..."
            }
            className="w-full px-3 py-2 pr-14 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
          />
          <button
            onClick={() => setShowValue(!showValue)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)]"
          >
            {showValue ? "Hide" : "Show"}
          </button>
        </div>
        <button
          onClick={handleSave}
          disabled={!value.trim() || saving}
          className="px-4 py-2 text-sm font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
        >
          {saving ? "..." : "Set"}
        </button>
        {configured && (
          <button
            onClick={handleDelete}
            className={`px-3 py-2 text-xs font-medium rounded-md transition-colors ${
              confirmDelete
                ? "bg-red-500 text-white"
                : "bg-red-500/10 text-red-400 hover:bg-red-500/20"
            }`}
          >
            {confirmDelete ? "Confirm" : "Delete"}
          </button>
        )}
      </div>

      {error && <p className="text-xs text-red-400 mt-2 ml-4">{error}</p>}
    </div>
  );
}

// ─── Custom Var Row ───────────────────────────────────────────────────

function CustomVarRow({
  envKey,
  maskedValue,
  onUpdate,
  onDelete,
}: {
  envKey: string;
  maskedValue: string;
  onUpdate: (value: string) => Promise<void>;
  onDelete: () => Promise<void>;
}) {
  const [editing, setEditing] = useState(false);
  const [newValue, setNewValue] = useState("");
  const [saving, setSaving] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const handleUpdate = async () => {
    if (!newValue.trim()) return;
    setSaving(true);
    try {
      await onUpdate(newValue.trim());
      setNewValue("");
      setEditing(false);
    } catch {
      /* ignore */
    }
    setSaving(false);
  };

  return (
    <div className="flex items-center gap-3 p-3 rounded-md border border-[var(--border)] bg-[var(--bg-primary)]">
      <span className="w-2 h-2 rounded-full bg-green-400 flex-shrink-0" />
      <code className="text-xs font-mono text-[var(--text-primary)] min-w-[140px]">
        {envKey}
      </code>
      <code className="text-xs font-mono text-[var(--text-muted)] flex-1">
        {maskedValue}
      </code>

      {editing ? (
        <div className="flex gap-1.5">
          <input
            type="password"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="New value..."
            className="w-40 px-2 py-1 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-xs text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
            autoFocus
          />
          <button
            onClick={handleUpdate}
            disabled={!newValue.trim() || saving}
            className="px-2 py-1 text-xs rounded-md bg-[var(--accent)] text-white disabled:opacity-50"
          >
            {saving ? "..." : "Save"}
          </button>
          <button
            onClick={() => {
              setEditing(false);
              setNewValue("");
            }}
            className="px-2 py-1 text-xs rounded-md text-[var(--text-muted)] hover:bg-[var(--bg-hover)]"
          >
            Cancel
          </button>
        </div>
      ) : (
        <div className="flex gap-1.5">
          <button
            onClick={() => setEditing(true)}
            className="px-2 py-1 text-xs rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors"
          >
            Edit
          </button>
          <button
            onClick={async () => {
              if (!confirmDelete) {
                setConfirmDelete(true);
                return;
              }
              await onDelete();
            }}
            className={`px-2 py-1 text-xs rounded-md transition-colors ${
              confirmDelete
                ? "bg-red-500 text-white"
                : "bg-red-500/10 text-red-400 hover:bg-red-500/20"
            }`}
          >
            {confirmDelete ? "Confirm" : "Delete"}
          </button>
        </div>
      )}
    </div>
  );
}

// ─── Add Env Var Form ─────────────────────────────────────────────────

function AddEnvVarForm({
  existingKeys,
  onAdd,
  onCancel,
}: {
  existingKeys: string[];
  onAdd: (key: string, value: string) => Promise<void>;
  onCancel: () => void;
}) {
  const [key, setKey] = useState("");
  const [value, setValue] = useState("");
  const [showValue, setShowValue] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async () => {
    const cleanKey = key.trim().toUpperCase();
    if (!cleanKey) {
      setError("Key is required");
      return;
    }
    if (!/^[A-Z0-9_]+$/.test(cleanKey)) {
      setError("Key must contain only A-Z, 0-9, and underscores");
      return;
    }
    if (!value.trim()) {
      setError("Value is required");
      return;
    }
    if (existingKeys.includes(cleanKey)) {
      setError(`"${cleanKey}" already exists. Edit it instead.`);
      return;
    }
    setSaving(true);
    setError("");
    try {
      await onAdd(cleanKey, value.trim());
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  return (
    <div className="p-4 rounded-lg border border-[var(--accent)]/30 bg-[var(--bg-primary)] space-y-3 mb-3">
      <div>
        <label className="block text-xs text-[var(--text-muted)] mb-1">
          Variable Name
        </label>
        <input
          type="text"
          value={key}
          onChange={(e) => {
            setKey(e.target.value.toUpperCase().replace(/[^A-Z0-9_]/g, ""));
            setError("");
          }}
          placeholder="e.g. MY_API_KEY"
          className="w-full px-3 py-2 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
        />
      </div>
      <div>
        <label className="block text-xs text-[var(--text-muted)] mb-1">
          Value
        </label>
        <div className="relative">
          <input
            type={showValue ? "text" : "password"}
            value={value}
            onChange={(e) => {
              setValue(e.target.value);
              setError("");
            }}
            placeholder="Enter value..."
            className="w-full px-3 py-2 pr-14 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
          />
          <button
            onClick={() => setShowValue(!showValue)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)]"
          >
            {showValue ? "Hide" : "Show"}
          </button>
        </div>
      </div>

      {error && <p className="text-xs text-red-400">{error}</p>}

      <div className="flex gap-2 pt-1">
        <button
          onClick={handleSubmit}
          disabled={saving}
          className="px-4 py-2 text-sm font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
        >
          {saving ? "Saving..." : "Add Variable"}
        </button>
        <button
          onClick={onCancel}
          className="px-4 py-2 text-sm font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}
