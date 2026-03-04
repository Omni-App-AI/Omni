import { ShieldCheck, Users, HelpCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { TRUST_LEVELS, type TrustLevel } from "@/lib/constants";
import { cn } from "@/lib/utils";

interface TrustBadgeProps {
  level: TrustLevel;
  showLabel?: boolean;
}

const icons = {
  verified: ShieldCheck,
  community: Users,
  unverified: HelpCircle,
};

export function TrustBadge({ level, showLabel = false }: TrustBadgeProps) {
  const config = TRUST_LEVELS[level];
  const Icon = icons[level];

  return (
    <Badge
      variant={level === "verified" ? "success" : level === "community" ? "secondary" : "outline"}
      className={cn("gap-1", !showLabel && "px-1.5")}
    >
      <Icon className="h-3 w-3" />
      {showLabel && config.label}
    </Badge>
  );
}
