import type { NodeProps } from "@xyflow/react";
import { CircleDot } from "lucide-react";
import { BaseNode } from "./BaseNode";

export function OutputNode(props: NodeProps) {
  return (
    <BaseNode
      nodeProps={props}
      icon={CircleDot}
      color="#ef4444"
      handles={{ outputs: [] }}
    >
      <span className="truncate">Result</span>
    </BaseNode>
  );
}
