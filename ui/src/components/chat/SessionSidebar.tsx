import { Plus, MessageSquare } from "lucide-react";
import { useChatStore } from "../../stores/chatStore";

export function SessionSidebar() {
  const sessions = useChatStore((s) => s.sessions);
  const activeSessionId = useChatStore((s) => s.activeSessionId);
  const selectSession = useChatStore((s) => s.selectSession);
  const newSession = useChatStore((s) => s.newSession);

  const formatDate = (iso: string): string => {
    try {
      const d = new Date(iso);
      return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
    } catch {
      return "";
    }
  };

  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-[var(--border)] bg-[var(--bg-secondary)]">
      <div className="flex items-center justify-between border-b border-[var(--border)] px-3 py-3">
        <h2 className="text-sm font-semibold text-[var(--text-primary)]">Sessions</h2>
        <button
          onClick={() => newSession()}
          className="flex items-center gap-1 rounded-md bg-[var(--accent)] px-2 py-1 text-xs font-medium text-white transition-opacity hover:opacity-90"
          aria-label="New chat session"
        >
          <Plus size={14} />
          New Chat
        </button>
      </div>

      <div className="flex-1 overflow-y-auto py-1">
        {sessions.length === 0 && (
          <p className="px-3 py-4 text-xs text-[var(--text-muted)] text-center">
            No sessions yet
          </p>
        )}

        {sessions.map((session) => {
          const isActive = session.id === activeSessionId;
          const truncatedId = session.id.length > 12
            ? `${session.id.slice(0, 8)}...`
            : session.id;

          return (
            <button
              key={session.id}
              onClick={() => selectSession(session.id)}
              className={`flex w-full items-center gap-2 px-3 py-2 text-left text-sm transition-colors ${
                isActive
                  ? "bg-[var(--accent)]/10 text-[var(--accent)] border-r-2 border-[var(--accent)]"
                  : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)]"
              }`}
              title={session.id}
            >
              <MessageSquare size={14} className="shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="truncate font-mono text-xs">{truncatedId}</p>
                <p className="text-[10px] text-[var(--text-muted)]">
                  {formatDate(session.updated_at)}
                </p>
              </div>
            </button>
          );
        })}
      </div>
    </aside>
  );
}
