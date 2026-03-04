import { FileText } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const LogNode = createSimpleNode(FileText, "#84cc16", (config) => {
  const level = (config.level as string) ?? "info";
  const message = (config.message_template as string) ?? "";
  return `[${level}] ${message}`;
});
