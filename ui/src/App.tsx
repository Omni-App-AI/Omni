import { useState, useCallback, useEffect } from "react";
import { Layout } from "./components/shared/Layout";
import { Sidebar, type Page } from "./components/shared/Sidebar";
import { ToastProvider, useToast } from "./components/shared/ToastProvider";
import { UpdateBanner } from "./components/UpdateBanner";
import { ChatPanel } from "./components/chat/ChatPanel";
import { ActionFeed } from "./components/action-feed/ActionFeed";
import { PermissionDashboard } from "./components/permissions/PermissionDashboard";
import { ExtensionManager } from "./components/extensions/ExtensionManager";
import { GuardianMonitor } from "./components/guardian/GuardianMonitor";
import { ChannelPanel } from "./components/channels/ChannelPanel";
import { Settings } from "./components/settings/Settings";
import { MarketplacePage } from "./components/marketplace/MarketplacePage";
import { FlowchartBuilder } from "./components/flowchart/FlowchartBuilder";
import { useOmniEvent } from "./hooks/useOmniEvents";
import { useEventStore } from "./stores/eventStore";
import { useSettingsStore } from "./stores/settingsStore";
import { startAutoUpdateCheck, stopAutoUpdateCheck } from "./stores/updateStore";

function AppContent() {
  const [page, setPage] = useState<Page>("chat");
  const addEvent = useEventStore((s) => s.addEvent);
  const { addToast } = useToast();

  // Load persisted settings and apply theme + appearance on mount
  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const autoUpdate = useSettingsStore((s) => s.autoUpdate);
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Start/stop auto-update checking based on settings
  useEffect(() => {
    if (autoUpdate) {
      startAutoUpdateCheck();
    } else {
      stopAutoUpdateCheck();
    }
    return () => stopAutoUpdateCheck();
  }, [autoUpdate]);

  // Subscribe to events for the event store and toasts
  const handleGuardianBlocked = useCallback(
    (payload: Record<string, unknown>) => {
      addEvent("guardian-blocked", payload);
      addToast(
        `Guardian blocked: ${payload.reason ?? "Unknown reason"}`,
        "warning",
      );
    },
    [addEvent, addToast],
  );

  const handleExtensionError = useCallback(
    (payload: Record<string, unknown>) => {
      addEvent("extension-error", payload);
      addToast(`Extension error: ${payload.error ?? "Unknown"}`, "error");
    },
    [addEvent, addToast],
  );

  const handleGenericEvent = useCallback(
    (eventType: string) => (payload: Record<string, unknown>) => {
      addEvent(eventType, payload);
    },
    [addEvent],
  );

  useOmniEvent("omni:guardian-blocked", handleGuardianBlocked);
  useOmniEvent("omni:extension-error", handleExtensionError);
  useOmniEvent("omni:llm-chunk", handleGenericEvent("llm-chunk"));
  useOmniEvent("omni:llm-complete", handleGenericEvent("llm-complete"));
  useOmniEvent("omni:permission-prompt", handleGenericEvent("permission-prompt"));
  useOmniEvent("omni:permission-checked", handleGenericEvent("permission-checked"));
  useOmniEvent("omni:extension-invoked", handleGenericEvent("extension-invoked"));
  useOmniEvent("omni:extension-result", handleGenericEvent("extension-result"));
  useOmniEvent("omni:guardian-scan", handleGenericEvent("guardian-scan"));
  useOmniEvent("omni:channel-connected", handleGenericEvent("channel-connected"));
  useOmniEvent("omni:channel-disconnected", handleGenericEvent("channel-disconnected"));
  useOmniEvent("omni:channel-message-received", handleGenericEvent("channel-message-received"));
  useOmniEvent("omni:channel-message-sent", handleGenericEvent("channel-message-sent"));
  useOmniEvent("omni:channel-error", handleGenericEvent("channel-error"));
  useOmniEvent("omni:channel-instance-created", handleGenericEvent("channel-instance-created"));
  useOmniEvent("omni:channel-instance-removed", handleGenericEvent("channel-instance-removed"));
  useOmniEvent("omni:channel-binding-added", handleGenericEvent("channel-binding-added"));
  useOmniEvent("omni:channel-binding-removed", handleGenericEvent("channel-binding-removed"));

  const handleNotification = useCallback(
    (payload: Record<string, unknown>) => {
      addEvent("notification", payload);
      const title = payload.title ?? "Notification";
      const body = payload.body ?? "";
      const urgency = payload.urgency as string | undefined;
      const variant = urgency === "critical" ? "error" : urgency === "high" ? "warning" : "info";
      addToast(`${title}: ${body}`, variant);
    },
    [addEvent, addToast],
  );
  useOmniEvent("omni:notification", handleNotification);

  const renderPage = () => {
    switch (page) {
      case "chat":
        return <ChatPanel />;
      case "action-feed":
        return <ActionFeed />;
      case "permissions":
        return <PermissionDashboard />;
      case "extensions":
        return <ExtensionManager />;
      case "marketplace":
        return <MarketplacePage />;
      case "channels":
        return <ChannelPanel />;
      case "flowcharts":
        return <FlowchartBuilder />;
      case "guardian":
        return <GuardianMonitor />;
      case "settings":
        return <Settings />;
    }
  };

  return (
    <div className="flex flex-col h-screen">
      <UpdateBanner />
      <div className="flex-1 min-h-0">
        <Layout sidebar={<Sidebar activePage={page} onNavigate={setPage} />}>
          {renderPage()}
        </Layout>
      </div>
    </div>
  );
}

function App() {
  return (
    <ToastProvider>
      <AppContent />
    </ToastProvider>
  );
}

export default App;
