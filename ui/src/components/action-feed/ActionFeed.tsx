import { useEffect, useRef } from "react";
import { useEventStore } from "../../stores/eventStore";
import { ActionEntry } from "./ActionEntry";

export function ActionFeed() {
  const events = useEventStore((s) => s.events);
  const clearEvents = useEventStore((s) => s.clearEvents);
  const listRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to top when new events arrive (newest first)
  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = 0;
    }
  }, [events.length]);

  return (
    <div className="flex flex-col h-full bg-[var(--bg-primary)]">
      <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--border)]">
        <h2 className="text-sm font-semibold text-[var(--text-primary)]">
          Activity Feed
        </h2>
        <button
          onClick={clearEvents}
          className="text-xs px-2 py-1 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)] transition-colors"
        >
          Clear
        </button>
      </div>

      <div ref={listRef} className="flex-1 overflow-y-auto">
        {events.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <p className="text-sm text-[var(--text-muted)]">
              No activity yet
            </p>
          </div>
        ) : (
          <div className="divide-y divide-[var(--border)]">
            {events.map((event) => (
              <ActionEntry key={event.id} event={event} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
