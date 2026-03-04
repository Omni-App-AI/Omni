import { useState, useEffect } from "react";
import { useProviderStore } from "../../stores/providerStore";
import type { ProviderDto, ProviderTypeInfoDto } from "../../lib/tauri-commands";

export function ProviderSettings() {
  const {
    providers,
    providerTypes,
    loading,
    selectedProviderId,
    testResult,
    testingProviderId,
    loadAll,
    selectProvider,
    addProvider,
    updateProvider,
    removeProvider,
    setCredential,
    deleteCredential,
    testCredential,
    clearTestResult,
  } = useProviderStore();

  const [showAddForm, setShowAddForm] = useState(false);

  useEffect(() => {
    loadAll();
  }, [loadAll]);

  const selectedProvider =
    providers.find((p) => p.id === selectedProviderId) ?? null;

  return (
    <div className="space-y-6">
      {/* Provider Cards */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-[var(--text-primary)]">
            Providers
          </h3>
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className="px-3 py-1.5 text-xs font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
          >
            {showAddForm ? "Cancel" : "+ Add Provider"}
          </button>
        </div>

        {loading && providers.length === 0 ? (
          <p className="text-sm text-[var(--text-muted)]">
            Loading providers...
          </p>
        ) : providers.length === 0 && !showAddForm ? (
          <p className="text-sm text-[var(--text-muted)]">
            No providers configured. Click &quot;+ Add Provider&quot; to get
            started.
          </p>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {providers.map((p) => (
              <ProviderCard
                key={p.id}
                provider={p}
                selected={selectedProviderId === p.id}
                onSelect={() =>
                  selectProvider(selectedProviderId === p.id ? null : p.id)
                }
              />
            ))}
          </div>
        )}
      </div>

      {/* Add Provider Form */}
      {showAddForm && (
        <AddProviderForm
          providerTypes={providerTypes}
          existingIds={providers.map((p) => p.id)}
          onAdd={async (id, type, model, endpoint) => {
            await addProvider(id, type, model, endpoint);
            setShowAddForm(false);
          }}
          onCancel={() => setShowAddForm(false)}
        />
      )}

      {/* Provider Detail Panel */}
      {selectedProvider && (
        <ProviderDetailPanel
          provider={selectedProvider}
          testResult={
            testResult?.providerId === selectedProvider.id ? testResult : null
          }
          testing={testingProviderId === selectedProvider.id}
          onUpdate={updateProvider}
          onRemove={removeProvider}
          onSetCredential={setCredential}
          onDeleteCredential={deleteCredential}
          onTestCredential={testCredential}
          onClearTestResult={clearTestResult}
        />
      )}
    </div>
  );
}

// ─── Provider Card ────────────────────────────────────────────────────

function ProviderCard({
  provider,
  selected,
  onSelect,
}: {
  provider: ProviderDto;
  selected: boolean;
  onSelect: () => void;
}) {
  const statusColor =
    provider.auth_type === "none"
      ? "bg-gray-400"
      : provider.has_credential
        ? "bg-green-400"
        : "bg-yellow-400";

  const statusText =
    provider.auth_type === "none"
      ? "No auth needed"
      : provider.has_credential
        ? "Key configured"
        : "Key missing";

  return (
    <button
      onClick={onSelect}
      className={`text-left p-4 rounded-lg border transition-colors ${
        selected
          ? "border-[var(--accent)] bg-[var(--accent)]/5"
          : "border-[var(--border)] hover:bg-[var(--bg-hover)]"
      }`}
    >
      <div className="flex items-center justify-between mb-1">
        <span className="text-sm font-medium text-[var(--text-primary)]">
          {provider.display_name}
        </span>
        {!provider.enabled && (
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--border)] text-[var(--text-muted)]">
            Disabled
          </span>
        )}
      </div>
      <div className="text-xs text-[var(--text-muted)] mb-2">
        {provider.id !== provider.provider_type && (
          <span className="mr-2">{provider.id}</span>
        )}
        {provider.default_model && (
          <span className="text-[var(--text-secondary)]">
            {provider.default_model}
          </span>
        )}
      </div>
      <div className="flex items-center gap-1.5">
        <span className={`w-2 h-2 rounded-full ${statusColor}`} />
        <span className="text-[11px] text-[var(--text-muted)]">
          {statusText}
        </span>
      </div>
    </button>
  );
}

