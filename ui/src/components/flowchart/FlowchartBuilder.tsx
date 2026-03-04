import { useEffect } from "react";
import { useFlowchartStore } from "../../stores/flowchartStore";
import { FlowchartListPanel } from "./FlowchartListPanel";
import { FlowchartEditor } from "./FlowchartEditor";

export function FlowchartBuilder() {
  const activeFlowchart = useFlowchartStore((s) => s.activeFlowchart);
  const loadFlowcharts = useFlowchartStore((s) => s.loadFlowcharts);
  const error = useFlowchartStore((s) => s.error);

  useEffect(() => {
    loadFlowcharts();
  }, [loadFlowcharts]);

  return (
    <div className="flex h-full">
      <FlowchartListPanel />
      <div className="flex-1 min-w-0 flex flex-col">
        {error && (
          <div className="px-3 py-2 bg-red-500/10 border-b border-red-500/30 text-red-400 text-xs flex items-center justify-between">
            <span>{error}</span>
            <button
              className="ml-2 text-red-300 hover:text-red-100"
              onClick={() => useFlowchartStore.setState({ error: null })}
            >
              Dismiss
            </button>
          </div>
        )}
        {activeFlowchart ? (
          <FlowchartEditor />
        ) : (
          <div className="flex items-center justify-center flex-1 text-[var(--text-muted)]">
            <div className="text-center">
              <p className="text-lg mb-2">Visual Flowchart Builder</p>
              <p className="text-sm">
                Select a flowchart or create a new one to get started
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
