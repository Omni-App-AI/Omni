import { useState, useCallback } from "react";
import { Wifi, WifiOff, Loader2, Settings, Trash2, AlertCircle } from "lucide-react";
import { ChannelIcon } from "./ChannelIcon";
import { getChannelMeta } from "./channelMeta";
import { InlineBindingEditor } from "./InlineBindingEditor";
import { useOmniEvent } from "../../hooks/useOmniEvents";
import type { ChannelDto, ChannelStatus, BindingDto, ExtensionDto, ExtensionInstanceDto } from "../../lib/tauri-commands";

interface ChannelSetupCardProps {
  channel: ChannelDto;
  bindings: BindingDto[];
  extensions: ExtensionDto[];
  instances?: ExtensionInstanceDto[];
  onConnect: (channelId: string) => void;
  onDisconnect: (channelId: string) => void;
  onConfigure: (channel: ChannelDto) => void;
  onRemove: (channelType: string, instanceId: string) => void;
  onAddBinding: (
    channelInstance: string,
    extensionId: string,
    peerFilter?: string,
    groupFilter?: string,
    priority?: number,
  ) => Promise<string | void>;
  onRemoveBinding: (bindingId: string) => Promise<void>;
}

function statusConfig(status: string): {
  color: string;
  label: string;
  icon: React.ReactNode;
} {
  switch (status as ChannelStatus) {
    case "connected":
      return {
        color: "var(--success)",
        label: "Connected",
        icon: <Wifi size={12} />,
      };
    case "connecting":
    case "reconnecting":
      return {
        color: "var(--warning)",
        label: status === "connecting" ? "Connecting" : "Reconnecting",
        icon: <Loader2 size={12} className="animate-spin" />,
      };
    case "error":
      return {
        color: "var(--error)",
        label: "Error",
        icon: <AlertCircle size={12} />,
      };
    default:
      return {
        color: "var(--text-muted)",
        label: "Disconnected",
        icon: <WifiOff size={12} />,
      };
  }
}