// ─── Add Provider Form ────────────────────────────────────────────────

function AddProviderForm({
  providerTypes,
  existingIds,
  onAdd,
  onCancel,
}: {
  providerTypes: ProviderTypeInfoDto[];
  existingIds: string[];
  onAdd: (
    id: string,
    type: string,
    model?: string,
    endpoint?: string,
  ) => Promise<void>;
  onCancel: () => void;
}) {
  const [selectedType, setSelectedType] = useState("");
  const [customId, setCustomId] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const typeInfo = providerTypes.find((t) => t.provider_type === selectedType);

  const effectiveId = customId || selectedType;

  const handleSubmit = async () => {
    if (!selectedType) {
      setError("Select a provider type");
      return;
    }
    if (existingIds.includes(effectiveId)) {
      setError(`Provider '${effectiveId}' already exists`);
      return;
    }
    setSaving(true);
    setError("");
    try {
      await onAdd(
        effectiveId,
        selectedType,
        undefined,
        typeInfo?.default_endpoint ?? undefined,
      );
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  return (
    <div className="rounded-lg border border-[var(--accent)]/30 bg-[var(--bg-secondary)] p-6">
      <h3 className="text-sm font-medium text-[var(--text-primary)] mb-4">
        Add Provider
      </h3>

      <div className="space-y-4">
        {/* Provider Type Selection */}
        <div>
          <label className="block text-xs text-[var(--text-muted)] mb-1.5">
            Provider Type
          </label>
          <div className="grid grid-cols-2 sm:grid-cols-3 gap-2">
            {providerTypes.map((t) => (
              <button
                key={t.provider_type}
                onClick={() => {
                  setSelectedType(t.provider_type);
                  setCustomId("");
                  setError("");
                }}
                className={`p-3 rounded-md border text-left transition-colors ${
                  selectedType === t.provider_type
                    ? "border-[var(--accent)] bg-[var(--accent)]/10"
                    : "border-[var(--border)] hover:bg-[var(--bg-hover)]"
                }`}
              >
                <div className="text-sm font-medium text-[var(--text-primary)]">
                  {t.display_name}
                </div>
                <div className="text-[11px] text-[var(--text-muted)] mt-0.5">
                  {t.description}
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Custom ID */}
        {selectedType && (
          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1.5">
              Provider ID
              <span className="ml-1 text-[var(--text-muted)]">
                (used as config key)
              </span>
            </label>
            <input
              type="text"
              value={customId}
              onChange={(e) => {
                setCustomId(e.target.value.replace(/[^a-z0-9-]/g, ""));
                setError("");
              }}
              placeholder={selectedType}
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
            />
          </div>
        )}

        {error && (
          <p className="text-xs text-red-400">{error}</p>
        )}

        <div className="flex gap-2 pt-1">
          <button
            onClick={handleSubmit}
            disabled={!selectedType || saving}
            className="px-4 py-2 text-sm font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
          >
            {saving ? "Adding..." : "Add Provider"}
          </button>
          <button
            onClick={onCancel}
            className="px-4 py-2 text-sm font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}

// ─── Provider Detail Panel ────────────────────────────────────────────

function ProviderDetailPanel({
  provider,
  testResult,
  testing,
  onUpdate,
  onRemove,
  onSetCredential,
  onDeleteCredential,
  onTestCredential,
  onClearTestResult,
}: {
  provider: ProviderDto;
  testResult: { message: string; success: boolean } | null;
  testing: boolean;
  onUpdate: (
    id: string,
    defaultModel?: string,
    endpoint?: string,
    maxTokens?: number,
    temperature?: number,
    enabled?: boolean,
  ) => Promise<void>;
  onRemove: (id: string) => Promise<void>;
  onSetCredential: (
    providerId: string,
    credentialType: string,
    apiKey?: string,
    awsAccessKeyId?: string,
    awsSecretAccessKey?: string,
    awsSessionToken?: string,
    awsRegion?: string,
  ) => Promise<void>;
  onDeleteCredential: (providerId: string) => Promise<void>;
  onTestCredential: (providerId: string) => Promise<void>;
  onClearTestResult: () => void;
}) {
  const [model, setModel] = useState(provider.default_model ?? "");
  const [endpoint, setEndpoint] = useState(provider.endpoint ?? "");
  const [temperature, setTemperature] = useState(provider.temperature ?? 0.7);
  const [maxTokens, setMaxTokens] = useState(
    provider.max_tokens?.toString() ?? "",
  );
  const [enabled, setEnabled] = useState(provider.enabled);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState("");
  const [confirmRemove, setConfirmRemove] = useState(false);

  // Reset local state when selected provider changes
  useEffect(() => {
    setModel(provider.default_model ?? "");
    setEndpoint(provider.endpoint ?? "");
    setTemperature(provider.temperature ?? 0.7);
    setMaxTokens(provider.max_tokens?.toString() ?? "");
    setEnabled(provider.enabled);
    setSaveMsg("");
    setConfirmRemove(false);
  }, [provider.id, provider.default_model, provider.endpoint, provider.temperature, provider.max_tokens, provider.enabled]);

  const handleSave = async () => {
    setSaving(true);
    setSaveMsg("");
    try {
      const parsedTokens = maxTokens ? parseInt(maxTokens, 10) : undefined;
      await onUpdate(
        provider.id,
        model,
        endpoint,
        parsedTokens,
        temperature,
        enabled,
      );
      setSaveMsg("Saved");
      setTimeout(() => setSaveMsg(""), 2000);
    } catch (e) {
      setSaveMsg(String(e));
    }
    setSaving(false);
  };

  const handleRemove = async () => {
    if (!confirmRemove) {
      setConfirmRemove(true);
      return;
    }
    await onRemove(provider.id);
  };

  return (
    <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium text-[var(--text-primary)]">
            {provider.display_name}
          </h3>
          <p className="text-xs text-[var(--text-muted)] mt-0.5">
            {provider.id} &middot; {provider.provider_type}
          </p>
        </div>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 cursor-pointer">
            <span className="text-xs text-[var(--text-muted)]">Enabled</span>
            <button
              onClick={() => setEnabled(!enabled)}
              className={`relative w-9 h-5 rounded-full transition-colors ${
                enabled ? "bg-[var(--accent)]" : "bg-[var(--border)]"
              }`}
            >
              <span
                className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                  enabled ? "translate-x-4" : ""
                }`}
              />
            </button>
          </label>
        </div>
      </div>

      {/* Credential Section */}
      {provider.auth_type === "api_key" && (
        <ApiKeySection
          provider={provider}
          testResult={testResult}
          testing={testing}
          onSetCredential={onSetCredential}
          onDeleteCredential={onDeleteCredential}
          onTestCredential={onTestCredential}
          onClearTestResult={onClearTestResult}
        />
      )}
      {provider.auth_type === "aws" && (
        <AwsCredentialSection
          provider={provider}
          testResult={testResult}
          testing={testing}
          onSetCredential={onSetCredential}
          onDeleteCredential={onDeleteCredential}
          onTestCredential={onTestCredential}
          onClearTestResult={onClearTestResult}
        />
      )}
      {provider.auth_type === "none" && (
        <div className="p-4 rounded-md bg-[var(--bg-primary)] border border-[var(--border)]">
          <p className="text-sm text-[var(--text-muted)]">
            No authentication required. Ensure the service is running locally.
          </p>
        </div>
      )}

      {/* Configuration Section */}
      <div>
        <h4 className="text-xs font-medium text-[var(--text-muted)] uppercase tracking-wider mb-3">
          Configuration
        </h4>
        <div className="space-y-3">
          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Default Model
            </label>
            <input
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder="e.g. gpt-5.2, claude-opus-4-6, gemini-3.1-pro-preview"
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
            />
          </div>

          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              API Endpoint
            </label>
            <input
              type="text"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="Leave empty for default"
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
            />
          </div>

          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Temperature
              <span className="ml-2 text-[var(--text-primary)] font-medium">
                {temperature.toFixed(1)}
              </span>
            </label>
            <div className="flex items-center gap-3">
              <span className="text-xs text-[var(--text-muted)] w-4">0</span>
              <input
                type="range"
                min={0}
                max={2}
                step={0.1}
                value={temperature}
                onChange={(e) => setTemperature(parseFloat(e.target.value))}
                className="flex-1 h-1.5 rounded-full appearance-none bg-[var(--border)] accent-[var(--accent)] cursor-pointer"
              />
              <span className="text-xs text-[var(--text-muted)] w-4">2</span>
            </div>
          </div>

          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Max Tokens
            </label>
            <input
              type="number"
              value={maxTokens}
              onChange={(e) => setMaxTokens(e.target.value)}
              placeholder="Leave empty for model default"
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
            />
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center justify-between pt-2 border-t border-[var(--border)]">
        <div className="flex items-center gap-2">
          <button
            onClick={handleSave}
            disabled={saving}
            className="px-4 py-2 text-sm font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
          >
            {saving ? "Saving..." : "Save Changes"}
          </button>
          {saveMsg && (
            <span
              className={`text-xs ${
                saveMsg === "Saved"
                  ? "text-green-400"
                  : "text-red-400"
              }`}
            >
              {saveMsg}
            </span>
          )}
        </div>
        <button
          onClick={handleRemove}
          className={`px-4 py-2 text-sm font-medium rounded-md transition-colors ${
            confirmRemove
              ? "bg-red-500 text-white hover:bg-red-600"
              : "bg-red-500/10 text-red-400 hover:bg-red-500/20"
          }`}
        >
          {confirmRemove ? "Confirm Remove" : "Remove Provider"}
        </button>
      </div>
    </div>
  );
}

// ─── API Key Section ──────────────────────────────────────────────────

function ApiKeySection({
  provider,
  testResult,
  testing,
  onSetCredential,
  onDeleteCredential,
  onTestCredential,
  onClearTestResult,
}: {
  provider: ProviderDto;
  testResult: { message: string; success: boolean } | null;
  testing: boolean;
  onSetCredential: (
    providerId: string,
    credentialType: string,
    apiKey?: string,
  ) => Promise<void>;
  onDeleteCredential: (providerId: string) => Promise<void>;
  onTestCredential: (providerId: string) => Promise<void>;
  onClearTestResult: () => void;
}) {
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const handleSave = async () => {
    if (!apiKey.trim()) return;
    setSaving(true);
    setError("");
    onClearTestResult();
    try {
      await onSetCredential(provider.id, "api_key", apiKey.trim());
      setApiKey("");
      setShowKey(false);
    } catch (e) {
      setError(String(e));
    }
    setSaving(false);
  };

  const handleDelete = async () => {
    onClearTestResult();
    await onDeleteCredential(provider.id);
  };

  return (
    <div>
      <h4 className="text-xs font-medium text-[var(--text-muted)] uppercase tracking-wider mb-3">
        API Key
      </h4>
      <div className="p-4 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] space-y-3">
        {provider.has_credential ? (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-green-400" />
              <span className="text-sm text-[var(--text-primary)]">
                API key configured
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => onTestCredential(provider.id)}
                disabled={testing}
                className="px-3 py-1.5 text-xs font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors disabled:opacity-50"
              >
                {testing ? "Testing..." : "Test"}
              </button>
              <button
                onClick={handleDelete}
                className="px-3 py-1.5 text-xs font-medium rounded-md bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
              >
                Delete
              </button>
            </div>
          </div>
        ) : (
          <div className="flex items-center gap-2 mb-2">
            <span className="w-2 h-2 rounded-full bg-yellow-400" />
            <span className="text-sm text-[var(--text-muted)]">
              No API key configured
            </span>
          </div>
        )}

        {/* Key input -- always available for setting/replacing */}
        <div className="flex gap-2">
          <div className="relative flex-1">
            <input
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => {
                setApiKey(e.target.value);
                setError("");
              }}
              placeholder={
                provider.has_credential
                  ? "Enter new key to replace..."
                  : provider.env_var_hint
                    ? `Paste key (or set ${provider.env_var_hint})`
                    : "Paste your API key"
              }
              className="w-full px-3 py-2 pr-16 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
            />
            <button
              onClick={() => setShowKey(!showKey)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)]"
            >
              {showKey ? "Hide" : "Show"}
            </button>
          </div>
          <button
            onClick={handleSave}
            disabled={!apiKey.trim() || saving}
            className="px-4 py-2 text-sm font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
          >
            {saving ? "..." : "Set Key"}
          </button>
        </div>

        {error && <p className="text-xs text-red-400">{error}</p>}

        {testResult && (
          <div
            className={`text-xs p-2 rounded-md ${
              testResult.success
                ? "bg-green-500/10 text-green-400"
                : "bg-red-500/10 text-red-400"
            }`}
          >
            {testResult.message}
          </div>
        )}

        {provider.env_var_hint && !provider.has_credential && (
          <p className="text-[11px] text-[var(--text-muted)]">
            Alternatively, set the <code className="font-mono text-[var(--text-secondary)]">{provider.env_var_hint}</code> environment
            variable.
          </p>
        )}
      </div>
    </div>
  );
}

// ─── AWS Credential Section ───────────────────────────────────────────

function AwsCredentialSection({
  provider,
  testResult,
  testing,
  onSetCredential,
  onDeleteCredential,
  onTestCredential,
  onClearTestResult,
}: {
  provider: ProviderDto;
  testResult: { message: string; success: boolean } | null;
  testing: boolean;
  onSetCredential: (
    providerId: string,
    credentialType: string,
    apiKey?: string,
    awsAccessKeyId?: string,
    awsSecretAccessKey?: string,
    awsSessionToken?: string,
    awsRegion?: string,
  ) => Promise<void>;
  onDeleteCredential: (providerId: string) => Promise<void>;
  onTestCredential: (providerId: string) => Promise<void>;
  onClearTestResult: () => void;
}) {
  const [accessKeyId, setAccessKeyId] = useState("");
  const [secretAccessKey, setSecretAccessKey] = useState("");
  const [sessionToken, setSessionToken] = useState("");
  const [region, setRegion] = useState("us-east-1");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const handleSave = async () => {
    if (!accessKeyId.trim() || !secretAccessKey.trim()) {
      setError("Access Key ID and Secret Access Key are required");
      return;
    }
    setSaving(true);
    setError("");
    onClearTestResult();
    try {
      await onSetCredential(
        provider.id,
        "aws",
        undefined,
        accessKeyId.trim(),
        secretAccessKey.trim(),
        sessionToken.trim() || undefined,
        region.trim(),
      );
      setAccessKeyId("");
      setSecretAccessKey("");
      setSessionToken("");
    } catch (e) {
      setError(String(e));
    }
    setSaving(false);
  };

  const handleDelete = async () => {
    onClearTestResult();
    await onDeleteCredential(provider.id);
  };

  return (
    <div>
      <h4 className="text-xs font-medium text-[var(--text-muted)] uppercase tracking-wider mb-3">
        AWS Credentials
      </h4>
      <div className="p-4 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] space-y-3">
        {provider.has_credential && (
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-green-400" />
              <span className="text-sm text-[var(--text-primary)]">
                AWS credentials configured
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => onTestCredential(provider.id)}
                disabled={testing}
                className="px-3 py-1.5 text-xs font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors disabled:opacity-50"
              >
                {testing ? "Testing..." : "Test"}
              </button>
              <button
                onClick={handleDelete}
                className="px-3 py-1.5 text-xs font-medium rounded-md bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
              >
                Delete
              </button>
            </div>
          </div>
        )}

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Access Key ID
            </label>
            <input
              type="password"
              value={accessKeyId}
              onChange={(e) => setAccessKeyId(e.target.value)}
              placeholder="AKIA..."
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
            />
          </div>
          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Secret Access Key
            </label>
            <input
              type="password"
              value={secretAccessKey}
              onChange={(e) => setSecretAccessKey(e.target.value)}
              placeholder="Secret key"
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
            />
          </div>
          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Session Token
              <span className="ml-1 text-[var(--text-muted)]">(optional)</span>
            </label>
            <input
              type="password"
              value={sessionToken}
              onChange={(e) => setSessionToken(e.target.value)}
              placeholder="Optional session token"
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
            />
          </div>
          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Region
            </label>
            <input
              type="text"
              value={region}
              onChange={(e) => setRegion(e.target.value)}
              placeholder="us-east-1"
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-secondary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
            />
          </div>
        </div>

        <button
          onClick={handleSave}
          disabled={!accessKeyId.trim() || !secretAccessKey.trim() || saving}
          className="px-4 py-2 text-sm font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
        >
          {saving
            ? "Saving..."
            : provider.has_credential
              ? "Replace Credentials"
              : "Set Credentials"}
        </button>

        {error && <p className="text-xs text-red-400">{error}</p>}

        {testResult && (
          <div
            className={`text-xs p-2 rounded-md ${
              testResult.success
                ? "bg-green-500/10 text-green-400"
                : "bg-red-500/10 text-red-400"
            }`}
          >
            {testResult.message}
          </div>
        )}
      </div>
    </div>
  );
}
