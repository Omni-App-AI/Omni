"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { LayoutDashboard, Package, Settings, Key, MessageSquare, LogOut, ChevronRight, Shield, FileText } from "lucide-react";
import { createClient } from "@/lib/supabase/client";
import { useRouter } from "next/navigation";
import { cn } from "@/lib/utils";

const links = [
  { href: "/dashboard", label: "Overview", icon: LayoutDashboard },
  { href: "/dashboard/extensions", label: "My Extensions", icon: Package },
  { href: "/dashboard/api-keys", label: "API Keys", icon: Key },
  { href: "/dashboard/posts", label: "My Posts", icon: MessageSquare },
  { href: "/dashboard/settings", label: "Settings", icon: Settings },
];

interface DashboardSidebarProps {
  isModerator?: boolean;
}

export function DashboardSidebar({ isModerator }: DashboardSidebarProps) {
  const pathname = usePathname();
  const router = useRouter();

  const handleSignOut = async () => {
    const supabase = createClient();
    await supabase.auth.signOut();
    router.push("/");
    router.refresh();
  };

  return (
    <aside className="w-60 shrink-0 border-r border-border/50 min-h-[calc(100vh-3.5rem)] bg-card/30">
      <div className="p-4 pt-6">
        <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 px-3 mb-3">
          Dashboard
        </p>
        <nav className="space-y-0.5">
          {links.map((link) => {
            const isActive =
              link.href === "/dashboard"
                ? pathname === "/dashboard"
                : pathname.startsWith(link.href);

            return (
              <Link
                key={link.href}
                href={link.href}
                className={cn(
                  "group flex items-center gap-3 px-3 py-2 rounded-md text-[13px] font-medium transition-all duration-150",
                  isActive
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:text-foreground hover:bg-secondary/50",
                )}
              >
                <link.icon className={cn("h-4 w-4", isActive && "text-primary")} />
                <span className="flex-1">{link.label}</span>
                {isActive && <ChevronRight className="h-3 w-3 text-primary/50" />}
              </Link>
            );
          })}
        </nav>

        {isModerator && (
          <div className="mt-6 pt-6 border-t border-border/50">
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 px-3 mb-3">
              Moderation
            </p>
            <nav className="space-y-0.5">
              {(() => {
                const isActive = pathname.startsWith("/admin/moderation");
                return (
                  <Link
                    href="/admin/moderation"
                    className={cn(
                      "group flex items-center gap-3 px-3 py-2 rounded-md text-[13px] font-medium transition-all duration-150",
                      isActive
                        ? "bg-primary/10 text-primary"
                        : "text-muted-foreground hover:text-foreground hover:bg-secondary/50",
                    )}
                  >
                    <Shield className={cn("h-4 w-4", isActive && "text-primary")} />
                    <span className="flex-1">Mod Dashboard</span>
                    {isActive && <ChevronRight className="h-3 w-3 text-primary/50" />}
                  </Link>
                );
              })()}
              {(() => {
                const isActive = pathname.startsWith("/admin/blog");
                return (
                  <Link
                    href="/admin/blog"
                    className={cn(
                      "group flex items-center gap-3 px-3 py-2 rounded-md text-[13px] font-medium transition-all duration-150",
                      isActive
                        ? "bg-primary/10 text-primary"
                        : "text-muted-foreground hover:text-foreground hover:bg-secondary/50",
                    )}
                  >
                    <FileText className={cn("h-4 w-4", isActive && "text-primary")} />
                    <span className="flex-1">Blog Posts</span>
                    {isActive && <ChevronRight className="h-3 w-3 text-primary/50" />}
                  </Link>
                );
              })()}
            </nav>
          </div>
        )}

        <div className="mt-6 pt-6 border-t border-border/50">
          <button
            onClick={handleSignOut}
            className="flex items-center gap-3 px-3 py-2 rounded-md text-[13px] font-medium text-muted-foreground hover:text-foreground hover:bg-secondary/50 w-full transition-colors"
          >
            <LogOut className="h-4 w-4" />
            Sign out
          </button>
        </div>
      </div>
    </aside>
  );
}
