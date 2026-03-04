import {
  MessageSquare,
  Activity,
  Shield,
  Puzzle,
  Store,
  ShieldAlert,
  Settings,
  Radio,
  Workflow,
} from "lucide-react";

export type Page =
  | "chat"
  | "action-feed"
  | "permissions"
  | "extensions"
  | "marketplace"
  | "channels"
  | "flowcharts"
  | "guardian"
  | "settings";

interface SidebarProps {
  activePage: Page;
  onNavigate: (page: Page) => void;
}

const navItems: { page: Page; label: string; icon: React.ElementType }[] = [
  { page: "chat", label: "Chat", icon: MessageSquare },
  { page: "action-feed", label: "Activity", icon: Activity },
  { page: "permissions", label: "Permissions", icon: Shield },
  { page: "extensions", label: "Extensions", icon: Puzzle },
  { page: "marketplace", label: "Marketplace", icon: Store },
  { page: "channels", label: "Channels", icon: Radio },
  { page: "flowcharts", label: "Flowcharts", icon: Workflow },
  { page: "guardian", label: "Guardian", icon: ShieldAlert },
  { page: "settings", label: "Settings", icon: Settings },
];

export function Sidebar({ activePage, onNavigate }: SidebarProps) {
  return (
    <aside className="flex-shrink-0 bg-[var(--bg-sidebar)] border-r border-[var(--border)] flex flex-col" style={{ width: 'var(--sidebar-width)' }}>
      <div className="p-4 border-b border-[var(--border)]">
        <h1 className="text-xl font-bold text-[var(--accent)]">Omni</h1>
        <p className="text-xs text-[var(--text-muted)] mt-1">
          AI Agent Platform
        </p>
      </div>

      <nav className="flex-1 py-2">
        {navItems.map(({ page, label, icon: Icon }) => (
          <button
            key={page}
            onClick={() => onNavigate(page)}
            className={`w-full flex items-center gap-3 px-4 py-2.5 text-sm transition-colors ${
              activePage === page
                ? "text-[var(--accent)] border-r-2 border-[var(--accent)]"
                : "text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] hover:text-[var(--text-primary)]"
            }`}
            style={
              activePage === page
                ? { backgroundColor: "color-mix(in srgb, var(--accent) 12%, transparent)" }
                : undefined
            }
          >
            <Icon size={18} />
            <span>{label}</span>
          </button>
        ))}
      </nav>

      <div className="p-4 border-t border-[var(--border)] text-xs text-[var(--text-muted)]">
        v0.1.0
      </div>
    </aside>
  );
}
