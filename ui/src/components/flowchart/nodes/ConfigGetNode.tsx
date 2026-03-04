import { Settings } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const ConfigGetNode = createSimpleNode(
  Settings,
  "#78716c",
  (config) => (config.key as string) ?? "",
);