export function ChannelSetupCard({
  channel,
  bindings,
  extensions,
  instances,
  onConnect,
  onDisconnect,
  onConfigure,
  onRemove,
  onAddBinding,
  onRemoveBinding,
}: ChannelSetupCardProps) {
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [qrCodeData, setQrCodeData] = useState<string | null>(null);
  const meta = getChannelMeta(channel.channel_type);
  const status = statusConfig(channel.status);
  const isConnected = channel.status === "connected";
  const isConnecting =
    channel.status === "connecting" || channel.status === "reconnecting";
  const isDefault = channel.instance_id === "default";

  // Listen for QR code events for this channel
  const handleQrCode = useCallback(
    (payload: { channelId: string; qrData: string }) => {
      if (payload.channelId === channel.id) {
        setQrCodeData(payload.qrData);
      }
    },
    [channel.id],
  );
  useOmniEvent("omni:channel-qr-code", handleQrCode);

  // Clear QR when actually connected (user scanned successfully)
  const handleChannelConnected = useCallback(
    (payload: { channelId: string }) => {
      if (payload.channelId === channel.id) {
        setQrCodeData(null);
      }
    },
    [channel.id],
  );
  useOmniEvent("omni:channel-connected", handleChannelConnected);

  // Clear stale QR when disconnected (QR becomes invalid)
  const handleChannelDisconnected = useCallback(
    (payload: { channelId: string }) => {
      if (payload.channelId === channel.id) {
        setQrCodeData(null);
      }
    },
    [channel.id],
  );
  useOmniEvent("omni:channel-disconnected", handleChannelDisconnected);

  const features: string[] = [];
  if (channel.features.direct_messages) features.push("DM");
  if (channel.features.group_messages) features.push("Groups");
  if (channel.features.media_attachments) features.push("Media");
  if (channel.features.reactions) features.push("Reactions");
  if (channel.features.read_receipts) features.push("Read receipts");

  return (
    <div
      className="rounded-lg p-4 flex flex-col gap-3 transition-colors"
      style={{
        backgroundColor: "var(--bg-secondary)",
        border: "1px solid var(--border)",
      }}
      onMouseEnter={(e) =>
        (e.currentTarget.style.backgroundColor = "var(--bg-hover)")
      }
      onMouseLeave={(e) =>
        (e.currentTarget.style.backgroundColor = "var(--bg-secondary)")
      }
    >
      {/* Header: Icon + Name + Status */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div
            className="w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0"
            style={{
              backgroundColor: "color-mix(in srgb, var(--accent) 15%, transparent)",
            }}
          >
            <ChannelIcon
              iconName={meta.icon}
              size={18}
              style={{ color: "var(--accent)" }}
            />
          </div>
          <div className="flex flex-col">
            <h3
              className="font-semibold text-sm"
              style={{ color: "var(--text-primary)" }}
            >
              {meta.displayName}
            </h3>
            {!isDefault && (
              <span
                className="text-xs"
                style={{ color: "var(--text-muted)" }}
              >
                {channel.instance_id}
              </span>
            )}
          </div>
        </div>

        {/* Status badge */}
        <span
          className="flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium"
          style={{
            color: status.color,
            backgroundColor: `color-mix(in srgb, ${status.color} 12%, transparent)`,
          }}
        >
          {status.icon}
          {status.label}
        </span>
      </div>

      {/* Inline bindings */}
      <InlineBindingEditor
        channelId={channel.id}
        bindings={bindings}
        extensions={extensions}
        instances={instances}
        onAddBinding={onAddBinding}
        onRemoveBinding={onRemoveBinding}
      />

      {/* Features */}
      {features.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {features.map((f) => (
            <span
              key={f}
              className="px-1.5 py-0.5 rounded text-xs"
              style={{
                backgroundColor: "var(--bg-primary)",
                color: "var(--text-muted)",
              }}
            >
              {f}
            </span>
          ))}
        </div>
      )}

      {/* QR Code display (for QR-authenticated channels like WhatsApp) */}
      {qrCodeData && !isConnected && (
        <div
          className="rounded-lg p-4 flex flex-col items-center gap-3"
          style={{
            backgroundColor: "var(--bg-primary)",
            border: "1px solid var(--border)",
          }}
        >
          <p
            className="text-xs font-medium"
            style={{ color: "var(--text-primary)" }}
          >
            Scan with your {meta.displayName} app
          </p>
          <img
            src={qrCodeData}
            alt="QR Code"
            className="rounded"
            style={{ width: 200, height: 200, imageRendering: "pixelated" }}
          />
          <p
            className="text-xs text-center"
            style={{ color: "var(--text-muted)" }}
          >
            Open {meta.displayName} on your phone, go to Linked Devices, and
            scan this code
          </p>
          <p
            className="text-xs text-center"
            style={{ color: "var(--text-muted)", opacity: 0.7 }}
          >
            If scanning fails, remove any old "Omni" entries from Linked
            Devices first
          </p>
        </div>
      )}

      {/* Actions */}
      <div
        className="flex items-center gap-2 mt-auto pt-3"
        style={{ borderTop: "1px solid var(--border)" }}
      >
        {/* Primary action */}
        {isConnected ? (
          <button
            onClick={() => onDisconnect(channel.id)}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors"
            style={{
              backgroundColor: "color-mix(in srgb, var(--error) 15%, transparent)",
              color: "var(--error)",
            }}
          >
            Disconnect
          </button>
        ) : channel.status === "error" ? (
          <button
            onClick={() => onConnect(channel.id)}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors"
            style={{
              backgroundColor: "color-mix(in srgb, var(--warning) 15%, transparent)",
              color: "var(--warning)",
            }}
          >
            Retry
          </button>
        ) : (
          <button
            onClick={() => onConnect(channel.id)}
            disabled={isConnecting}
            className="px-3 py-1.5 rounded text-xs font-medium transition-colors disabled:opacity-50"
            style={{
              backgroundColor: "color-mix(in srgb, var(--accent) 15%, transparent)",
              color: "var(--accent)",
            }}
          >
            {isConnecting ? "Connecting..." : "Connect"}
          </button>
        )}

        {/* Configure button */}
        <button
          onClick={() => onConfigure(channel)}
          className="p-1.5 rounded transition-colors hover:opacity-80"
          style={{ color: "var(--text-muted)" }}
          title="Configure credentials"
        >
          <Settings size={14} />
        </button>

        {/* Remove button */}
        <div className="ml-auto relative">
          {confirmRemove ? (
            <div className="flex items-center gap-2">
              <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                Remove?
              </span>
              <button
                onClick={() => {
                  onRemove(channel.channel_type, channel.instance_id);
                  setConfirmRemove(false);
                }}
                className="px-2 py-1 rounded text-xs font-medium"
                style={{
                  backgroundColor: "color-mix(in srgb, var(--error) 15%, transparent)",
                  color: "var(--error)",
                }}
              >
                Yes
              </button>
              <button
                onClick={() => setConfirmRemove(false)}
                className="px-2 py-1 rounded text-xs font-medium"
                style={{
                  backgroundColor: "var(--bg-primary)",
                  color: "var(--text-muted)",
                }}
              >
                No
              </button>
            </div>
          ) : (
            <button
              onClick={() => setConfirmRemove(true)}
              className="p-1.5 rounded transition-colors hover:opacity-80"
              style={{ color: "var(--text-muted)" }}
              title="Remove channel"
            >
              <Trash2 size={14} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
