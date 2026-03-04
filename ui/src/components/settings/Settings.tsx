import { useState } from "react";
import { AppearanceSettings } from "./AppearanceSettings";
import { ProviderSettings } from "./ProviderSettings";
import { EnvKeysSettings } from "./EnvKeysSettings";
import { McpSettings } from "./McpSettings";
import { PrivacySettings } from "./PrivacySettings";
import { AdvancedSettings } from "./AdvancedSettings";

type SettingsTab = "appearance" | "providers" | "env-keys" | "mcp" | "privacy" | "advanced";

const TABS: { key: SettingsTab; label: string }[] = [
  { key: "appearance", label: "Appearance" },
  { key: "providers", label: "Providers" },
  { key: "env-keys", label: "Keys & Config" },
  { key: "mcp", label: "MCP Servers" },
  { key: "privacy", label: "Privacy" },
  { key: "advanced", label: "Advanced" },
];

const TAB_PANELS: Record<SettingsTab, React.FC> = {
  appearance: AppearanceSettings,
  providers: ProviderSettings,
  "env-keys": EnvKeysSettings,
  mcp: McpSettings,
  privacy: PrivacySettings,
  advanced: AdvancedSettings,
};

export function Settings() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("appearance");
  const Panel = TAB_PANELS[activeTab];

  return (
    <div className="flex flex-col h-full bg-[var(--bg-primary)]">
      <div className="px-6 pt-6 pb-0">
        <h1 className="text-lg font-semibold text-[var(--text-primary)] mb-4">
          Settings
        </h1>

        <div className="flex gap-1 border-b border-[var(--border)]">
          {TABS.map((tab) => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={`px-4 py-2 text-sm font-medium transition-colors relative ${
                activeTab === tab.key
                  ? "text-[var(--accent)]"
                  : "text-[var(--text-muted)] hover:text-[var(--text-primary)]"
              }`}
            >
              {tab.label}
              {activeTab === tab.key && (
                <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-[var(--accent)]" />
              )}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-6 py-6">
        <Panel />
      </div>
    </div>
  );
}
