import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  useReactFlow,
  type Node,
  type Edge,
  type OnNodesChange,
  type OnEdgesChange,
  type OnConnect,
  type NodeTypes,
  applyNodeChanges,
  applyEdgeChanges,
  addEdge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useFlowchartStore } from "../../stores/flowchartStore";
import { TriggerNode } from "./nodes/TriggerNode";
import { ConditionNode } from "./nodes/ConditionNode";
import { HttpRequestNode } from "./nodes/HttpRequestNode";
import { LlmRequestNode } from "./nodes/LlmRequestNode";
import { ChannelSendNode } from "./nodes/ChannelSendNode";
import { StorageOpNode } from "./nodes/StorageOpNode";
import { ConfigGetNode } from "./nodes/ConfigGetNode";
import { TransformNode } from "./nodes/TransformNode";
import { MergeNode } from "./nodes/MergeNode";
import { LoopNode } from "./nodes/LoopNode";
import { SetVariableNode } from "./nodes/SetVariableNode";
import { DelayNode } from "./nodes/DelayNode";
import { LogNode } from "./nodes/LogNode";
import { OutputNode } from "./nodes/OutputNode";
import { ErrorHandlerNode } from "./nodes/ErrorHandlerNode";
import { NativeToolNode } from "./nodes/NativeToolNode";
import { SubFlowNode } from "./nodes/SubFlowNode";
import { SwitchNode } from "./nodes/SwitchNode";
import { CommentNode } from "./nodes/CommentNode";
import { MiniMapWithEdges } from "./MiniMapWithEdges";

/** Maps each node type to its palette color for MiniMap rendering. */
const nodeColorMap: Record<string, string> = {
  trigger: "#22c55e",
  condition: "#f59e0b",
  switch: "#d946ef",
  loop: "#8b5cf6",
  merge: "#6366f1",
  sub_flow: "#0ea5e9",
  output: "#ef4444",
  error_handler: "#f97316",
  http_request: "#3b82f6",
  llm_request: "#a855f7",
  channel_send: "#14b8a6",
  storage_op: "#64748b",
  config_get: "#78716c",
  native_tool: "#f43f5e",
  transform: "#ec4899",
  set_variable: "#06b6d4",
  log: "#84cc16",
  delay: "#a3a3a3",
  comment: "#eab308",
};

function getMiniMapNodeColor(node: Node): string {
  return nodeColorMap[node.type ?? ""] ?? "#64748b";
}

const nodeTypes: NodeTypes = {
  trigger: TriggerNode,
  condition: ConditionNode,
  http_request: HttpRequestNode,
  llm_request: LlmRequestNode,
  channel_send: ChannelSendNode,
  storage_op: StorageOpNode,
  config_get: ConfigGetNode,
  transform: TransformNode,
  merge: MergeNode,
  loop: LoopNode,
  set_variable: SetVariableNode,
  delay: DelayNode,
  log: LogNode,
  output: OutputNode,
  error_handler: ErrorHandlerNode,
  native_tool: NativeToolNode,
  sub_flow: SubFlowNode,
  switch: SwitchNode,
  comment: CommentNode,
};

interface FlowchartCanvasProps {
  selectedNodeId: string | null;
  onSelectNode: (id: string | null) => void;
}

function toReactFlowNodes(raw: unknown[]): Node[] {
  return (raw as Array<Record<string, unknown>>).map((n) => ({
    id: n.id as string,
    type: n.node_type as string,
    position: n.position as { x: number; y: number },
    data: {
      label: n.label as string,
      config: n.config ?? {},
    },
  }));
}

/** Auto-derive edge labels from source handle names (E4).
 *  E.g., Condition "true"/"false" handles, Switch "case_0"/"default" handles. */
function deriveEdgeLabel(
  sourceHandle: string | undefined,
  sourceNodeType: string | undefined,
): string | undefined {
  if (!sourceHandle) return undefined;
  // Condition branches
  if (sourceNodeType === "condition") {
    if (sourceHandle === "true") return "True";
    if (sourceHandle === "false") return "False";
  }
  // Switch branches
  if (sourceNodeType === "switch") {
    if (sourceHandle === "default") return "Default";
    if (sourceHandle.startsWith("case_")) return sourceHandle.replace("case_", "Case ");
  }
  return undefined;
}

