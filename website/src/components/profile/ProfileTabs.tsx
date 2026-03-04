"use client";

import { cn } from "@/lib/utils";

interface ProfileTabsProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
  extensionCount: number;
  postCount: number;
  reviewCount: number;
}

const tabs = [
  { id: "overview", label: "Overview" },
  { id: "extensions", label: "Extensions" },
  { id: "posts", label: "Posts" },
  { id: "reviews", label: "Reviews" },
];

export function ProfileTabs({ activeTab, onTabChange, extensionCount, postCount, reviewCount }: ProfileTabsProps) {
  const counts: Record<string, number> = {
    extensions: extensionCount,
    posts: postCount,
    reviews: reviewCount,
  };

  return (
    <div className="flex border-b border-border/50">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onTabChange(tab.id)}
          className={cn(
            "px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px",
            activeTab === tab.id
              ? "border-primary text-foreground"
              : "border-transparent text-muted-foreground hover:text-foreground hover:border-border",
          )}
        >
          {tab.label}
          {counts[tab.id] !== undefined && (
            <span className="ml-1.5 text-xs text-muted-foreground/60">
              {counts[tab.id]}
            </span>
          )}
        </button>
      ))}
    </div>
  );
}
