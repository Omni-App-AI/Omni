import type { NodeProps } from "@xyflow/react";
import { GitMerge } from "lucide-react";
import { BaseNode } from "./BaseNode";

export function MergeNode(props: NodeProps) {
  return (
    <BaseNode
      nodeProps={props}
      icon={GitMerge}
      color="#6366f1"
      handles={{
        inputs: [{ id: "a" }, { id: "b" }, { id: "c" }],
      }}
    >
      <span className="truncate">Merge</span>
    </BaseNode>
  );
}
