"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Megaphone, HelpCircle, Sparkles, Lightbulb, Code2, MessageCircle } from "lucide-react";
import { FORUM_CATEGORIES } from "@/lib/constants";
import { cn } from "@/lib/utils";

const iconMap: Record<string, React.ComponentType<{ className?: string }>> = {
  Megaphone,
  HelpCircle,
  Sparkles,
  Lightbulb,
  Code2,
  MessageCircle,
};

export function ForumSidebar() {
  const pathname = usePathname();

  return (
    <aside className="w-56 shrink-0 hidden lg:block">
      <div className="sticky top-20 space-y-4">
        <nav className="space-y-0.5">
          <p className="text-[11px] font-mono text-muted-foreground/60 uppercase tracking-wider mb-3 px-3">
            Categories
          </p>
          <Link
            href="/community"
            className={cn(
              "flex items-center gap-2 px-3 py-1.5 text-[13px] transition-colors rounded-md",
              pathname === "/community"
                ? "text-foreground font-medium bg-secondary/50"
                : "text-muted-foreground hover:text-foreground hover:bg-secondary/30",
            )}
          >
            All Posts
          </Link>
          {FORUM_CATEGORIES.map((cat) => {
            const Icon = iconMap[cat.icon] || MessageCircle;
            const isActive = pathname === `/community/${cat.id}`;

            return (
              <Link
                key={cat.id}
                href={`/community/${cat.id}`}
                className={cn(
                  "flex items-center gap-2 px-3 py-1.5 text-[13px] transition-colors rounded-md",
                  isActive
                    ? "text-foreground font-medium bg-secondary/50"
                    : "text-muted-foreground hover:text-foreground hover:bg-secondary/30",
                )}
              >
                <Icon className="h-3.5 w-3.5" />
                {cat.name}
              </Link>
            );
          })}
        </nav>
      </div>
    </aside>
  );
}
