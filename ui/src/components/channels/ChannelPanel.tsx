import { useEffect, useState, useMemo, useCallback } from "react";
import { Radio, Plus, Loader2 } from "lucide-react";
import { useChannelStore } from "../../stores/channelStore";
import { listExtensions, listExtensionInstances, type ExtensionDto, type ExtensionInstanceDto, type ChannelDto } from "../../lib/tauri-commands";
import { useOmniEvent } from "../../hooks/useOmniEvents";
import { EmptyChannelState } from "./EmptyChannelState";
import { ChannelSetupCard } from "./ChannelSetupCard";
import { AddChannelWizard } from "./AddChannelWizard";
import { ChannelConfigDrawer } from "./ChannelConfigDrawer";
import { getChannelMeta } from "./channelMeta";

const STATUS_ORDER: Record<string, number> = {
  connected: 0,
  connecting: 1,
  reconnecting: 2,
  error: 3,
  disconnected: 4,
};

export function ChannelPanel() {
  const {
    channels,
    channelBindings,
    loading,
    loadAll,
    connect,
    disconnect,
    login,
    createInstance,
    removeInstance,
    addBindingForChannel,
    removeBindingForChannel,
  } = useChannelStore();

  const [extensions, setExtensions] = useState<ExtensionDto[]>([]);
  const [extInstances, setExtInstances] = useState<ExtensionInstanceDto[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [showWizard, setShowWizard] = useState(false);
  const [wizardPreselect, setWizardPreselect] = useState<string | null>(null);
  const [configuringChannel, setConfiguringChannel] = useState<ChannelDto | null>(null);

  useEffect(() => {
    loadAll();
    listExtensions()
      .then(setExtensions)
      .catch(() => {});
    listExtensionInstances()
      .then(setExtInstances)
      .catch(() => {});
  }, [loadAll]);

  // Refresh channel list when backend status changes
  const { loadChannels, loadBindings } = useChannelStore();
  const handleChannelStatusChange = useCallback(() => {
    loadChannels();
  }, [loadChannels]);
  const handleBindingChange = useCallback(() => {
    loadBindings();
  }, [loadBindings]);

  useOmniEvent("omni:channel-connected", handleChannelStatusChange);
  useOmniEvent("omni:channel-disconnected", handleChannelStatusChange);
  useOmniEvent("omni:channel-error", handleChannelStatusChange);
  useOmniEvent("omni:channel-binding-added", handleBindingChange);
  useOmniEvent("omni:channel-binding-removed", handleBindingChange);

  // Sort channels: connected first, then by name
  const sortedChannels = useMemo(() => {
    return [...channels].sort((a, b) => {
      const aOrder = STATUS_ORDER[a.status] ?? 4;
      const bOrder = STATUS_ORDER[b.status] ?? 4;
      if (aOrder !== bOrder) return aOrder - bOrder;
      return a.name.localeCompare(b.name);
    });
  }, [channels]);

  const openWizard = (preselect?: string) => {
    setWizardPreselect(preselect ?? null);
    setShowWizard(true);
  };

  const handleWizardComplete = async (result: {
    channelType: string;
    instanceId: string;
    credentialType: string;
    credentials: Record<string, string>;
    extensionId: string | null;
  }) => {
    setError(null);
    try {
      // 1. Create instance
      const channelId = await createInstance(result.channelType, result.instanceId);

      // 2. Login (if credentials provided)
      const meta = getChannelMeta(result.channelType);
      if (
        meta.authMode === "credentials" &&
        result.credentialType &&
        Object.keys(result.credentials).length > 0
      ) {
        const loginResult = await login(channelId, result.credentialType, result.credentials);
        if (!loginResult.includes("Success")) {
          setError(`Login failed: ${loginResult}`);
          setShowWizard(false);
          return;
        }
      }

      // 3. Add binding (if extension selected)
      if (result.extensionId) {
        await addBindingForChannel(channelId, result.extensionId);
      }

      // 4. Connect
      await connect(channelId, {});

      setShowWizard(false);
      setWizardPreselect(null);
    } catch (e) {
      throw e; // Let wizard display the error
    }
  };

  const handleConnect = async (channelId: string) => {
    setError(null);
    try {
      await connect(channelId, {});
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDisconnect = async (channelId: string) => {
    setError(null);
    try {
      await disconnect(channelId);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemove = async (channelType: string, instanceId: string) => {
    setError(null);
    try {
      await removeInstance(channelType, instanceId);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div
      className="flex flex-col gap-6 p-6 flex-1 overflow-y-auto"
      style={{
        backgroundColor: "var(--bg-primary)",
        color: "var(--text-primary)",
      }}
    >
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Radio size={24} style={{ color: "var(--accent)" }} />
          <div>
            <h2 className="text-xl font-bold">Channels</h2>
            <p className="text-sm" style={{ color: "var(--text-muted)" }}>
              Messaging platform integrations
            </p>
          </div>
        </div>
        {channels.length > 0 && (
          <button
            onClick={() => openWizard()}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded text-sm font-medium transition-opacity hover:opacity-90"
            style={{ backgroundColor: "var(--accent)", color: "white" }}
          >
            <Plus size={16} />
            Add Channel
          </button>
        )}
      </div>

      {/* Error */}
      {error && (
        <div
          className="rounded-lg px-4 py-3 text-sm"
          style={{
            backgroundColor: "color-mix(in srgb, var(--error) 15%, transparent)",
            color: "var(--error)",
            border: "1px solid var(--error)",
          }}
        >
          {error}
          <button
            onClick={() => setError(null)}
            className="ml-3 underline text-xs"
          >
            dismiss
          </button>
        </div>
      )}

      {/* Content */}
      {loading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2
            size={24}
            className="animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      ) : channels.length === 0 ? (
        <EmptyChannelState onAddChannel={openWizard} />
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {sortedChannels.map((ch) => (
            <ChannelSetupCard
              key={ch.id}
              channel={ch}
              bindings={channelBindings[ch.id] ?? []}
              extensions={extensions}
              instances={extInstances}
              onConnect={handleConnect}
              onDisconnect={handleDisconnect}
              onConfigure={setConfiguringChannel}
              onRemove={handleRemove}
              onAddBinding={addBindingForChannel}
              onRemoveBinding={removeBindingForChannel}
            />
          ))}
        </div>
      )}

      {/* Wizard overlay */}
      {showWizard && (
        <AddChannelWizard
          preselectedType={wizardPreselect}
          onClose={() => {
            setShowWizard(false);
            setWizardPreselect(null);
          }}
          onComplete={handleWizardComplete}
        />
      )}

      {/* Config drawer */}
      {configuringChannel && (
        <ChannelConfigDrawer
          channel={configuringChannel}
          onClose={() => setConfiguringChannel(null)}
          onLogin={login}
          onConnect={connect}
        />
      )}
    </div>
  );
}
