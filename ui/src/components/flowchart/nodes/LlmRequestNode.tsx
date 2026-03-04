import { Brain } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const LlmRequestNode = createSimpleNode(Brain, "#a855f7", (config) => {
  const prompt = (config.prompt_template as string) ?? "";
  return prompt.length > 40 ? prompt.slice(0, 40) + "..." : prompt;
});
