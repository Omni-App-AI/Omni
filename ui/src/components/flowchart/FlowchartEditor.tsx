import { useState } from "react";
import { ReactFlowProvider } from "@xyflow/react";
import { FlowchartToolbar } from "./FlowchartToolbar";
import { FlowchartCanvas } from "./FlowchartCanvas";
import { FlowchartNodePalette } from "./FlowchartNodePalette";
import { FlowchartConfigPanel } from "./FlowchartConfigPanel";
import { FlowchartTestPanel } from "./FlowchartTestPanel";

export function FlowchartEditor() {
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [showTestPanel, setShowTestPanel] = useState(false);
  const [rightPanel, setRightPanel] = useState<"palette" | "config">("palette");

  return (
    <ReactFlowProvider>
      <div className="flex flex-col h-full">
        <FlowchartToolbar
          onToggleTest={() => setShowTestPanel((p) => !p)}
          showTestPanel={showTestPanel}
        />
        <div className="flex flex-1 min-h-0">
          <div className="flex-1 min-w-0 flex flex-col">
            <FlowchartCanvas
              selectedNodeId={selectedNodeId}
              onSelectNode={(id) => {
                setSelectedNodeId(id);
                if (id) setRightPanel("config");
              }}
            />
            {showTestPanel && <FlowchartTestPanel />}
          </div>
          <div className="w-72 border-l border-[var(--border)] bg-[var(--bg-secondary)] flex flex-col">
            <div className="flex border-b border-[var(--border)]">
              <button
                className={`flex-1 px-3 py-2 text-xs font-medium ${
                  rightPanel === "palette"
                    ? "text-[var(--accent)] border-b-2 border-[var(--accent)]"
                    : "text-[var(--text-muted)]"
                }`}
                onClick={() => setRightPanel("palette")}
              >
                Nodes
              </button>
              <button
                className={`flex-1 px-3 py-2 text-xs font-medium ${
                  rightPanel === "config"
                    ? "text-[var(--accent)] border-b-2 border-[var(--accent)]"
                    : "text-[var(--text-muted)]"
                }`}
                onClick={() => setRightPanel("config")}
              >
                Config
              </button>
            </div>
            <div className="flex-1 overflow-y-auto">
              {rightPanel === "palette" ? (
                <FlowchartNodePalette />
              ) : (
                <FlowchartConfigPanel selectedNodeId={selectedNodeId} />
              )}
            </div>
          </div>
        </div>
      </div>
    </ReactFlowProvider>
  );
}