function toReactFlowEdges(raw: unknown[], nodes: unknown[]): Edge[] {
  const nodeMap = new Map<string, string>();
  for (const n of nodes as Array<Record<string, unknown>>) {
    nodeMap.set(n.id as string, n.node_type as string);
  }

  return (raw as Array<Record<string, unknown>>).map((e) => {
    const sourceHandle = (e.source_handle as string) ?? undefined;
    const explicitLabel = (e.label as string) ?? undefined;
    const sourceType = nodeMap.get(e.source as string);
    const autoLabel = explicitLabel || deriveEdgeLabel(sourceHandle, sourceType);

    return {
      id: e.id as string,
      source: e.source as string,
      target: e.target as string,
      sourceHandle,
      targetHandle: (e.target_handle as string) ?? undefined,
      label: autoLabel,
      animated: true,
      selectable: true,
      interactionWidth: 20,
      style: { stroke: "var(--text-muted)", strokeWidth: 2 },
      labelStyle: autoLabel ? { fontSize: 10, fill: "var(--text-muted)" } : undefined,
    };
  });
}

function fromReactFlowNodes(nodes: Node[]): unknown[] {
  return nodes.map((n) => ({
    id: n.id,
    node_type: n.type,
    label: n.data.label ?? n.type,
    position: n.position,
    config: n.data.config ?? {},
  }));
}

function fromReactFlowEdges(edges: Edge[]): unknown[] {
  return edges.map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    source_handle: e.sourceHandle ?? null,
    target_handle: e.targetHandle ?? null,
    label: e.label ?? null,
  }));
}

