import { useSettingsStore, type MessageStyle, type CodeTheme } from "../../../stores/settingsStore";
import { SettingsCard } from "./SettingsCard";
import { SegmentedControl } from "./SegmentedControl";
import { ToggleSwitch } from "./ToggleSwitch";

const MESSAGE_STYLES: { key: MessageStyle; label: string }[] = [
  { key: "bubbles", label: "Bubbles" },
  { key: "flat", label: "Flat" },
  { key: "compact", label: "Compact" },
];

const CODE_THEME_OPTIONS: { key: CodeTheme; label: string }[] = [
  { key: "light", label: "Light" },
  { key: "dark", label: "Dark" },
  { key: "auto", label: "Auto" },
];

// Mini preview for message styles
function StylePreview({ style, selected }: { style: MessageStyle; selected: boolean }) {
  const accentColor = "#3b82f6";
  const borderColor = selected ? accentColor : "var(--border)";

  return (
    <div
      className={`h-20 rounded-md border-2 p-2 transition-colors overflow-hidden ${
        selected ? "" : ""
      }`}
      style={{
        borderColor,
        background: "var(--bg-primary)",
      }}
    >
      {style === "bubbles" && (
        <div className="space-y-1.5">
          <div className="flex justify-end">
            <div className="h-3 rounded-xl px-2" style={{ background: accentColor, width: "50%", opacity: 0.8 }} />
          </div>
          <div className="flex justify-start">
            <div className="h-3 rounded-xl border" style={{ borderColor: "var(--border)", width: "60%", background: "var(--bg-secondary)" }} />
          </div>
          <div className="flex justify-end">
            <div className="h-3 rounded-xl" style={{ background: accentColor, width: "35%", opacity: 0.8 }} />
          </div>
        </div>
      )}
      {style === "flat" && (
        <div className="space-y-0">
          <div className="py-1 border-b" style={{ borderColor: "var(--border)" }}>
            <div className="h-2 rounded-sm" style={{ background: accentColor, width: "70%", opacity: 0.6 }} />
          </div>
          <div className="py-1 border-b" style={{ borderColor: "var(--border)" }}>
            <div className="h-2 rounded-sm" style={{ background: "var(--text-muted)", width: "85%", opacity: 0.3 }} />
          </div>
          <div className="py-1">
            <div className="h-2 rounded-sm" style={{ background: accentColor, width: "45%", opacity: 0.6 }} />
          </div>
        </div>
      )}
      {style === "compact" && (
        <div className="space-y-0.5">
          <div className="flex justify-end">
            <div className="h-2 rounded-md" style={{ background: accentColor, width: "45%", opacity: 0.7 }} />
          </div>
          <div className="flex justify-start">
            <div className="h-2 rounded-md" style={{ background: "var(--bg-secondary)", width: "55%", border: "1px solid var(--border)" }} />
          </div>
          <div className="flex justify-end">
            <div className="h-2 rounded-md" style={{ background: accentColor, width: "30%", opacity: 0.7 }} />
          </div>
          <div className="flex justify-start">
            <div className="h-2 rounded-md" style={{ background: "var(--bg-secondary)", width: "40%", border: "1px solid var(--border)" }} />
          </div>
        </div>
      )}
    </div>
  );
}

export function ChatAppearanceSection() {
  const messageStyle = useSettingsStore((s) => s.messageStyle);
  const maxMessageWidth = useSettingsStore((s) => s.maxMessageWidth);
  const codeTheme = useSettingsStore((s) => s.codeTheme);
  const showTimestamps = useSettingsStore((s) => s.showTimestamps);
  const setMessageStyle = useSettingsStore((s) => s.setMessageStyle);
  const setMaxMessageWidth = useSettingsStore((s) => s.setMaxMessageWidth);
  const setCodeTheme = useSettingsStore((s) => s.setCodeTheme);
  const setShowTimestamps = useSettingsStore((s) => s.setShowTimestamps);

  return (
    <SettingsCard title="Chat Appearance" description="Customize how messages look in the chat">
      <div className="space-y-5">
        {/* Message Style */}
        <div>
          <label className="text-xs text-[var(--text-secondary)] mb-2 block">Message Style</label>
          <div className="grid grid-cols-3 gap-3">
            {MESSAGE_STYLES.map(({ key, label }) => (
              <button key={key} onClick={() => setMessageStyle(key)} className="text-left">
                <StylePreview style={key} selected={messageStyle === key} />
                <span className={`block text-xs font-medium mt-1.5 text-center ${
                  messageStyle === key ? "text-[var(--accent)]" : "text-[var(--text-secondary)]"
                }`}>
                  {label}
                </span>
              </button>
            ))}
          </div>
        </div>

        {/* Max Message Width */}
        <div>
          <label className="text-xs text-[var(--text-secondary)] mb-2 block">Max Message Width</label>
          <div className="flex items-center gap-4">
            <span className="text-xs text-[var(--text-muted)] w-8">60%</span>
            <input
              type="range"
              min={60}
              max={100}
              step={5}
              value={maxMessageWidth}
              onChange={(e) => setMaxMessageWidth(Number(e.target.value))}
              className="flex-1 h-1.5 rounded-full appearance-none bg-[var(--border)] accent-[var(--accent)] cursor-pointer"
            />
            <span className="text-xs text-[var(--text-muted)] w-8">100%</span>
            <span className="text-sm text-[var(--text-primary)] font-medium w-12 text-right">
              {maxMessageWidth}%
            </span>
          </div>
        </div>

        {/* Code Theme */}
        <div>
          <label className="text-xs text-[var(--text-secondary)] mb-2 block">Code Block Theme</label>
          <SegmentedControl
            options={CODE_THEME_OPTIONS}
            value={codeTheme}
            onChange={setCodeTheme}
          />
        </div>

        {/* Show Timestamps */}
        <ToggleSwitch
          checked={showTimestamps}
          onChange={setShowTimestamps}
          label="Show Timestamps"
          description="Display time below each message"
        />
      </div>
    </SettingsCard>
  );
}
