import { useState } from "react";
import { X, Loader2, Check, Copy, CheckCheck } from "lucide-react";
import { ChannelIcon } from "./ChannelIcon";
import { getChannelMeta } from "./channelMeta";
import { channelGetApiKey } from "../../lib/tauri-commands";
import type { ChannelDto } from "../../lib/tauri-commands";

interface ChannelConfigDrawerProps {
  channel: ChannelDto;
  onClose: () => void;
  onLogin: (
    channelId: string,
    credentialType: string,
    data: Record<string, string>,
  ) => Promise<string>;
  onConnect: (channelId: string, settings: Record<string, unknown>) => Promise<void>;
}

export function ChannelConfigDrawer({
  channel,
  onClose,
  onLogin,
  onConnect,
}: ChannelConfigDrawerProps) {
  const meta = getChannelMeta(channel.channel_type);
  const [formData, setFormData] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [generatedApiKey, setGeneratedApiKey] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const updateField = (key: string, value: string) => {
    setFormData((prev) => ({ ...prev, [key]: value }));
  };

  const hasRequiredFields = meta.fields
    .filter((f) => f.required)
    .every((f) => (formData[f.key] ?? "").trim().length > 0);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSuccess(false);
    setGeneratedApiKey(null);
    try {
      if (meta.authMode === "credentials" && meta.credentialType) {
        // Only call login if the user provided credential data
        const hasData = Object.values(formData).some((v) => v.trim().length > 0);
        if (hasData) {
          const result = await onLogin(channel.id, meta.credentialType, formData);
          if (!result.includes("Success")) {
            setError(`Login failed: ${result}`);
            setSaving(false);
            return;
          }
        }
      }
      await onConnect(channel.id, {});
      setSuccess(true);

      // For webchat, fetch the API key (may have been auto-generated)
      if (channel.channel_type === "webchat") {
        try {
          const key = await channelGetApiKey(channel.id);
          if (key) {
            setGeneratedApiKey(key);
            return; // Don't auto-close — let user copy the key
          }
        } catch {
          // Non-fatal: key display is a convenience
        }
      }

      setTimeout(() => onClose(), 1200);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleCopyKey = async () => {
    if (!generatedApiKey) return;
    await navigator.clipboard.writeText(generatedApiKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-40"
        style={{ backgroundColor: "rgba(0, 0, 0, 0.3)" }}
        onClick={onClose}
      />

      {/* Drawer */}
      <div
        className="fixed top-0 right-0 bottom-0 z-50 w-96 flex flex-col shadow-xl"
        style={{
          backgroundColor: "var(--bg-primary)",
          borderLeft: "1px solid var(--border)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between p-4"
          style={{ borderBottom: "1px solid var(--border)" }}
        >
          <div className="flex items-center gap-3">
            <ChannelIcon
              iconName={meta.icon}
              size={20}
              style={{ color: "var(--accent)" }}
            />
            <div>
              <h3
                className="font-semibold text-sm"
                style={{ color: "var(--text-primary)" }}
              >
                Configure {meta.displayName}
              </h3>
              {channel.instance_id !== "default" && (
                <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                  {channel.instance_id}
                </span>
              )}
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded hover:opacity-70 transition-opacity"
            style={{ color: "var(--text-muted)" }}
          >
            <X size={18} />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-4">
          {meta.authMode === "qr_code" ? (
            <div
              className="rounded-lg p-4 text-center"
              style={{
                backgroundColor: "var(--bg-secondary)",
                border: "1px solid var(--border)",
              }}
            >
              <p className="text-sm" style={{ color: "var(--text-primary)" }}>
                QR Code Authentication
              </p>
              <p
                className="text-xs mt-2"
                style={{ color: "var(--text-muted)" }}
              >
                Click "Connect" on the channel card. A QR code will appear for
                you to scan with your {meta.displayName} app.
              </p>
              {meta.fields.length > 0 && (
                <div className="mt-4 flex flex-col gap-3 text-left">
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
                      <input
                        type={field.type === "textarea" ? "text" : field.type}
                        value={formData[field.key] ?? ""}
                        onChange={(e) => updateField(field.key, e.target.value)}
                        placeholder={field.placeholder}
                        className="px-3 py-2 rounded text-sm"
                        style={{
                          backgroundColor: "var(--bg-secondary)",
                          color: "var(--text-primary)",
                          border: "1px solid var(--border)",
                        }}
                      />
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
            </div>
          ) : meta.authMode === "none" ? (
            <div
              className="rounded-lg p-4 text-center"
              style={{
                backgroundColor: "var(--bg-secondary)",
                border: "1px solid var(--border)",
              }}
            >
              <p className="text-sm" style={{ color: "var(--text-primary)" }}>
                No credentials needed
              </p>
              <p
                className="text-xs mt-2"
                style={{ color: "var(--text-muted)" }}
              >
                This channel works without any authentication. Just connect it.
              </p>
              {meta.fields.length > 0 && (
                <div className="mt-4 flex flex-col gap-3 text-left">
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
                      <input
                        type={field.type === "textarea" ? "text" : field.type}
                        value={formData[field.key] ?? ""}
                        onChange={(e) => updateField(field.key, e.target.value)}
                        placeholder={field.placeholder}
                        className="px-3 py-2 rounded text-sm"
                        style={{
                          backgroundColor: "var(--bg-secondary)",
                          color: "var(--text-primary)",
                          border: "1px solid var(--border)",
                        }}
                      />
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
            </div>
          ) : (
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
                      value={formData[field.key] ?? ""}
                      onChange={(e) => updateField(field.key, e.target.value)}
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
                      value={formData[field.key] ?? ""}
                      onChange={(e) => updateField(field.key, e.target.value)}
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

          {/* Error */}
          {error && (
            <div
              className="rounded px-3 py-2 text-xs"
              style={{
                backgroundColor: "color-mix(in srgb, var(--error) 15%, transparent)",
                color: "var(--error)",
              }}
            >
              {error}
            </div>
          )}

          {/* Success */}
          {success && (
            <div
              className="rounded px-3 py-2 text-xs flex items-center gap-2"
              style={{
                backgroundColor: "color-mix(in srgb, var(--success) 15%, transparent)",
                color: "var(--success)",
              }}
            >
              <Check size={14} />
              Connected successfully
            </div>
          )}

          {/* Generated API Key display (WebChat) */}
          {generatedApiKey && (
            <div
              className="rounded-lg p-4 flex flex-col gap-2"
              style={{
                backgroundColor: "color-mix(in srgb, var(--accent) 10%, transparent)",
                border: "1px solid var(--accent)",
              }}
            >
              <p
                className="text-xs font-medium"
                style={{ color: "var(--text-primary)" }}
              >
                Your WebChat API Key
              </p>
              <div className="flex items-center gap-2">
                <code
                  className="flex-1 px-3 py-2 rounded text-xs font-mono select-all"
                  style={{
                    backgroundColor: "var(--bg-secondary)",
                    color: "var(--text-primary)",
                    border: "1px solid var(--border)",
                    wordBreak: "break-all",
                  }}
                >
                  {generatedApiKey}
                </code>
                <button
                  onClick={handleCopyKey}
                  className="p-2 rounded transition-colors"
                  style={{
                    backgroundColor: "var(--bg-secondary)",
                    color: copied ? "var(--success)" : "var(--text-muted)",
                    border: "1px solid var(--border)",
                  }}
                  title="Copy to clipboard"
                >
                  {copied ? <CheckCheck size={14} /> : <Copy size={14} />}
                </button>
              </div>
              <p
                className="text-xs"
                style={{ color: "var(--text-muted)" }}
              >
                Send this in the <code>x-api-key</code> header to authenticate requests.
              </p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div
          className="p-4 flex gap-2"
          style={{ borderTop: "1px solid var(--border)" }}
        >
          <button
            onClick={handleSave}
            disabled={saving || (meta.authMode === "credentials" && !hasRequiredFields)}
            className="flex-1 flex items-center justify-center gap-2 px-4 py-2 rounded text-sm font-medium transition-colors disabled:opacity-50"
            style={{ backgroundColor: "var(--accent)", color: "white" }}
          >
            {saving ? (
              <>
                <Loader2 size={14} className="animate-spin" />
                Saving...
              </>
            ) : (
              "Save & Connect"
            )}
          </button>
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
        </div>
      </div>
    </>
  );
}
