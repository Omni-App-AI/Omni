import { Variable } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const SetVariableNode = createSimpleNode(
  Variable,
  "#06b6d4",
  (config) => `$var.${(config.variable_name as string) ?? "..."}`,
);
