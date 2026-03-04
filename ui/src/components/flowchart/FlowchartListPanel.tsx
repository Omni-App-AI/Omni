import { useState } from "react";
import { Plus, Trash2, ToggleLeft, ToggleRight, BookOpen, ChevronDown, ChevronRight } from "lucide-react";
import { useFlowchartStore } from "../../stores/flowchartStore";
import { EXAMPLE_FLOWCHARTS } from "../../lib/example-flowcharts";

export function FlowchartListPanel() {
  const flowcharts = useFlowchartStore((s) => s.flowcharts);
  const activeFlowchart = useFlowchartStore((s) => s.activeFlowchart);
  const openFlowchart = useFlowchartStore((s) => s.openFlowchart);
  const createNew = useFlowchartStore((s) => s.createNew);
  const loadExample = useFlowchartStore((s) => s.loadExample);
  const deleteFlowchart = useFlowchartStore((s) => s.deleteFlowchart);
  const toggleEnabled = useFlowchartStore((s) => s.toggleEnabled);
  const [examplesOpen, setExamplesOpen] = useState(false);

  return (
    <div className="w-64 border-r border-[var(--border)] bg-[var(--bg-secondary)] flex flex-col h-full">
      <div className="p-3 border-b border-[var(--border)] flex items-center justify-between">
        <span className="text-sm font-medium text-[var(--text-primary)]">
          Flowcharts
        </span>
        <button
          onClick={createNew}
          className="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)]"
          title="Create new flowchart"
          aria-label="Create new flowchart"
        >
          <Plus size={16} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">
        {flowcharts.length === 0 ? (
          <p className="p-3 text-xs text-[var(--text-muted)]">
            No flowcharts yet. Click + to create one, or load an example below.
          </p>
        ) : (
          flowcharts.map((fc) => (
            <div
              key={fc.id}
              role="button"
              tabIndex={0}
              className={`p-3 border-b border-[var(--border)] cursor-pointer transition-colors ${
                activeFlowchart?.id === fc.id
                  ? "bg-[color-mix(in_srgb,var(--accent)_10%,transparent)]"
                  : "hover:bg-[var(--bg-hover)]"
              }`}
              onClick={() => openFlowchart(fc.id)}
              onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openFlowchart(fc.id); } }}
              aria-label={`Open flowchart: ${fc.name}`}
              aria-current={activeFlowchart?.id === fc.id ? "true" : undefined}
            >
              <div className="flex items-center justify-between mb-1">
                <span className="text-sm font-medium text-[var(--text-primary)] truncate">
                  {fc.name}
                </span>
                <div className="flex items-center gap-1 flex-shrink-0">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleEnabled(fc.id, !fc.enabled);
                    }}
                    className="p-0.5 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
                    title={fc.enabled ? "Disable" : "Enable"}
                    aria-label={fc.enabled ? `Disable ${fc.name}` : `Enable ${fc.name}`}
                    aria-pressed={fc.enabled}
                  >
                    {fc.enabled ? (
                      <ToggleRight size={14} className="text-green-500" />
                    ) : (
                      <ToggleLeft size={14} />
                    )}
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      if (confirm(`Delete "${fc.name}"?`)) {
                        deleteFlowchart(fc.id);
                      }
                    }}
                    className="p-0.5 text-[var(--text-muted)] hover:text-red-500"
                    title="Delete"
                    aria-label={`Delete ${fc.name}`}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
              <div className="text-xs text-[var(--text-muted)]">
                {fc.tool_count} tool{fc.tool_count !== 1 ? "s" : ""} &middot;{" "}
                {fc.permission_count} permission{fc.permission_count !== 1 ? "s" : ""}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Examples section */}
      <div className="border-t border-[var(--border)]">
        <button
          onClick={() => setExamplesOpen(!examplesOpen)}
          className="w-full p-3 flex items-center gap-2 text-sm font-medium text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] transition-colors"
        >
          <BookOpen size={14} />
          <span>Examples</span>
          {examplesOpen ? <ChevronDown size={14} className="ml-auto" /> : <ChevronRight size={14} className="ml-auto" />}
        </button>
        {examplesOpen && (
          <div className="pb-2">
            {EXAMPLE_FLOWCHARTS.map((ex) => (
              <button
                key={ex.id}
                onClick={() => loadExample(ex.id)}
                className="w-full text-left px-3 py-2 hover:bg-[var(--bg-hover)] transition-colors"
              >
                <div className="text-xs font-medium text-[var(--accent)]">
                  {ex.name}
                </div>
                <div className="text-[10px] text-[var(--text-muted)] mt-0.5 leading-tight">
                  {ex.description}
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
