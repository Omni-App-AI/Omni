import { useEffect, useState } from "react";
import { getSettings, updateSettings } from "../../lib/tauri-commands";

const LEVELS = ["strict", "balanced", "permissive"] as const;
type SensitivityLevel = (typeof LEVELS)[number];

const LEVEL_LABELS: Record<SensitivityLevel, string> = {
  strict: "Strict",
  balanced: "Balanced",
  permissive: "Permissive",
};

const LEVEL_DESCRIPTIONS: Record<SensitivityLevel, string> = {
  strict: "Maximum protection. May produce more false positives.",
  balanced: "Recommended. Good balance of safety and usability.",
  permissive: "Minimal blocking. Only high-confidence threats stopped.",
};

function isValidLevel(value: string): value is SensitivityLevel {
  return LEVELS.includes(value as SensitivityLevel);
}

export function SensitivitySlider() {
  const [selected, setSelected] = useState<SensitivityLevel>("balanced");
  const [saving, setSaving] = useState(false);

  // Load saved sensitivity on mount
  useEffect(() => {
    getSettings()
      .then((json) => {
        const parsed = JSON.parse(json);
        const saved = parsed.guardian_sensitivity;
        if (saved && isValidLevel(saved)) {
          setSelected(saved);
        }
      })
      .catch(() => {});
  }, []);

  const handleChange = async (level: SensitivityLevel) => {
    const previous = selected;
    setSelected(level);
    setSaving(true);
    try {
      await updateSettings({ guardianSensitivity: level });
    } catch (err) {
      console.error("Failed to update sensitivity:", err);
      setSelected(previous); // Rollback on failure
    } finally {
      setSaving(false);
    }
  };

  return (
    <div
      className="rounded-lg p-4 flex flex-col gap-3"
      style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border)" }}
    >
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
          Guardian Sensitivity
        </h3>
        {saving && (
          <span className="text-xs" style={{ color: "var(--text-muted)" }}>
            Saving...
          </span>
        )}
      </div>

      <div className="flex rounded overflow-hidden" style={{ border: "1px solid var(--border)" }}>
        {LEVELS.map((level) => (
          <button
            key={level}
            onClick={() => handleChange(level)}
            className="flex-1 px-3 py-2 text-sm font-medium transition-colors"
            style={{
              backgroundColor: selected === level ? "var(--accent)" : "var(--bg-primary)",
              color: selected === level ? "white" : "var(--text-secondary)",
              borderRight: level !== "permissive" ? "1px solid var(--border)" : undefined,
            }}
          >
            {LEVEL_LABELS[level]}
          </button>
        ))}
      </div>

      <p className="text-xs" style={{ color: "var(--text-muted)" }}>
        {LEVEL_DESCRIPTIONS[selected]}
      </p>
    </div>
  );
}
