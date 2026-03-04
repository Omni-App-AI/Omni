import {
  Save,
  CheckCircle,
  Play,
  X,
  FileDown,
  FileUp,
} from "lucide-react";
import { useFlowchartStore } from "../../stores/flowchartStore";

interface FlowchartToolbarProps {
  onToggleTest: () => void;
  showTestPanel: boolean;
}

export function FlowchartToolbar({
  onToggleTest,
  showTestPanel,
}: FlowchartToolbarProps) {
  const activeFlowchart = useFlowchartStore((s) => s.activeFlowchart);
  const editorDirty = useFlowchartStore((s) => s.editorDirty);
  const save = useFlowchartStore((s) => s.save);
  const validate = useFlowchartStore((s) => s.validate);
  const closeEditor = useFlowchartStore((s) => s.closeEditor);
  const lastValidation = useFlowchartStore((s) => s.lastValidation);
  const updateActive = useFlowchartStore((s) => s.updateActive);

  if (!activeFlowchart) return null;

  const handleExport = () => {
    const blob = new Blob([JSON.stringify(activeFlowchart, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${activeFlowchart.id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      const text = await file.text();
      try {
        const def = JSON.parse(text);
        updateActive(def);
      } catch {
        alert("Invalid JSON file");
      }
    };
    input.click();
  };

  return (
    <div className="h-10 border-b border-[var(--border)] bg-[var(--bg-secondary)] flex items-center px-3 gap-2">
      <div className="flex items-center gap-2 flex-1 min-w-0">
        <input
          className="text-sm font-medium bg-transparent border-none outline-none text-[var(--text-primary)] w-48"
          value={activeFlowchart.name}
          onChange={(e) => updateActive({ name: e.target.value })}
          aria-label="Flowchart name"
        />
        {editorDirty && (
          <span className="text-xs text-[var(--text-muted)]">(unsaved)</span>
        )}
        {lastValidation && (
          <span
            className={`text-xs ${lastValidation.valid ? "text-green-500" : "text-red-500"}`}
          >
            {lastValidation.valid
              ? "Valid"
              : `${lastValidation.errors.length} error(s)`}
          </span>
        )}
      </div>

      <div className="flex items-center gap-1">
        <button
          onClick={save}
          disabled={!editorDirty}
          className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-[var(--accent)] text-white disabled:opacity-40 hover:opacity-90"
          title="Save"
        >
          <Save size={14} />
          Save
        </button>
        <button
          onClick={validate}
          className="flex items-center gap-1 px-2 py-1 text-xs rounded border border-[var(--border)] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]"
          title="Validate"
        >
          <CheckCircle size={14} />
          Validate
        </button>
        <button
          onClick={onToggleTest}
          className={`flex items-center gap-1 px-2 py-1 text-xs rounded border border-[var(--border)] ${
            showTestPanel
              ? "bg-[var(--accent)] text-white border-transparent"
              : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]"
          }`}
          title="Test"
        >
          <Play size={14} />
          Test
        </button>
        <button
          onClick={handleExport}
          className="p-1 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
          title="Export JSON"
          aria-label="Export flowchart as JSON"
        >
          <FileDown size={14} />
        </button>
        <button
          onClick={handleImport}
          className="p-1 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
          title="Import JSON"
          aria-label="Import flowchart from JSON"
        >
          <FileUp size={14} />
        </button>
        <button
          onClick={closeEditor}
          className="p-1 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
          title="Close"
          aria-label="Close editor"
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
