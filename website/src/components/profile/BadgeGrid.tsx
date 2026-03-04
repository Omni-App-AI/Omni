import {
  MessageSquare,
  Package,
  CheckCircle,
  TrendingUp,
  Star,
  Award,
  Shield,
  Eye,
} from "lucide-react";
import { BADGE_DEFINITIONS } from "@/lib/constants";

const iconMap: Record<string, React.ComponentType<{ className?: string }>> = {
  MessageSquare,
  Package,
  CheckCircle,
  TrendingUp,
  Star,
  Award,
  Shield,
  Eye,
};

interface BadgeGridProps {
  badges: { badge_id: string; earned_at: string }[];
}

export function BadgeGrid({ badges }: BadgeGridProps) {
  if (badges.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">No badges earned yet.</p>
    );
  }

  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
      {badges.map((badge) => {
        const def = BADGE_DEFINITIONS[badge.badge_id];
        if (!def) return null;

        const Icon = iconMap[def.icon] || Award;

        return (
          <div
            key={badge.badge_id}
            className="flex items-center gap-3 p-3 border border-border/50 rounded-lg bg-card/30"
          >
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary/10">
              <Icon className="h-4 w-4 text-primary" />
            </div>
            <div className="min-w-0">
              <p className="text-xs font-medium truncate">{def.name}</p>
              <p className="text-[10px] text-muted-foreground truncate">
                {def.description}
              </p>
            </div>
          </div>
        );
      })}
    </div>
  );
}
