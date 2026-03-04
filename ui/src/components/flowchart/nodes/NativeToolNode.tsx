import { Wrench } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const NativeToolNode = createSimpleNode(Wrench, "#f43f5e", (config) => {
  const toolName = (config.tool_name as string) ?? "...";
  return toolName;
});
