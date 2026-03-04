import type { NodeProps } from "@xyflow/react";
import { Repeat } from "lucide-react";
import { BaseNode } from "./BaseNode";

export function LoopNode(props: NodeProps) {
  const config = (props.data as Record<string, unknown>).config as Record<string, unknown> | undefined;

  return (
    <BaseNode
      nodeProps={props}
      icon={Repeat}
      color="#8b5cf6"
      handles={{
        outputs: [
          { id: "body", label: "Body" },
          { id: "done", label: "Done" },
        ],
      }}
    >
      <span className="truncate">{(config?.array_path as string) ?? ""}</span>
    </BaseNode>
  );
}
