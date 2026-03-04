import { Shield, AlertTriangle, Info } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { PERMISSIONS_DISPLAY } from "@/lib/constants";
import { cn } from "@/lib/utils";

interface Permission {
  capability: string;
  scope?: Record<string, unknown>;
  reason?: string;
}

interface PermissionBadgesProps {
  permissions: Permission[];
  showReasons?: boolean;
}

const severityIcons = {
  low: Info,
  medium: Shield,
  high: AlertTriangle,
};

const severityVariants = {
  low: "secondary" as const,
  medium: "warning" as const,
  high: "destructive" as const,
};

export function PermissionBadges({ permissions, showReasons = false }: PermissionBadgesProps) {
  if (permissions.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">No special permissions required.</p>
    );
  }

  return (
    <div className={cn(showReasons ? "space-y-3" : "flex flex-wrap gap-2")}>
      {permissions.map((perm) => {
        const display = PERMISSIONS_DISPLAY[perm.capability] || {
          label: perm.capability,
          severity: "medium" as const,
        };
        const Icon = severityIcons[display.severity];

        if (showReasons) {
          return (
            <div key={perm.capability} className="flex items-start gap-3">
              <div className={cn(
                "flex h-8 w-8 shrink-0 items-center justify-center rounded-lg",
                display.severity === "high" ? "bg-destructive/10" :
                display.severity === "medium" ? "bg-warning/10" : "bg-secondary",
              )}>
                <Icon className={cn(
                  "h-4 w-4",
                  display.severity === "high" ? "text-destructive" :
                  display.severity === "medium" ? "text-warning" : "text-muted-foreground",
                )} />
              </div>
              <div>
                <p className="text-sm font-medium">{display.label}</p>
                {perm.reason && (
                  <p className="text-xs text-muted-foreground mt-0.5">{perm.reason}</p>
                )}
              </div>
            </div>
          );
        }

        return (
          <Badge key={perm.capability} variant={severityVariants[display.severity]}>
            <Icon className="h-3 w-3 mr-1" />
            {display.label}
          </Badge>
        );
      })}
    </div>
  );
}