export function FlowchartCanvas({
  selectedNodeId,
  onSelectNode,
}: FlowchartCanvasProps) {
  const activeFlowchart = useFlowchartStore((s) => s.activeFlowchart);
  const updateActive = useFlowchartStore((s) => s.updateActive);
  useReactFlow();

  // --- Two-layer state management ---
  // React Flow needs dimension/measured data on nodes that the Zustand store
  // doesn't track.  We keep local state (rfNodes/rfEdges) as the source of
  // truth for React Flow, and sync bidirectionally with the store.
  //
  // When onNodesChange fires dimension-only changes we update local state but
  // do NOT push to the store -- this breaks the infinite re-render loop that
  // was caused by: store update → re-derive nodes (without dimensions) →
  // React Flow re-measures → fires dimensions → store update → ...
  const [rfNodes, setRfNodes] = useState<Node[]>([]);
  const [rfEdges, setRfEdges] = useState<Edge[]>([]);
  const rfNodesRef = useRef<Node[]>([]);
  const rfEdgesRef = useRef<Edge[]>([]);

  // When we push changes to the store ourselves, set these flags so the
  // store→local sync effects skip the next update (we already have the data).
  const skipNodesSyncRef = useRef(false);
  const skipEdgesSyncRef = useRef(false);

  // Sync store → local nodes (fires on external changes: palette add, config edit, open flowchart)
  useEffect(() => {
    if (skipNodesSyncRef.current) {
      skipNodesSyncRef.current = false;
      return;
    }
    const storeNodes = activeFlowchart?.nodes ?? [];
    const freshRfNodes = toReactFlowNodes(storeNodes);

    // Preserve measured dimensions from existing local nodes so React Flow
    // doesn't need to re-measure nodes that haven't structurally changed.
    const merged = freshRfNodes.map((n) => {
      const existing = rfNodesRef.current.find((e) => e.id === n.id);
      if (existing?.measured) {
        return { ...n, measured: existing.measured };
      }
      return n;
    });

    rfNodesRef.current = merged;
    setRfNodes(merged);
  }, [activeFlowchart?.nodes]);

  // Sync store → local edges (with auto-derived labels from node types)
  useEffect(() => {
    if (skipEdgesSyncRef.current) {
      skipEdgesSyncRef.current = false;
      return;
    }
    const storeEdges = activeFlowchart?.edges ?? [];
    const storeNodes = activeFlowchart?.nodes ?? [];
    const freshRfEdges = toReactFlowEdges(storeEdges, storeNodes);
    rfEdgesRef.current = freshRfEdges;
    setRfEdges(freshRfEdges);
  }, [activeFlowchart?.edges, activeFlowchart?.nodes]);

  // --- React Flow callbacks ---

  const onNodesChange: OnNodesChange = useCallback(
    (changes) => {
      const updated = applyNodeChanges(changes, rfNodesRef.current);
      rfNodesRef.current = updated;
      setRfNodes([...updated]);

      // Only push structural changes (position, add, remove, select, reset)
      // to the store -- NOT dimension measurements.
      const hasMeaningful = changes.some((c) => c.type !== "dimensions");
      if (hasMeaningful) {
        skipNodesSyncRef.current = true;
        updateActive({ nodes: fromReactFlowNodes(updated) });
      }
    },
    [updateActive],
  );

  const onEdgesChange: OnEdgesChange = useCallback(
    (changes) => {
      const updated = applyEdgeChanges(changes, rfEdgesRef.current);
      rfEdgesRef.current = updated;
      setRfEdges([...updated]);

      // Only push structural changes (add, remove, reset) to the store.
      // Selection-only changes stay local to avoid the sync loop that
      // would immediately strip the `selected` property.
      const hasStructural = changes.some((c) => c.type !== "select");
      if (hasStructural) {
        skipEdgesSyncRef.current = true;
        updateActive({ edges: fromReactFlowEdges(updated) });
      }
    },
    [updateActive],
  );

  const onConnect: OnConnect = useCallback(
    (connection) => {
      const updated = addEdge(
        {
          ...connection,
          id: `e_${connection.source}_${connection.target}_${crypto.randomUUID().slice(0, 8)}`,
          animated: true,
          selectable: true,
          interactionWidth: 20,
          style: { stroke: "var(--text-muted)", strokeWidth: 2 },
        },
        rfEdgesRef.current,
      );
      rfEdgesRef.current = updated;
      setRfEdges([...updated]);
      skipEdgesSyncRef.current = true;
      updateActive({ edges: fromReactFlowEdges(updated) });
    },
    [updateActive],
  );

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      onSelectNode(node.id);
    },
    [onSelectNode],
  );

  const onPaneClick = useCallback(() => {
    onSelectNode(null);
  }, [onSelectNode]);

  // --- Copy/Paste support (E5) ---
  const clipboardRef = useRef<Array<Record<string, unknown>>>([]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Only handle when flow canvas is focused (not in input/textarea)
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      if (e.ctrlKey && e.key === "c") {
        // Copy selected nodes
        const selected = rfNodesRef.current.filter((n) => n.selected);
        if (selected.length === 0) return;
        clipboardRef.current = selected.map((n) => ({
          id: n.id,
          node_type: n.type,
          label: n.data.label ?? n.type,
          position: { x: n.position.x, y: n.position.y },
          config: n.data.config ?? {},
        }));
      }

      if (e.ctrlKey && e.key === "v") {
        // Paste nodes (offset by 40px)
        if (clipboardRef.current.length === 0 || !activeFlowchart) return;
        e.preventDefault();
        const newNodes = clipboardRef.current.map((n) => ({
          ...n,
          id: `${n.node_type}_${crypto.randomUUID().slice(0, 8)}`,
          position: {
            x: (n.position as { x: number }).x + 40,
            y: (n.position as { y: number }).y + 40,
          },
        }));
        // Update clipboard positions for subsequent pastes
        clipboardRef.current = newNodes.map((n) => ({ ...n }));
        updateActive({
          nodes: [...(activeFlowchart.nodes ?? []), ...newNodes],
        });
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeFlowchart, updateActive]);

  // Highlight selected node
  const styledNodes = useMemo(
    () =>
      rfNodes.map((n) => ({
        ...n,
        selected: n.id === selectedNodeId,
      })),
    [rfNodes, selectedNodeId],
  );

  return (
    <div className="flex-1" style={{ minHeight: 400, width: "100%", height: "100%" }}>
      <ReactFlow
        nodes={styledNodes}
        edges={rfEdges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        onPaneClick={onPaneClick}
        nodeTypes={nodeTypes}
        defaultViewport={
          activeFlowchart?.viewport ?? { x: 0, y: 0, zoom: 1 }
        }
        fitView={!activeFlowchart?.viewport}
        deleteKeyCode="Delete"
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={16} size={1} />
        <Controls />
        <MiniMapWithEdges nodeColor={getMiniMapNodeColor} />
      </ReactFlow>
    </div>
  );
}
