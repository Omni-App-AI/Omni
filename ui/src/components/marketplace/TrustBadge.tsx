import { ShieldCheck, Users, ShieldAlert } from "lucide-react";

interface TrustBadgeProps {
  level: string;
  size?: number;
}

const TRUST_CONFIG: Record<string, { label: string; color: string; Icon: React.ElementType }> = {
  verified: { label: "Verified", color: "#3b82f6", Icon: ShieldCheck },
  community: { label: "Community", color: "#94a3b8", Icon: Users },
  unverified: { label: "Unverified", color: "#eab308", Icon: ShieldAlert },
};

export function TrustBadge({ level, size = 14 }: TrustBadgeProps) {
  const config = TRUST_CONFIG[level] ?? TRUST_CONFIG.unverified;
  const { label, color, Icon } = config;

  return (
    <span
      className="inline-flex items-center gap-1 text-xs font-medium px-1.5 py-0.5 rounded"
      style={{
        color,
        backgroundColor: `color-mix(in srgb, ${color} 12%, transparent)`,
      }}
    >
      <Icon size={size} />
      {label}
    </span>
  );
}
