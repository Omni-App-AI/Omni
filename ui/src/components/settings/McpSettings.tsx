import { useState, useEffect } from "react";
import { useMcpStore } from "../../stores/mcpStore";
import type { McpServerDto } from "../../lib/tauri-commands";

export function McpSettings() {
  const {
    servers,
    loading,
    selectedServerName,
    serverTools,
    error,
    actionInProgress,
    loadServers,
    selectServer,
    addServer,
    removeServer,
    updateServer,
    startServer,
    stopServer,
    restartServer,
  } = useMcpStore();

  const [showAddForm, setShowAddForm] = useState(false);

  useEffect(() => {
    loadServers();
  }, [loadServers]);

  const selectedServer =
    servers.find((s) => s.name === selectedServerName) ?? null;

  return (
    <div className="space-y-6">
      {/* Server Cards */}
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-[var(--text-primary)]">
            MCP Servers
          </h3>
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className="px-3 py-1.5 text-xs font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
          >
            {showAddForm ? "Cancel" : "+ Add Server"}
          </button>
        </div>

        {loading && servers.length === 0 ? (
          <p className="text-sm text-[var(--text-muted)]">
            Loading servers...
          </p>
        ) : servers.length === 0 && !showAddForm ? (
          <p className="text-sm text-[var(--text-muted)]">
            No MCP servers configured. Click &quot;+ Add Server&quot; to get
            started.
          </p>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {servers.map((s) => (
              <McpServerCard
                key={s.name}
                server={s}
                selected={selectedServerName === s.name}
                onSelect={() =>
                  selectServer(selectedServerName === s.name ? null : s.name)
                }
              />
            ))}
          </div>
        )}
      </div>

      {/* Add Server Form */}
      {showAddForm && (
        <AddServerForm
          existingNames={servers.map((s) => s.name)}
          onAdd={async (name, command, args, env, workingDir, autoStart, connectNow) => {
            await addServer(name, command, args, env, workingDir, autoStart, connectNow);
            setShowAddForm(false);
          }}
          onCancel={() => setShowAddForm(false)}
        />
      )}

      {/* Server Detail Panel */}
      {selectedServer && (
        <McpServerDetailPanel
          server={selectedServer}
          tools={serverTools}
          actionInProgress={actionInProgress}
          error={error}
          onUpdate={updateServer}
          onRemove={removeServer}
          onStart={startServer}
          onStop={stopServer}
          onRestart={restartServer}
        />
      )}
    </div>
  );
}

// ─── Server Card ──────────────────────────────────────────────────────

function McpServerCard({
  server,
  selected,
  onSelect,
}: {
  server: McpServerDto;
  selected: boolean;
  onSelect: () => void;
}) {
  const isConnected = server.status === "connected";
  const statusColor = isConnected ? "bg-green-400" : "bg-gray-400";
  const statusText = isConnected
    ? `${server.tool_count} tool${server.tool_count !== 1 ? "s" : ""}`
    : "Disconnected";

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
          {server.name}
        </span>
        {server.auto_start && (
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--accent)]/10 text-[var(--accent)]">
            Auto
          </span>
        )}
      </div>
      <div className="text-xs text-[var(--text-muted)] mb-2 font-mono truncate">
        {server.command} {server.args.join(" ")}
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

// ─── Add Server Form ──────────────────────────────────────────────────

