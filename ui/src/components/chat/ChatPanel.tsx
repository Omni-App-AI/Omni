import { useEffect, useRef, useCallback, useState } from "react";
import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { useChatStore } from "../../stores/chatStore";
import { useOmniEvent } from "../../hooks/useOmniEvents";
import { SessionSidebar } from "./SessionSidebar";
import { MessageBubble } from "./MessageBubble";
import { InputBar } from "./InputBar";
import { StreamingIndicator } from "./StreamingIndicator";

interface LlmChunkPayload {
  chunk: string;
}

interface LlmCompletePayload {
  text: string;
}

interface LlmErrorPayload {
  sessionId: string;
  error: string;
}

export function ChatPanel() {
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const messages = useChatStore((s) => s.messages);
  const isStreaming = useChatStore((s) => s.isStreaming);
  const streamBuffer = useChatStore((s) => s.streamBuffer);
  const activeSessionId = useChatStore((s) => s.activeSessionId);
  const sessions = useChatStore((s) => s.sessions);
  const loadSessions = useChatStore((s) => s.loadSessions);
  const send = useChatStore((s) => s.send);
  const appendChunk = useChatStore((s) => s.appendChunk);
  const completeStream = useChatStore((s) => s.completeStream);
  const cancelStream = useChatStore((s) => s.cancelStream);

  // Load sessions on mount
  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  // Auto-scroll to bottom on new messages or stream updates
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamBuffer]);

  // Listen for LLM streaming events
  const handleChunk = useCallback(
    (payload: LlmChunkPayload) => {
      appendChunk(payload.chunk);
    },
    [appendChunk],
  );

  const handleComplete = useCallback(
    (_payload: LlmCompletePayload) => {
      completeStream(useChatStore.getState().streamBuffer);
    },
    [completeStream],
  );

  const handleError = useCallback(
    (payload: LlmErrorPayload) => {
      cancelStream(payload.error);
    },
    [cancelStream],
  );

  useOmniEvent<LlmChunkPayload>("omni:llm-chunk", handleChunk);
  useOmniEvent<LlmCompletePayload>("omni:llm-complete", handleComplete);
  useOmniEvent<LlmErrorPayload>("omni:llm-error", handleError);

  const handleSend = useCallback(
    (content: string) => {
      send(content);
    },
    [send],
  );

  return (
    <div className="flex h-full bg-[var(--bg-primary)]">
      {/* Collapsible session sidebar */}
      {sidebarOpen && <SessionSidebar />}

      {/* Main chat area */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* Header */}
        <div className="flex items-center gap-2 border-b border-[var(--border)] bg-[var(--bg-secondary)] px-4 py-2">
          <button
            onClick={() => setSidebarOpen((prev) => !prev)}
            className="rounded p-1 text-[var(--text-muted)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)] transition-colors"
            aria-label={sidebarOpen ? "Close sidebar" : "Open sidebar"}
          >
            {sidebarOpen ? <PanelLeftClose size={18} /> : <PanelLeftOpen size={18} />}
          </button>
          <h2 className="text-sm font-medium text-[var(--text-primary)] truncate">
            {activeSessionId
              ? (() => {
                  const s = sessions.find((s) => s.id === activeSessionId);
                  if (s?.metadata) {
                    try {
                      const meta = JSON.parse(s.metadata);
                      if (meta.title) return meta.title;
                    } catch { /* ignore */ }
                  }
                  return "New Chat";
                })()
              : "Select or start a session"}
          </h2>
          {isStreaming && (
            <span className="ml-auto text-xs text-[var(--accent)] animate-pulse">
              Streaming...
            </span>
          )}
        </div>

        {/* Messages list */}
        <div className="flex-1 overflow-y-auto py-4">
          {!activeSessionId && (
            <div className="flex h-full items-center justify-center">
              <p className="text-sm text-[var(--text-muted)]">
                Create a new session or select one from the sidebar to get started.
              </p>
            </div>
          )}

          {activeSessionId && messages.length === 0 && !isStreaming && (
            <div className="flex h-full items-center justify-center">
              <p className="text-sm text-[var(--text-muted)]">
                No messages yet. Send a message to begin.
              </p>
            </div>
          )}

          {messages.map((msg, idx) => (
            <MessageBubble
              key={`${idx}-${msg.role}`}
              role={msg.role}
              content={msg.content}
            />
          ))}

          {/* Streaming bubble for in-progress assistant response */}
          {isStreaming && streamBuffer && (
            <MessageBubble
              role="assistant"
              content={streamBuffer}
              isStreaming
            />
          )}

          {isStreaming && !streamBuffer && <StreamingIndicator />}

          <div ref={messagesEndRef} />
        </div>

        {/* Input bar */}
        <InputBar onSend={handleSend} disabled={isStreaming || !activeSessionId} />
      </div>
    </div>
  );
}
