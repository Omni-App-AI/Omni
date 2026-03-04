import { useState, useMemo, useCallback, useRef } from "react";
import { Play, CheckCircle, XCircle, ChevronDown, ChevronRight, Radio } from "lucide-react";
import { useFlowchartStore } from "../../stores/flowchartStore";
import { useOmniEvent } from "../../hooks/useOmniEvents";

interface LiveNodeEvent {
  nodeId: string;
  nodeType: string;
  durationMs: number;
  success: boolean;
  streamChunks?: string[];
}

export function FlowchartTestPanel() {
  const activeFlowchart = useFlowchartStore((s) => s.activeFlowchart);
  const lastTestResult = useFlowchartStore((s) => s.lastTestResult);
  const test = useFlowchartStore((s) => s.test);
  const loading = useFlowchartStore((s) => s.loading);
  const [paramsJson, setParamsJson] = useState("{}");
  const [selectedTool, setSelectedTool] = useState("");
  const [showTrace, setShowTrace] = useState(false);
  const [expandedTraceIdx, setExpandedTraceIdx] = useState<number | null>(null);

  // Real-time node execution events during test
  const [liveNodes, setLiveNodes] = useState<LiveNodeEvent[]>([]);
  const isRunning = useRef(false);

  // Listen for real-time node execution events
  const handleNodeExecuted = useCallback(
    (payload: { flowchartId: string; nodeId: string; nodeType: string; durationMs: number; success: boolean }) => {
      if (!isRunning.current) return;
      if (activeFlowchart && payload.flowchartId !== activeFlowchart.id) return;
      setLiveNodes((prev) => [
        ...prev,
        {
          nodeId: payload.nodeId,
          nodeType: payload.nodeType,
          durationMs: payload.durationMs,
          success: payload.success,
        },
      ]);
    },
    [activeFlowchart],
  );

  // Listen for streaming progress events (LLM chunks)
  const handleNodeProgress = useCallback(
    (payload: { flowchartId: string; nodeId: string; chunk: string }) => {
      if (!isRunning.current) return;
      if (activeFlowchart && payload.flowchartId !== activeFlowchart.id) return;
      setLiveNodes((prev) => {
        // Append chunk to the last node's streamChunks if it matches
        const last = prev[prev.length - 1];
        if (last && last.nodeId === payload.nodeId) {
          return [
            ...prev.slice(0, -1),
            { ...last, streamChunks: [...(last.streamChunks ?? []), payload.chunk] },
          ];
        }
        return prev;
      });
    },
    [activeFlowchart],
  );

  useOmniEvent("omni:flowchart-node-executed", handleNodeExecuted);
  useOmniEvent("omni:flowchart-node-progress", handleNodeProgress);

  const tools = useMemo(
    () => activeFlowchart?.tools ?? [],
    [activeFlowchart?.tools],
  );

  const toolName = selectedTool || tools[0]?.name || "";

  const handleRun = async () => {
    try {
      const params = JSON.parse(paramsJson);
      setLiveNodes([]);
      isRunning.current = true;
      await test(toolName, params);
      isRunning.current = false;
      setShowTrace(true);
    } catch {
      isRunning.current = false;
      alert("Invalid JSON parameters");
    }
  };

  const trace = lastTestResult?.node_trace ?? [];

  return (
    <div className="h-64 border-t border-[var(--border)] bg-[var(--bg-secondary)] flex flex-col">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-[var(--border)]">
        <span className="text-xs font-medium text-[var(--text-primary)]">
          Test
        </span>
        {tools.length > 1 && (
          <select
            className="text-xs px-1 py-0.5 rounded border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-primary)]"
            value={toolName}
            onChange={(e) => setSelectedTool(e.target.value)}
          >
            {tools.map((t) => (
              <option key={t.name} value={t.name}>
                {t.name}
              </option>
            ))}
          </select>
        )}
        <button
          onClick={handleRun}
          disabled={loading}
          className="flex items-center gap-1 px-2 py-0.5 text-xs rounded bg-green-600 text-white hover:bg-green-700 disabled:opacity-50"
        >
          <Play size={12} />
          Run
        </button>
        {loading && liveNodes.length > 0 && (
          <span className="flex items-center gap-1 text-xs text-blue-400 ml-2">
            <Radio size={12} className="animate-pulse" />
            {liveNodes.length} nodes
          </span>
        )}
        {lastTestResult && !loading && (
          <span className="flex items-center gap-1 text-xs ml-auto">
            {lastTestResult.success ? (
              <CheckCircle size={14} className="text-green-500" />
            ) : (
              <XCircle size={14} className="text-red-500" />
            )}
            {lastTestResult.execution_time_ms}ms
          </span>
        )}
      </div>
      <div className="flex flex-1 min-h-0 overflow-hidden">
        <div className="flex-1 flex flex-col border-r border-[var(--border)]">
          <div className="text-[10px] text-[var(--text-muted)] px-2 pt-1">
            Parameters (JSON)
          </div>
          <textarea
            className="flex-1 p-2 text-xs font-mono bg-transparent text-[var(--text-primary)] outline-none resize-none"
            value={paramsJson}
            onChange={(e) => setParamsJson(e.target.value)}
            placeholder='{"message": "hello"}'
          />
        </div>
        <div className="flex-1 flex flex-col">
          <div className="text-[10px] text-[var(--text-muted)] px-2 pt-1">
            Result
          </div>
          <div className="flex-1 p-2 text-xs font-mono text-[var(--text-secondary)] overflow-auto">
            {lastTestResult ? (
              lastTestResult.success ? (
                <pre className="whitespace-pre-wrap">
                  {JSON.stringify(lastTestResult.output, null, 2)}
                </pre>
              ) : (
                <span className="text-red-400">
                  Error: {lastTestResult.error}
                </span>
              )
            ) : (
              <span className="text-[var(--text-muted)]">
                Run a test to see results here.
              </span>
            )}
          </div>
          {/* Live node execution feed (L3 fix) */}
          {loading && liveNodes.length > 0 && (
            <div className="border-t border-[var(--border)]">
              <div className="text-[10px] text-blue-400 px-2 py-0.5 flex items-center gap-1">
                <Radio size={10} className="animate-pulse" /> Live Execution
              </div>
              <div className="max-h-24 overflow-auto px-2 pb-1">
                {liveNodes.map((entry, i) => (
                  <div
                    key={i}
                    className={`flex items-center gap-2 py-0.5 text-[10px] ${
                      entry.success ? "text-[var(--text-secondary)]" : "text-red-400"
                    }`}
                  >
                    <span
                      className="w-1.5 h-1.5 rounded-full shrink-0"
                      style={{
                        backgroundColor: entry.success ? "#22c55e" : "#ef4444",
                      }}
                    />
                    <span className="font-medium truncate max-w-[80px]">
                      {entry.nodeType}
                    </span>
                    <span className="text-[var(--text-muted)]">
                      {entry.nodeId}
                    </span>
                    <span className="ml-auto text-[var(--text-muted)] shrink-0">
                      {entry.durationMs}ms
                    </span>
                    {entry.streamChunks && entry.streamChunks.length > 0 && (
                      <span className="text-blue-300 truncate max-w-[100px]" title={entry.streamChunks.join("")}>
                        ...streaming
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
          {/* Node trace visualization (E3) */}
          {lastTestResult && !loading && trace.length > 0 && (
            <div className="border-t border-[var(--border)]">
              <button
                type="button"
                onClick={() => setShowTrace((s) => !s)}
                className="flex items-center gap-1 w-full px-2 py-1 text-[10px] text-[var(--text-muted)] hover:text-[var(--text-primary)]"
              >
                {showTrace ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
                Node Trace ({trace.length} nodes)
              </button>
              {showTrace && (
                <div className="max-h-32 overflow-auto px-2 pb-1">
                  {trace.map((entry, i) => (
                    <div key={i}>
                      <div
                        className={`flex items-center gap-2 py-0.5 text-[10px] cursor-pointer hover:bg-[var(--bg-primary)] ${
                          entry.error ? "text-red-400" : "text-[var(--text-secondary)]"
                        }`}
                        onClick={() => setExpandedTraceIdx(expandedTraceIdx === i ? null : i)}
                      >
                        <span className="text-[var(--text-muted)] w-4 text-right shrink-0">
                          {expandedTraceIdx === i ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
                        </span>
                        <span
                          className="w-1.5 h-1.5 rounded-full shrink-0"
                          style={{
                            backgroundColor: entry.error ? "#ef4444" : "#22c55e",
                          }}
                        />
                        <span className="font-medium truncate shrink-0 max-w-[80px]">
                          {entry.label}
                        </span>
                        <span className="text-[var(--text-muted)] truncate">
                          {entry.node_type}
                        </span>
                        <span className="ml-auto text-[var(--text-muted)] shrink-0">
                          {entry.duration_ms}ms
                        </span>
                        {entry.error && (
                          <span className="text-red-400 truncate max-w-[100px]" title={entry.error}>
                            {entry.error}
                          </span>
                        )}
                      </div>
                      {expandedTraceIdx === i && (entry.input != null || entry.output != null) && (
                        <div className="ml-6 mb-1 text-[9px] font-mono text-[var(--text-muted)] bg-[var(--bg-primary)] rounded p-1 overflow-auto max-h-24">
                          {entry.input != null && (
                            <div><span className="text-blue-400">in:</span> {String(JSON.stringify(entry.input, null, 1))}</div>
                          )}
                          {entry.output != null && (
                            <div><span className="text-green-400">out:</span> {String(JSON.stringify(entry.output, null, 1))}</div>
                          )}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
          {lastTestResult && trace.length === 0 && (
            <div className="border-t border-[var(--border)] px-2 py-1">
              <div className="text-[10px] text-[var(--text-muted)]">
                Execution time:{" "}
                <span className="text-[var(--text-secondary)]">
                  {lastTestResult.execution_time_ms}ms
                </span>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
