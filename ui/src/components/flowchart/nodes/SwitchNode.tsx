import { useMemo } from "react";
import type { NodeProps } from "@xyflow/react";
import { ListTree } from "lucide-react";
import { BaseNode } from "./BaseNode";

export function SwitchNode(props: NodeProps) {
  const config = (props.data as Record<string, unknown>).config as Record<string, unknown> | undefined;

  const cases = useMemo(() => {
    const raw = config?.cases;
    if (Array.isArray(raw)) {
      return raw as Array<{ value: string; label?: string }>;
    }
    return [];
  }, [config?.cases]);

  // Build output handles from cases + default
  const outputs = useMemo(() => {
    const handles = cases.map((c, i) => ({
      id: `case_${i}`,
      label: c.label || String(c.value),
    }));
    handles.push({ id: "default", label: "Default" });
    return handles;
  }, [cases]);

  return (
    <BaseNode
      nodeProps={props}
      icon={ListTree}
      color="#d946ef"
      handles={{ outputs }}
    >
      <span className="truncate">
        {(config?.expression as string) || "No expression"}
      </span>
    </BaseNode>
  );
}
