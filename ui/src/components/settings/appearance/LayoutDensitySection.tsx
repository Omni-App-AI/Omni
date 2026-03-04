import { useSettingsStore, type UiDensity } from "../../../stores/settingsStore";
import { SettingsCard } from "./SettingsCard";
import { SegmentedControl } from "./SegmentedControl";

const DENSITY_OPTIONS: { key: UiDensity; label: string }[] = [
  { key: "compact", label: "Compact" },
  { key: "comfortable", label: "Comfortable" },
  { key: "spacious", label: "Spacious" },
];

export function LayoutDensitySection() {
  const uiDensity = useSettingsStore((s) => s.uiDensity);
  const sidebarWidth = useSettingsStore((s) => s.sidebarWidth);
  const setUiDensity = useSettingsStore((s) => s.setUiDensity);
  const setSidebarWidth = useSettingsStore((s) => s.setSidebarWidth);

  return (
    <SettingsCard title="Layout & Density" description="Control spacing and layout proportions">
      <div className="space-y-5">
        {/* UI Density */}
        <div>
          <label className="text-xs text-[var(--text-secondary)] mb-2 block">UI Density</label>
          <SegmentedControl
            options={DENSITY_OPTIONS}
            value={uiDensity}
            onChange={setUiDensity}
          />
          <p className="text-[10px] text-[var(--text-muted)] mt-1.5">
            {uiDensity === "compact" && "Tighter spacing for more content on screen"}
            {uiDensity === "comfortable" && "Balanced spacing for everyday use"}
            {uiDensity === "spacious" && "More breathing room between elements"}
          </p>
        </div>

        {/* Sidebar Width */}
        <div>
          <label className="text-xs text-[var(--text-secondary)] mb-2 block">Sidebar Width</label>
          <div className="flex items-center gap-4">
            <span className="text-xs text-[var(--text-muted)] w-8">200</span>
            <input
              type="range"
              min={200}
              max={400}
              step={10}
              value={sidebarWidth}
              onChange={(e) => setSidebarWidth(Number(e.target.value))}
              className="flex-1 h-1.5 rounded-full appearance-none bg-[var(--border)] accent-[var(--accent)] cursor-pointer"
            />
            <span className="text-xs text-[var(--text-muted)] w-8">400</span>
            <span className="text-sm text-[var(--text-primary)] font-medium w-12 text-right">
              {sidebarWidth}px
            </span>
          </div>
        </div>
      </div>
    </SettingsCard>
  );
}
