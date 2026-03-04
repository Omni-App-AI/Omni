import { useSettingsStore } from "../../../stores/settingsStore";
import { SettingsCard } from "./SettingsCard";
import { ToggleSwitch } from "./ToggleSwitch";

export function MotionAccessibilitySection() {
  const reduceAnimations = useSettingsStore((s) => s.reduceAnimations);
  const highContrast = useSettingsStore((s) => s.highContrast);
  const setReduceAnimations = useSettingsStore((s) => s.setReduceAnimations);
  const setHighContrast = useSettingsStore((s) => s.setHighContrast);

  return (
    <SettingsCard title="Motion & Accessibility">
      <div className="space-y-4">
        <ToggleSwitch
          checked={reduceAnimations}
          onChange={setReduceAnimations}
          label="Reduce Animations"
          description="Minimize motion for transitions and loading indicators"
        />
        <ToggleSwitch
          checked={highContrast}
          onChange={setHighContrast}
          label="High Contrast"
          description="Increase text contrast for better readability"
        />
      </div>
    </SettingsCard>
  );
}
