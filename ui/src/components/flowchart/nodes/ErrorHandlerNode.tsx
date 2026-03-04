import { ShieldAlert } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const ErrorHandlerNode = createSimpleNode(
  ShieldAlert,
  "#f97316",
  () => "Error fallback",
);
