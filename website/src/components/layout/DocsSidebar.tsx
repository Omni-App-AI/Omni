"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Search } from "lucide-react";
import { cn } from "@/lib/utils";

const sections = [
  {
    heading: "Getting Started",
    links: [
      { href: "/docs", label: "Overview" },
      { href: "/docs/getting-started", label: "Getting Started" },
      { href: "/docs/configuration", label: "Configuration" },
      { href: "/docs/providers", label: "LLM Providers" },
      { href: "/docs/channels", label: "Channels" },
    ],
  },
  {
    heading: "Core Concepts",
    links: [
      { href: "/docs/security", label: "Security & Permissions" },
      { href: "/docs/tools", label: "Native Tools" },
      { href: "/docs/flowcharts", label: "Flowchart Builder" },
      { href: "/docs/hooks", label: "Hook System" },
      { href: "/docs/architecture", label: "Architecture" },
    ],
  },
  {
    heading: "Developers",
    links: [
      { href: "/docs/sdk", label: "SDK Reference" },
      { href: "/docs/publishing", label: "Publishing Guide" },
      { href: "/docs/building", label: "Building from Source" },
    ],
  },
  {
    heading: "Resources",
    links: [
      { href: "/docs/changelog", label: "Changelog" },
    ],
  },
];

export function DocsSidebar() {
  const pathname = usePathname();

  return (
    <aside className="w-56 shrink-0 hidden md:block">
      <nav className="sticky top-20 space-y-5">
        <button
          onClick={() => {
            document.dispatchEvent(
              new KeyboardEvent("keydown", { key: "k", metaKey: true }),
            );
          }}
          className="flex items-center gap-2 w-full px-3 py-2 text-[13px] text-muted-foreground hover:text-foreground transition-colors rounded-md border border-border/50 hover:border-border bg-secondary/20 hover:bg-secondary/40 mb-2"
        >
          <Search className="h-3.5 w-3.5" />
          <span>Search</span>
          <kbd className="ml-auto text-[10px] font-mono text-muted-foreground/40">
            ⌘K
          </kbd>
        </button>

        {sections.map((section) => (
          <div key={section.heading}>
            <p className="text-xs font-mono text-muted-foreground/60 uppercase tracking-wider mb-2 px-3">
              {section.heading}
            </p>
            <div className="space-y-0.5">
              {section.links.map((link) => {
                const isActive =
                  link.href === "/docs"
                    ? pathname === "/docs"
                    : pathname.startsWith(link.href);

                return (
                  <Link
                    key={link.href}
                    href={link.href}
                    className={cn(
                      "block px-3 py-1.5 text-[13px] transition-colors border-l-2",
                      isActive
                        ? "border-primary text-foreground font-medium"
                        : "border-transparent text-muted-foreground hover:text-foreground hover:border-border",
                    )}
                  >
                    {link.label}
                  </Link>
                );
              })}
            </div>
          </div>
        ))}
      </nav>
    </aside>
  );
}
