import { useState } from "react";
import { RotateCcw } from "lucide-react";
import { useSettingsStore } from "../../../stores/settingsStore";

export function ResetSection() {
  const resetAppearance = useSettingsStore((s) => s.resetAppearance);
  const [confirming, setConfirming] = useState(false);

  const handleReset = () => {
    if (!confirming) {
      setConfirming(true);
      setTimeout(() => setConfirming(false), 3000);
      return;
    }
    resetAppearance();
    setConfirming(false);
  };

  return (
    <div className="flex justify-end pt-2">
      <button
        onClick={handleReset}
        className={`flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-md border transition-colors ${
          confirming
            ? "border-[var(--danger)] text-[var(--danger)] bg-[var(--danger)]/10"
            : "border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]"
        }`}
      >
        <RotateCcw size={14} />
        {confirming ? "Click again to confirm" : "Reset to Defaults"}
      </button>
    </div>
  );
}
