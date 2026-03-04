import type { NodeProps } from "@xyflow/react";
import { Workflow } from "lucide-react";
import { BaseNode } from "./BaseNode";

export function SubFlowNode(props: NodeProps) {
  const config = (props.data as Record<string, unknown>).config as Record<string, unknown> | undefined;

  return (
    <BaseNode nodeProps={props} icon={Workflow} color="#0ea5e9">
      <span className="truncate">
        {(config?.flowchart_id as string) || "No flowchart selected"}
      </span>
    </BaseNode>
  );
}
