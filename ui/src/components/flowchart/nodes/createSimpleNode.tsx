import type { NodeProps } from "@xyflow/react";
import type { ElementType } from "react";
import { BaseNode } from "./BaseNode";

type ConfigExtractor = (config: Record<string, unknown>) => string;

/**
 * Factory for creating simple flowchart node components that follow the
 * standard pattern: extract config fields → display preview text.
 */
export function createSimpleNode(
  icon: ElementType,
  color: string,
  getPreview: ConfigExtractor,
) {
  return function SimpleNode(props: NodeProps) {
    const config = (props.data as Record<string, unknown>).config as
      | Record<string, unknown>
      | undefined;
    const preview = config ? getPreview(config) : "";

    return (
      <BaseNode nodeProps={props} icon={icon} color={color}>
        <span className="truncate">{preview}</span>
      </BaseNode>
    );
  };
}
