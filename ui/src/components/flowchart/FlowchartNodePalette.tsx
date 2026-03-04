import {
  Zap,
  GitBranch,
  Globe,
  Brain,
  MessageSquare,
  Database,
  Settings,
  Shuffle,
  GitMerge,
  Repeat,
  Variable,
  Clock,
  FileText,
  CircleDot,
  ShieldAlert,
  Wrench,
  Workflow,
  ListTree,
  StickyNote,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState, type ElementType } from "react";
import { useReactFlow } from "@xyflow/react";
import { useFlowchartStore } from "../../stores/flowchartStore";

interface NodeTypeInfo {
  type: string;
  label: string;
  icon: ElementType;
  color: string;
}

const categories: { label: string; items: NodeTypeInfo[] }[] = [
  {
    label: "Flow Control",
    items: [
      { type: "trigger", label: "Trigger", icon: Zap, color: "#22c55e" },
      { type: "condition", label: "Condition", icon: GitBranch, color: "#f59e0b" },
      { type: "switch", label: "Switch", icon: ListTree, color: "#d946ef" },
      { type: "loop", label: "Loop", icon: Repeat, color: "#8b5cf6" },
      { type: "merge", label: "Merge", icon: GitMerge, color: "#6366f1" },
      { type: "sub_flow", label: "Sub-Flow", icon: Workflow, color: "#0ea5e9" },
      { type: "output", label: "Output", icon: CircleDot, color: "#ef4444" },
      { type: "error_handler", label: "Error Handler", icon: ShieldAlert, color: "#f97316" },
    ],
  },
  {
    label: "Actions",
    items: [
      { type: "http_request", label: "HTTP Request", icon: Globe, color: "#3b82f6" },
      { type: "llm_request", label: "LLM Request", icon: Brain, color: "#a855f7" },
      { type: "channel_send", label: "Channel Send", icon: MessageSquare, color: "#14b8a6" },
      { type: "storage_op", label: "Storage", icon: Database, color: "#64748b" },
      { type: "config_get", label: "Config Get", icon: Settings, color: "#78716c" },
      { type: "native_tool", label: "Native Tool", icon: Wrench, color: "#f43f5e" },
    ],
  },
  {
    label: "Data",
    items: [
      { type: "transform", label: "Transform", icon: Shuffle, color: "#ec4899" },
      { type: "set_variable", label: "Set Variable", icon: Variable, color: "#06b6d4" },
      { type: "log", label: "Log", icon: FileText, color: "#84cc16" },
      { type: "delay", label: "Delay", icon: Clock, color: "#a3a3a3" },
      { type: "comment", label: "Comment", icon: StickyNote, color: "#eab308" },
    ],
  },
];

const DRAG_THRESHOLD = 5;

