import { useRef } from "react";
import { Check } from "lucide-react";
import { useSettingsStore } from "../../../stores/settingsStore";
import { SettingsCard } from "./SettingsCard";

const ACCENT_PRESETS = [
  { name: "Blue", color: "#3b82f6" },
  { name: "Purple", color: "#8b5cf6" },
  { name: "Indigo", color: "#6366f1" },
  { name: "Teal", color: "#14b8a6" },
  { name: "Emerald", color: "#10b981" },
  { name: "Rose", color: "#f43f5e" },
  { name: "Amber", color: "#f59e0b" },
  { name: "Orange", color: "#f97316" },
];

export function AccentColorSection() {
  const accentColor = useSettingsStore((s) => s.accentColor);
  const setAccentColor = useSettingsStore((s) => s.setAccentColor);
  const colorInputRef = useRef<HTMLInputElement>(null);

  const isPreset = ACCENT_PRESETS.some((p) => p.color === accentColor);
  const isCustom = !isPreset;

  return (
    <SettingsCard title="Accent Color" description="Pick a color for buttons, links, and highlights">
      <div className="flex items-center gap-2 flex-wrap">
        {ACCENT_PRESETS.map(({ name, color }) => (
          <button
            key={color}
            onClick={() => setAccentColor(color)}
            title={name}
            className={`relative w-8 h-8 rounded-full transition-all ${
              accentColor === color
                ? "ring-2 ring-offset-2 ring-[var(--accent)] ring-offset-[var(--bg-secondary)]"
                : "hover:scale-110"
            }`}
            style={{ backgroundColor: color }}
          >
            {accentColor === color && (
              <Check size={14} className="absolute inset-0 m-auto text-white" />
            )}
          </button>
        ))}

        {/* Custom color picker */}
        <div className="relative">
          <button
            onClick={() => colorInputRef.current?.click()}
            title="Custom color"
            className={`w-8 h-8 rounded-full border-2 border-dashed transition-all overflow-hidden ${
              isCustom
                ? "ring-2 ring-offset-2 ring-[var(--accent)] ring-offset-[var(--bg-secondary)] border-transparent"
                : "border-[var(--border)] hover:scale-110"
            }`}
            style={isCustom ? { backgroundColor: accentColor } : undefined}
          >
            {isCustom ? (
              <Check size={14} className="absolute inset-0 m-auto text-white" />
            ) : (
              <span className="block w-full h-full bg-gradient-to-br from-red-400 via-green-400 to-blue-400 rounded-full" />
            )}
          </button>
          <input
            ref={colorInputRef}
            type="color"
            value={accentColor}
            onChange={(e) => setAccentColor(e.target.value)}
            className="absolute inset-0 opacity-0 cursor-pointer w-0 h-0"
            tabIndex={-1}
          />
        </div>
      </div>
    </SettingsCard>
  );
}
