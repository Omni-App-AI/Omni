import { REPUTATION_TIERS } from "@/lib/constants";
import { cn } from "@/lib/utils";

interface ReputationBadgeProps {
  reputation: number;
  showLabel?: boolean;
  className?: string;
}

export function ReputationBadge({ reputation, showLabel = true, className }: ReputationBadgeProps) {
  const tier = REPUTATION_TIERS.find((t) => reputation >= t.min) || REPUTATION_TIERS[REPUTATION_TIERS.length - 1];

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium",
        tier.bgColor,
        tier.color,
        className,
      )}
    >
      <span className="tabular-nums">{reputation}</span>
      {showLabel && <span className="text-[10px] opacity-70">{tier.label}</span>}
    </span>
  );
}
