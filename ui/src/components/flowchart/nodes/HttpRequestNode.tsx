import { Globe } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const HttpRequestNode = createSimpleNode(Globe, "#3b82f6", (config) => {
  const method = (config.method as string) ?? "GET";
  const url = (config.url as string) ?? "...";
  return `${method} ${url}`;
});
