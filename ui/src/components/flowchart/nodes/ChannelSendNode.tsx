import { MessageSquare } from "lucide-react";
import { createSimpleNode } from "./createSimpleNode";

export const ChannelSendNode = createSimpleNode(
  MessageSquare,
  "#14b8a6",
  (config) => (config.channel_id as string) ?? "",
);
