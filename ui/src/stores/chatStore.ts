import { create } from "zustand";
import {
  listSessions,
  createSession,
  getSessionMessages,
  sendMessage,
  type SessionDto,
  type MessageDto,
} from "../lib/tauri-commands";

interface ChatState {
  sessions: SessionDto[];
  activeSessionId: string | null;
  messages: MessageDto[];
  isStreaming: boolean;
  streamBuffer: string;

  loadSessions: () => Promise<void>;
  selectSession: (id: string) => Promise<void>;
  newSession: () => Promise<void>;
  send: (content: string) => Promise<void>;
  appendChunk: (chunk: string) => void;
  completeStream: (fullText: string) => void;
  cancelStream: (errorMsg?: string) => void;
}

export const useChatStore = create<ChatState>((set, get) => ({
  sessions: [],
  activeSessionId: null,
  messages: [],
  isStreaming: false,
  streamBuffer: "",

  loadSessions: async () => {
    const sessions = await listSessions();
    set({ sessions });
  },

  selectSession: async (id: string) => {
    set({ activeSessionId: id, isStreaming: false, streamBuffer: "" });
    const messages = await getSessionMessages(id);
    set({ messages });
  },

  newSession: async () => {
    const id = await createSession();
    await get().loadSessions();
    await get().selectSession(id);
  },

  send: async (content: string) => {
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    // Add user message locally
    const userMsg: MessageDto = { role: "user", content, tool_calls: null };
    set((s) => ({
      messages: [...s.messages, userMsg],
      isStreaming: true,
      streamBuffer: "",
    }));

    await sendMessage(activeSessionId, content);
  },

  appendChunk: (chunk: string) => {
    set((s) => ({ streamBuffer: s.streamBuffer + chunk }));
  },

  completeStream: (fullText: string) => {
    const assistantMsg: MessageDto = {
      role: "assistant",
      content: fullText,
      tool_calls: null,
    };
    set((s) => ({
      messages: [...s.messages, assistantMsg],
      isStreaming: false,
      streamBuffer: "",
    }));
  },

  cancelStream: (errorMsg?: string) => {
    if (errorMsg) {
      const errorMessage: MessageDto = {
        role: "system",
        content: `Error: ${errorMsg}`,
        tool_calls: null,
      };
      set((s) => ({
        messages: [...s.messages, errorMessage],
        isStreaming: false,
        streamBuffer: "",
      }));
    } else {
      set({ isStreaming: false, streamBuffer: "" });
    }
  },
}));
