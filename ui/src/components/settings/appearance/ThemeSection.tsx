import { useSettingsStore, type Theme } from "../../../stores/settingsStore";
import { SettingsCard } from "./SettingsCard";

const THEMES: { key: Theme; label: string }[] = [
  { key: "light", label: "Light" },
  { key: "dark", label: "Dark" },
  { key: "system", label: "System" },
];

// Mini preview colors for each theme
const PREVIEW_COLORS: Record<Theme, {
  sidebar: string; bg: string; border: string;
  accent: string; text: string; muted: string;
}> = {
  light: {
    sidebar: "#f1f3f5", bg: "#ffffff", border: "#e5e7eb",
    accent: "#3b82f6", text: "#1a1a2e", muted: "#9ca3af",
  },
  dark: {
    sidebar: "#16162a", bg: "#0f0f23", border: "#2d2d44",
    accent: "#60a5fa", text: "#e2e8f0", muted: "#64748b",
  },
  system: {
    sidebar: "#f1f3f5", bg: "#ffffff", border: "#e5e7eb",
    accent: "#3b82f6", text: "#1a1a2e", muted: "#9ca3af",
  },
};

function ThemeMiniPreview({ themeKey, selected }: { themeKey: Theme; selected: boolean }) {
  const c = PREVIEW_COLORS[themeKey];

  // System theme shows a diagonal split
  if (themeKey === "system") {
    const dark = PREVIEW_COLORS.dark;
    return (
      <div
        className={`relative h-16 rounded-md overflow-hidden border-2 transition-colors ${
          selected ? "border-[var(--accent)]" : "border-[var(--border)]"
        }`}
      >
        {/* Light half */}
        <div className="absolute inset-0" style={{ background: c.bg, clipPath: "polygon(0 0, 100% 0, 0 100%)" }}>
          <div className="absolute left-0 top-0 bottom-0 w-[30%]" style={{ background: c.sidebar, borderRight: `1px solid ${c.border}` }} />
          <div className="absolute left-[35%] top-[20%] right-[10%] h-[15%] rounded-sm" style={{ background: c.accent, opacity: 0.7 }} />
          <div className="absolute left-[35%] top-[50%] right-[20%] h-[10%] rounded-sm" style={{ background: c.muted, opacity: 0.3 }} />
        </div>
        {/* Dark half */}
        <div className="absolute inset-0" style={{ background: dark.bg, clipPath: "polygon(100% 0, 100% 100%, 0 100%)" }}>
          <div className="absolute left-0 top-0 bottom-0 w-[30%]" style={{ background: dark.sidebar, borderRight: `1px solid ${dark.border}` }} />
          <div className="absolute left-[35%] top-[20%] right-[10%] h-[15%] rounded-sm" style={{ background: dark.accent, opacity: 0.7 }} />
          <div className="absolute left-[35%] top-[50%] right-[20%] h-[10%] rounded-sm" style={{ background: dark.muted, opacity: 0.3 }} />
        </div>
      </div>
    );
  }

  return (
    <div
      className={`relative h-16 rounded-md overflow-hidden border-2 transition-colors ${
        selected ? "border-[var(--accent)]" : "border-[var(--border)]"
      }`}
      style={{ background: c.bg }}
    >
      {/* Sidebar */}
      <div className="absolute left-0 top-0 bottom-0 w-[30%]" style={{ background: c.sidebar, borderRight: `1px solid ${c.border}` }}>
        <div className="mt-2 mx-1.5 space-y-1">
          <div className="h-1 rounded-full" style={{ background: c.accent, width: "60%" }} />
          <div className="h-1 rounded-full" style={{ background: c.muted, opacity: 0.3, width: "80%" }} />
          <div className="h-1 rounded-full" style={{ background: c.muted, opacity: 0.3, width: "50%" }} />
        </div>
      </div>
      {/* Chat area */}
      <div className="absolute left-[32%] top-2 right-2 space-y-1.5">
        <div className="flex justify-end">
          <div className="h-2 rounded-full" style={{ background: c.accent, width: "40%", opacity: 0.8 }} />
        </div>
        <div className="flex justify-start">
          <div className="h-2 rounded-full" style={{ background: c.border, width: "55%" }} />
        </div>
        <div className="flex justify-end">
          <div className="h-2 rounded-full" style={{ background: c.accent, width: "30%", opacity: 0.8 }} />
        </div>
      </div>
    </div>
  );
}

export function ThemeSection() {
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);

  return (
    <SettingsCard title="Theme" description="Choose how Omni looks to you">
      <div className="grid grid-cols-3 gap-3">
        {THEMES.map(({ key, label }) => (
          <button key={key} onClick={() => setTheme(key)} className="text-left">
            <ThemeMiniPreview themeKey={key} selected={theme === key} />
            <span className={`block text-xs font-medium mt-1.5 text-center ${
              theme === key ? "text-[var(--accent)]" : "text-[var(--text-secondary)]"
            }`}>
              {label}
            </span>
          </button>
        ))}
      </div>
    </SettingsCard>
  );
}
