import { useSettingsStore, type FontFamily, type LineHeight, FONT_FAMILY_MAP } from "../../../stores/settingsStore";
import { SettingsCard } from "./SettingsCard";
import { SegmentedControl } from "./SegmentedControl";

const FONT_OPTIONS: { key: FontFamily; label: string }[] = [
  { key: "system", label: "System Default" },
  { key: "inter", label: "Inter" },
  { key: "jetbrains-mono", label: "JetBrains Mono" },
  { key: "fira-code", label: "Fira Code" },
  { key: "source-sans-3", label: "Source Sans 3" },
];

const LINE_HEIGHT_OPTIONS: { key: LineHeight; label: string }[] = [
  { key: "compact", label: "Compact" },
  { key: "normal", label: "Normal" },
  { key: "relaxed", label: "Relaxed" },
];

export function TypographySection() {
  const fontSize = useSettingsStore((s) => s.fontSize);
  const fontFamily = useSettingsStore((s) => s.fontFamily);
  const lineHeight = useSettingsStore((s) => s.lineHeight);
  const setFontSize = useSettingsStore((s) => s.setFontSize);
  const setFontFamily = useSettingsStore((s) => s.setFontFamily);
  const setLineHeight = useSettingsStore((s) => s.setLineHeight);

  return (
    <SettingsCard title="Typography" description="Customize fonts and text appearance">
      <div className="space-y-5">
        {/* Font Size */}
        <div>
          <label className="text-xs text-[var(--text-secondary)] mb-2 block">Font Size</label>
          <div className="flex items-center gap-4">
            <span className="text-xs text-[var(--text-muted)] w-6">12</span>
            <input
              type="range"
              min={12}
              max={20}
              step={1}
              value={fontSize}
              onChange={(e) => setFontSize(Number(e.target.value))}
              className="flex-1 h-1.5 rounded-full appearance-none bg-[var(--border)] accent-[var(--accent)] cursor-pointer"
            />
            <span className="text-xs text-[var(--text-muted)] w-6">20</span>
            <span className="text-sm text-[var(--text-primary)] font-medium w-12 text-right">
              {fontSize}px
            </span>
          </div>
        </div>

        {/* Font Family */}
        <div>
          <label className="text-xs text-[var(--text-secondary)] mb-2 block">Font Family</label>
          <select
            value={fontFamily}
            onChange={(e) => setFontFamily(e.target.value as FontFamily)}
            className="w-full px-3 py-2 text-sm rounded-md border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-primary)] cursor-pointer focus:outline-none focus:ring-1 focus:ring-[var(--accent)]"
          >
            {FONT_OPTIONS.map(({ key, label }) => (
              <option key={key} value={key} style={{ fontFamily: FONT_FAMILY_MAP[key] }}>
                {label}
              </option>
            ))}
          </select>
          {/* Preview */}
          <div
            className="mt-2 px-3 py-2 rounded-md border border-[var(--border)] bg-[var(--bg-primary)] text-sm text-[var(--text-secondary)]"
            style={{ fontFamily: FONT_FAMILY_MAP[fontFamily] }}
          >
            The quick brown fox jumps over the lazy dog.
          </div>
        </div>

        {/* Line Height */}
        <div>
          <label className="text-xs text-[var(--text-secondary)] mb-2 block">Line Height</label>
          <SegmentedControl
            options={LINE_HEIGHT_OPTIONS}
            value={lineHeight}
            onChange={setLineHeight}
          />
        </div>
      </div>
    </SettingsCard>
  );
}
