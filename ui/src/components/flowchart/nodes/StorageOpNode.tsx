import { Database } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const StorageOpNode = createSimpleNode(Database, "#64748b", (config) => {
  const operation = (config.operation as string) ?? "get";
  const key = (config.key_template as string) ?? "...";
  return `${operation}: ${key}`;
});
