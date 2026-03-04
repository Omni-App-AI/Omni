import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { ElementType, ReactNode } from "react";

interface BaseNodeProps {
  nodeProps: NodeProps;
  icon: ElementType;
  color: string;
  children?: ReactNode;
  handles?: {
    inputs?: Array<{ id?: string; label?: string }>;
    outputs?: Array<{ id?: string; label?: string }>;
  };
}

export function BaseNode({
  nodeProps,
  icon: Icon,
  color,
  children,
  handles,
}: BaseNodeProps) {
  const { data, selected } = nodeProps;
  const label = (data as Record<string, unknown>).label as string;

  const inputs = handles?.inputs ?? [{}];
  const outputs = handles?.outputs ?? [{}];

  return (
    <div
      className={`rounded-lg border-2 min-w-[140px] max-w-[200px] overflow-hidden ${
        selected ? "ring-2 ring-[var(--accent)]" : ""
      }`}
      style={{ borderColor: color, background: "var(--bg-primary)" }}
    >
      {inputs.map((h, i) => (
        <Handle
          key={`in-${h.id ?? i}`}
          type="target"
          position={Position.Top}
          id={h.id}
          style={{
            background: color,
            width: 8,
            height: 8,
            left: inputs.length > 1 ? `${((i + 1) / (inputs.length + 1)) * 100}%` : "50%",
          }}
        />
      ))}

      <div className="flex items-center gap-1.5 px-2 py-1.5 border-b" style={{ borderColor: `${color}33` }}>
        <Icon size={14} style={{ color, flexShrink: 0 }} />
        <span className="text-xs font-medium text-[var(--text-primary)] truncate">
          {label}
        </span>
      </div>

      {children && (
        <div className="px-2 py-1 text-[10px] text-[var(--text-muted)]">
          {children}
        </div>
      )}

      {outputs.map((h, i) => (
        <Handle
          key={`out-${h.id ?? i}`}
          type="source"
          position={Position.Bottom}
          id={h.id}
          style={{
            background: color,
            width: 8,
            height: 8,
            left: outputs.length > 1 ? `${((i + 1) / (outputs.length + 1)) * 100}%` : "50%",
          }}
        />
      ))}

      {/* Handle labels for multi-output nodes */}
      {outputs.length > 1 && (
        <div className="flex justify-between px-1 -mb-1">
          {outputs.map((h) => (
            <span
              key={h.id}
              className="text-[8px] text-[var(--text-muted)]"
            >
              {h.label}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
