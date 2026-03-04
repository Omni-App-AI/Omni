import Link from "next/link";
import {
  MessageSquare,
  Star,
  Package,
} from "lucide-react";
import { timeAgo } from "@/lib/utils";

interface ActivityItem {
  type: "post" | "review" | "extension";
  id: string;
  title: string;
  href: string;
  date: string;
  meta?: string;
}

interface ActivityFeedProps {
  items: ActivityItem[];
}

const typeConfig = {
  post: {
    icon: MessageSquare,
    label: "posted",
    color: "text-blue-400",
    bgColor: "bg-blue-400/10",
  },
  review: {
    icon: Star,
    label: "reviewed",
    color: "text-warning",
    bgColor: "bg-warning/10",
  },
  extension: {
    icon: Package,
    label: "published",
    color: "text-success",
    bgColor: "bg-success/10",
  },
};

export function ActivityFeed({ items }: ActivityFeedProps) {
  if (items.length === 0) {
    return (
      <p className="text-sm text-muted-foreground py-4">No activity yet.</p>
    );
  }

  return (
    <div className="space-y-0">
      {items.map((item) => {
        const config = typeConfig[item.type];
        const Icon = config.icon;

        return (
          <div
            key={`${item.type}-${item.id}`}
            className="flex items-start gap-3 py-3 border-b border-border/30 last:border-0"
          >
            <div
              className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-full ${config.bgColor}`}
            >
              <Icon className={`h-3.5 w-3.5 ${config.color}`} />
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-sm">
                <span className="text-muted-foreground">{config.label} </span>
                <Link
                  href={item.href}
                  className="font-medium hover:text-primary transition-colors"
                >
                  {item.title}
                </Link>
              </p>
              <div className="flex items-center gap-2 mt-0.5">
                <span className="text-[10px] text-muted-foreground/60">
                  {timeAgo(item.date)}
                </span>
                {item.meta && (
                  <span className="text-[10px] text-muted-foreground/60">
                    {item.meta}
                  </span>
                )}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
