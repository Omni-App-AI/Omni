import { Clock } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const DelayNode = createSimpleNode(
  Clock,
  "#a3a3a3",
  (config) => `${(config.milliseconds as number) ?? 1000}ms`,
);