export function FlowchartNodePalette() {
  const activeFlowchart = useFlowchartStore((s) => s.activeFlowchart);
  const updateActive = useFlowchartStore((s) => s.updateActive);
  const { screenToFlowPosition } = useReactFlow();

  // Pointer-event drag state
  const dragInfo = useRef<{
    nodeType: string;
    label: string;
    startX: number;
    startY: number;
    pointerId: number;
    target: HTMLElement;
  } | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [ghostPos, setGhostPos] = useState<{ x: number; y: number } | null>(null);
  const [ghostLabel, setGhostLabel] = useState("");

  const addNodeAt = useCallback(
    (nodeType: string, label: string, position: { x: number; y: number }) => {
      if (!activeFlowchart) return;
      const newId = `${nodeType}_${crypto.randomUUID().slice(0, 8)}`;
      const newNode = {
        id: newId,
        node_type: nodeType,
        label,
        position,
        config: {},
      };
      updateActive({
        nodes: [...(activeFlowchart.nodes ?? []), newNode],
      });
    },
    [activeFlowchart, updateActive],
  );

  const addNodeDefault = useCallback(
    (nodeType: string, label: string) => {
      if (!activeFlowchart) return;
      const count = (activeFlowchart.nodes ?? []).length;
      addNodeAt(nodeType, label, {
        x: 250 + (count % 5) * 40,
        y: 150 + (count % 5) * 40,
      });
    },
    [activeFlowchart, addNodeAt],
  );

  const handlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>, nodeType: string, label: string) => {
      const target = e.currentTarget;
      target.setPointerCapture(e.pointerId);
      dragInfo.current = {
        nodeType,
        label,
        startX: e.clientX,
        startY: e.clientY,
        pointerId: e.pointerId,
        target,
      };
    },
    [],
  );

  // Global pointer move -- show ghost when dragging
  useEffect(() => {
    const onPointerMove = (e: PointerEvent) => {
      if (!dragInfo.current) return;
      const dx = Math.abs(e.clientX - dragInfo.current.startX);
      const dy = Math.abs(e.clientY - dragInfo.current.startY);
      if (dx > DRAG_THRESHOLD || dy > DRAG_THRESHOLD) {
        if (!isDragging) {
          setIsDragging(true);
          setGhostLabel(dragInfo.current.label);
        }
        setGhostPos({ x: e.clientX, y: e.clientY });
      }
    };

    const onPointerUp = (e: PointerEvent) => {
      if (!dragInfo.current) return;
      const { nodeType, label, startX, startY, target, pointerId } = dragInfo.current;
      dragInfo.current = null;
      setIsDragging(false);
      setGhostPos(null);

      try { target.releasePointerCapture(pointerId); } catch { /* already released */ }

      const dx = Math.abs(e.clientX - startX);
      const dy = Math.abs(e.clientY - startY);

      if (dx < DRAG_THRESHOLD && dy < DRAG_THRESHOLD) {
        addNodeDefault(nodeType, label);
        return;
      }

      // Drag -- check if released over the flow canvas
      const elementUnder = document.elementFromPoint(e.clientX, e.clientY);
      const isOverFlow = elementUnder?.closest(".react-flow");

      if (isOverFlow) {
        const position = screenToFlowPosition({
          x: e.clientX,
          y: e.clientY,
        });
        addNodeAt(nodeType, label, position);
      } else {
        // Released outside canvas -- add at default position
        addNodeDefault(nodeType, label);
      }
    };

    document.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerup", onPointerUp);
    return () => {
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerup", onPointerUp);
    };
  }, [isDragging, addNodeAt, addNodeDefault, screenToFlowPosition]);

  return (
    <div className="p-2">
      {categories.map((cat) => (
        <div key={cat.label} className="mb-3">
          <h4 className="text-xs font-medium text-[var(--text-muted)] uppercase tracking-wider mb-1.5 px-1">
            {cat.label}
          </h4>
          <div className="grid grid-cols-2 gap-1">
            {cat.items.map(({ type, label, icon: Icon, color }) => (
              <div
                key={type}
                role="button"
                tabIndex={0}
                onPointerDown={(e) => handlePointerDown(e, type, label)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    addNodeDefault(type, label);
                  }
                }}
                className="flex items-center gap-1.5 px-2 py-1.5 rounded border border-[var(--border)] cursor-pointer hover:bg-[var(--bg-hover)] focus:ring-2 focus:ring-[var(--accent)] focus:outline-none transition-colors select-none touch-none"
                aria-label={`Add ${label} node`}
              >
                <Icon size={14} style={{ color }} />
                <span className="text-xs text-[var(--text-secondary)] truncate">
                  {label}
                </span>
              </div>
            ))}
          </div>
        </div>
      ))}

      <p className="text-[10px] text-[var(--text-muted)] px-1 mt-2">
        Click or drag to add nodes to the canvas.
      </p>

      {/* Drag ghost */}
      {isDragging && ghostPos && (
        <div
          className="fixed pointer-events-none z-50 px-3 py-1.5 rounded border border-[var(--accent)] bg-[var(--bg-primary)] text-xs text-[var(--text-primary)] shadow-lg opacity-80"
          style={{
            left: ghostPos.x,
            top: ghostPos.y,
            transform: "translate(-50%, -50%)",
          }}
        >
          {ghostLabel}
        </div>
      )}
    </div>
  );
}
