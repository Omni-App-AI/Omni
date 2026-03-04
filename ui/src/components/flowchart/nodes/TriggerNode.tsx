import type { NodeProps } from "@xyflow/react";
import { Zap } from "lucide-react";
import { BaseNode } from "./BaseNode";

export function TriggerNode(props: NodeProps) {
  return (
    <BaseNode
      nodeProps={props}
      icon={Zap}
      color="#22c55e"
      handles={{ inputs: [] }}
    >
      <span className="truncate">Entry point</span>
    </BaseNode>
  );
}
