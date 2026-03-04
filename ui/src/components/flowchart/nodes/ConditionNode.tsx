import type { NodeProps } from "@xyflow/react";
import { GitBranch } from "lucide-react";
import { BaseNode } from "./BaseNode";

export function ConditionNode(props: NodeProps) {
  const config = (props.data as Record<string, unknown>).config as Record<string, unknown> | undefined;

  return (
    <BaseNode
      nodeProps={props}
      icon={GitBranch}
      color="#f59e0b"
      handles={{
        outputs: [
          { id: "true", label: "True" },
          { id: "false", label: "False" },
        ],
      }}
    >
      <span className="truncate">{(config?.expression as string) ?? ""}</span>
    </BaseNode>
  );
}
