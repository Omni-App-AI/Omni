import { Shuffle } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const TransformNode = createSimpleNode(
  Shuffle,
  "#ec4899",
  (config) => (config.transform_type as string) ?? "",
);