function AddServerForm({
  existingNames,
  onAdd,
  onCancel,
}: {
  existingNames: string[];
  onAdd: (
    name: string,
    command: string,
    args: string[],
    env: Record<string, string>,
    workingDir?: string,
    autoStart?: boolean,
    connectNow?: boolean,
  ) => Promise<void>;
  onCancel: () => void;
}) {
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [argsText, setArgsText] = useState("");
  const [envRows, setEnvRows] = useState<{ key: string; value: string }[]>([]);
  const [workingDir, setWorkingDir] = useState("");
  const [autoStart, setAutoStart] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (connectNow: boolean) => {
    if (!name.trim()) {
      setError("Server name is required");
      return;
    }
    if (!/^[a-z0-9-]+$/.test(name)) {
      setError("Name must be lowercase alphanumeric with hyphens only");
      return;
    }
    if (existingNames.includes(name)) {
      setError(`Server '${name}' already exists`);
      return;
    }
    if (!command.trim()) {
      setError("Command is required");
      return;
    }

    const args = argsText
      .split("\n")
      .map((a) => a.trim())
      .filter((a) => a.length > 0);

    const env: Record<string, string> = {};
    for (const row of envRows) {
      if (row.key.trim()) {
        env[row.key.trim()] = row.value;
      }
    }

    setSaving(true);
    setError("");
    try {
      await onAdd(
        name,
        command.trim(),
        args,
        env,
        workingDir.trim() || undefined,
        autoStart,
        connectNow,
      );
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  return (
    <div className="rounded-lg border border-[var(--accent)]/30 bg-[var(--bg-secondary)] p-6">
      <h3 className="text-sm font-medium text-[var(--text-primary)] mb-4">
        Add MCP Server
      </h3>

      <div className="space-y-4">
        {/* Name */}
        <div>
          <label className="block text-xs text-[var(--text-muted)] mb-1.5">
            Server Name
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => {
              setName(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, ""));
              setError("");
            }}
            placeholder="e.g. filesystem, github, postgres"
            className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
          />
        </div>

        {/* Command */}
        <div>
          <label className="block text-xs text-[var(--text-muted)] mb-1.5">
            Command
          </label>
          <input
            type="text"
            value={command}
            onChange={(e) => {
              setCommand(e.target.value);
              setError("");
            }}
            placeholder="e.g. npx, node, python"
            className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
          />
        </div>

        {/* Arguments */}
        <div>
          <label className="block text-xs text-[var(--text-muted)] mb-1.5">
            Arguments
            <span className="ml-1 text-[var(--text-muted)]">
              (one per line)
            </span>
          </label>
          <textarea
            value={argsText}
            onChange={(e) => setArgsText(e.target.value)}
            placeholder={"-y\n@modelcontextprotocol/server-filesystem\nC:/path/to/workspace"}
            rows={3}
            className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono resize-y"
          />
        </div>

        {/* Environment Variables */}
        <div>
          <label className="block text-xs text-[var(--text-muted)] mb-1.5">
            Environment Variables
          </label>
          {envRows.map((row, i) => (
            <div key={i} className="flex gap-2 mb-2">
              <input
                type="text"
                value={row.key}
                onChange={(e) => {
                  const updated = [...envRows];
                  updated[i] = { ...row, key: e.target.value };
                  setEnvRows(updated);
                }}
                placeholder="KEY"
                className="w-1/3 px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
              />
              <input
                type="text"
                value={row.value}
                onChange={(e) => {
                  const updated = [...envRows];
                  updated[i] = { ...row, value: e.target.value };
                  setEnvRows(updated);
                }}
                placeholder="value"
                className="flex-1 px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
              />
              <button
                onClick={() => setEnvRows(envRows.filter((_, j) => j !== i))}
                className="px-2 py-2 text-xs rounded-md bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
              >
                -
              </button>
            </div>
          ))}
          <button
            onClick={() => setEnvRows([...envRows, { key: "", value: "" }])}
            className="px-3 py-1.5 text-xs font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors"
          >
            + Add Variable
          </button>
        </div>

        {/* Working Directory */}
        <div>
          <label className="block text-xs text-[var(--text-muted)] mb-1.5">
            Working Directory
            <span className="ml-1 text-[var(--text-muted)]">(optional)</span>
          </label>
          <input
            type="text"
            value={workingDir}
            onChange={(e) => setWorkingDir(e.target.value)}
            placeholder="Leave empty for default"
            className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
          />
        </div>

        {/* Auto-start */}
        <label className="flex items-center gap-2 cursor-pointer">
          <button
            onClick={() => setAutoStart(!autoStart)}
            className={`relative w-9 h-5 rounded-full transition-colors ${
              autoStart ? "bg-[var(--accent)]" : "bg-[var(--border)]"
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                autoStart ? "translate-x-4" : ""
              }`}
            />
          </button>
          <span className="text-xs text-[var(--text-secondary)]">
            Auto-start when Omni launches
          </span>
        </label>

        {error && <p className="text-xs text-red-400">{error}</p>}

        <div className="flex gap-2 pt-1">
          <button
            onClick={() => handleSubmit(true)}
            disabled={saving}
            className="px-4 py-2 text-sm font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
          >
            {saving ? "Adding..." : "Add & Connect"}
          </button>
          <button
            onClick={() => handleSubmit(false)}
            disabled={saving}
            className="px-4 py-2 text-sm font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors disabled:opacity-50"
          >
            Add Only
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

// ─── Server Detail Panel ──────────────────────────────────────────────

function McpServerDetailPanel({
  server,
  tools,
  actionInProgress,
  error,
  onUpdate,
  onRemove,
  onStart,
  onStop,
  onRestart,
}: {
  server: McpServerDto;
  tools: { name: string; description: string | null }[];
  actionInProgress: string | null;
  error: string | null;
  onUpdate: (
    name: string,
    command?: string,
    args?: string[],
    env?: Record<string, string>,
    workingDir?: string,
    autoStart?: boolean,
  ) => Promise<void>;
  onRemove: (name: string) => Promise<void>;
  onStart: (name: string) => Promise<void>;
  onStop: (name: string) => Promise<void>;
  onRestart: (name: string) => Promise<void>;
}) {
  const [command, setCommand] = useState(server.command);
  const [argsText, setArgsText] = useState(server.args.join("\n"));
  const [envRows, setEnvRows] = useState(
    Object.entries(server.env).map(([key, value]) => ({ key, value })),
  );
  const [workingDir, setWorkingDir] = useState(server.working_dir ?? "");
  const [autoStart, setAutoStart] = useState(server.auto_start);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState("");
  const [confirmRemove, setConfirmRemove] = useState(false);

  const isConnected = server.status === "connected";

  // Reset local state when selected server changes
  useEffect(() => {
    setCommand(server.command);
    setArgsText(server.args.join("\n"));
    setEnvRows(
      Object.entries(server.env).map(([key, value]) => ({ key, value })),
    );
    setWorkingDir(server.working_dir ?? "");
    setAutoStart(server.auto_start);
    setSaveMsg("");
    setConfirmRemove(false);
  }, [server.name, server.command, server.args, server.env, server.working_dir, server.auto_start]);

  const handleSave = async () => {
    setSaving(true);
    setSaveMsg("");
    try {
      const args = argsText
        .split("\n")
        .map((a) => a.trim())
        .filter((a) => a.length > 0);

      const env: Record<string, string> = {};
      for (const row of envRows) {
        if (row.key.trim()) {
          env[row.key.trim()] = row.value;
        }
      }

      await onUpdate(
        server.name,
        command.trim(),
        args,
        env,
        workingDir.trim() || undefined,
        autoStart,
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
    await onRemove(server.name);
  };

  return (
    <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium text-[var(--text-primary)]">
            {server.name}
          </h3>
          <div className="flex items-center gap-2 mt-1">
            <span
              className={`w-2 h-2 rounded-full ${
                isConnected ? "bg-green-400" : "bg-gray-400"
              }`}
            />
            <span className="text-xs text-[var(--text-muted)]">
              {isConnected
                ? `Connected - ${server.tool_count} tool${server.tool_count !== 1 ? "s" : ""}`
                : "Disconnected"}
            </span>
          </div>
        </div>
        <label className="flex items-center gap-2 cursor-pointer">
          <span className="text-xs text-[var(--text-muted)]">Auto-start</span>
          <button
            onClick={() => setAutoStart(!autoStart)}
            className={`relative w-9 h-5 rounded-full transition-colors ${
              autoStart ? "bg-[var(--accent)]" : "bg-[var(--border)]"
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform ${
                autoStart ? "translate-x-4" : ""
              }`}
            />
          </button>
        </label>
      </div>

      {/* Configuration */}
      <div>
        <h4 className="text-xs font-medium text-[var(--text-muted)] uppercase tracking-wider mb-3">
          Configuration
        </h4>
        <div className="space-y-3">
          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Command
            </label>
            <input
              type="text"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
            />
          </div>

          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Arguments (one per line)
            </label>
            <textarea
              value={argsText}
              onChange={(e) => setArgsText(e.target.value)}
              rows={3}
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono resize-y"
            />
          </div>

          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Environment Variables
            </label>
            {envRows.map((row, i) => (
              <div key={i} className="flex gap-2 mb-2">
                <input
                  type="text"
                  value={row.key}
                  onChange={(e) => {
                    const updated = [...envRows];
                    updated[i] = { ...row, key: e.target.value };
                    setEnvRows(updated);
                  }}
                  placeholder="KEY"
                  className="w-1/3 px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
                />
                <input
                  type="text"
                  value={row.value}
                  onChange={(e) => {
                    const updated = [...envRows];
                    updated[i] = { ...row, value: e.target.value };
                    setEnvRows(updated);
                  }}
                  placeholder="value"
                  className="flex-1 px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
                />
                <button
                  onClick={() => setEnvRows(envRows.filter((_, j) => j !== i))}
                  className="px-2 py-2 text-xs rounded-md bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
                >
                  -
                </button>
              </div>
            ))}
            <button
              onClick={() => setEnvRows([...envRows, { key: "", value: "" }])}
              className="px-3 py-1.5 text-xs font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors"
            >
              + Add Variable
            </button>
          </div>

          <div>
            <label className="block text-xs text-[var(--text-muted)] mb-1">
              Working Directory (optional)
            </label>
            <input
              type="text"
              value={workingDir}
              onChange={(e) => setWorkingDir(e.target.value)}
              placeholder="Leave empty for default"
              className="w-full px-3 py-2 rounded-md bg-[var(--bg-primary)] border border-[var(--border)] text-sm text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)] font-mono"
            />
          </div>
        </div>
      </div>

      {/* Tools */}
      {isConnected && tools.length > 0 && (
        <div>
          <h4 className="text-xs font-medium text-[var(--text-muted)] uppercase tracking-wider mb-3">
            Available Tools ({tools.length})
          </h4>
          <div className="space-y-1.5 max-h-48 overflow-y-auto">
            {tools.map((tool) => (
              <div
                key={tool.name}
                className="p-2.5 rounded-md bg-[var(--bg-primary)] border border-[var(--border)]"
              >
                <div className="text-xs font-medium text-[var(--text-primary)] font-mono">
                  {tool.name}
                </div>
                {tool.description && (
                  <div className="text-[11px] text-[var(--text-muted)] mt-0.5">
                    {tool.description}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {!isConnected && (
        <div className="p-4 rounded-md bg-[var(--bg-primary)] border border-[var(--border)]">
          <p className="text-sm text-[var(--text-muted)]">
            Start the server to see available tools.
          </p>
        </div>
      )}

      {error && <p className="text-xs text-red-400">{error}</p>}

      {/* Actions */}
      <div className="flex items-center justify-between pt-2 border-t border-[var(--border)]">
        <div className="flex items-center gap-2">
          {!isConnected && (
            <button
              onClick={() => onStart(server.name)}
              disabled={!!actionInProgress}
              className="px-4 py-2 text-sm font-medium rounded-md bg-[var(--accent)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
            >
              {actionInProgress === "starting" ? "Starting..." : "Start"}
            </button>
          )}
          {isConnected && (
            <>
              <button
                onClick={() => onStop(server.name)}
                disabled={!!actionInProgress}
                className="px-4 py-2 text-sm font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors disabled:opacity-50"
              >
                {actionInProgress === "stopping" ? "Stopping..." : "Stop"}
              </button>
              <button
                onClick={() => onRestart(server.name)}
                disabled={!!actionInProgress}
                className="px-4 py-2 text-sm font-medium rounded-md border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors disabled:opacity-50"
              >
                {actionInProgress === "restarting"
                  ? "Restarting..."
                  : "Restart"}
              </button>
            </>
          )}
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
                saveMsg === "Saved" ? "text-green-400" : "text-red-400"
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
          {confirmRemove ? "Confirm Remove" : "Remove Server"}
        </button>
      </div>
    </div>
  );
}
