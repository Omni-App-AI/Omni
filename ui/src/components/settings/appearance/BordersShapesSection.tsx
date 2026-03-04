import { useSettingsStore } from "../../../stores/settingsStore";
import { SettingsCard } from "./SettingsCard";

export function BordersShapesSection() {
  const borderRadius = useSettingsStore((s) => s.borderRadius);
  const setBorderRadius = useSettingsStore((s) => s.setBorderRadius);

  return (
    <SettingsCard title="Borders & Shapes" description="Control the roundness of UI elements">
      <div className="space-y-4">
        {/* Slider */}
        <div>
          <div className="flex items-center gap-4">
            <span className="text-xs text-[var(--text-muted)] w-10">Sharp</span>
            <input
              type="range"
              min={0}
              max={16}
              step={1}
              value={borderRadius}
              onChange={(e) => setBorderRadius(Number(e.target.value))}
              className="flex-1 h-1.5 rounded-full appearance-none bg-[var(--border)] accent-[var(--accent)] cursor-pointer"
            />
            <span className="text-xs text-[var(--text-muted)] w-8">Pill</span>
            <span className="text-sm text-[var(--text-primary)] font-medium w-12 text-right">
              {borderRadius}px
            </span>
          </div>
        </div>

        {/* Live Preview */}
        <div className="flex items-center gap-4 pt-2">
          <button
            className="px-4 py-2 text-xs font-medium text-white bg-[var(--accent)] transition-all"
            style={{ borderRadius: `${borderRadius}px` }}
          >
            Button
          </button>
          <div
            className="flex-1 px-4 py-3 border border-[var(--border)] bg-[var(--bg-primary)] text-xs text-[var(--text-secondary)] transition-all"
            style={{ borderRadius: `${borderRadius}px` }}
          >
            Sample card preview
          </div>
          <input
            readOnly
            value="Input field"
            className="px-3 py-2 text-xs border border-[var(--border)] bg-[var(--bg-input)] text-[var(--text-primary)] transition-all w-24"
            style={{ borderRadius: `${borderRadius}px` }}
          />
        </div>
      </div>
    </SettingsCard>
  );
}
