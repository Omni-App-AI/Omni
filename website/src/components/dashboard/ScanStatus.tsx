import { ShieldCheck, ShieldAlert, ShieldX, AlertCircle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface ScanStatusProps {
  scanResult: Record<string, unknown>;
  compact?: boolean;
}

const verdictConfig = {
  clean: { icon: ShieldCheck, label: "Clean", color: "text-success", bg: "bg-success/10" },
  suspicious: { icon: ShieldAlert, label: "Suspicious", color: "text-warning", bg: "bg-warning/10" },
  malicious: { icon: ShieldX, label: "Malicious", color: "text-destructive", bg: "bg-destructive/10" },
  error: { icon: AlertCircle, label: "Error", color: "text-muted-foreground", bg: "bg-muted" },
};

export function ScanStatus({ scanResult, compact = false }: ScanStatusProps) {
  const verdict = (scanResult.verdict as string) || "error";
  const config = verdictConfig[verdict as keyof typeof verdictConfig] || verdictConfig.error;
  const Icon = config.icon;
  const overallScore = scanResult.overall_score as number;

  const layers = [
    { label: "Signatures", score: scanResult.signature_score as number | null },
    { label: "Heuristics", score: scanResult.heuristic_score as number | null },
    { label: "AI Review", score: scanResult.ai_score as number | null },
    { label: "Sandbox", score: scanResult.sandbox_score as number | null },
  ];

  if (compact) {
    return (
      <div className="flex items-center gap-2">
        <Icon className={cn("h-4 w-4", config.color)} />
        <span className={cn("text-sm font-medium", config.color)}>{config.label}</span>
        {overallScore != null && (
          <Badge variant="secondary">{overallScore.toFixed(0)}/100</Badge>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <div className={cn("flex h-10 w-10 items-center justify-center rounded-lg", config.bg)}>
          <Icon className={cn("h-5 w-5", config.color)} />
        </div>
        <div>
          <span className={cn("font-semibold", config.color)}>{config.label}</span>
          {overallScore != null && (
            <p className="text-sm text-muted-foreground">
              Overall score: {overallScore.toFixed(1)}/100
            </p>
          )}
        </div>
      </div>

      <div className="space-y-2">
        {layers.map((layer) => (
          <div key={layer.label}>
            <div className="flex justify-between text-sm mb-1">
              <span className="text-muted-foreground">{layer.label}</span>
              <span className={cn(
                "font-medium",
                layer.score == null ? "text-muted-foreground" :
                layer.score >= 80 ? "text-success" :
                layer.score >= 50 ? "text-warning" : "text-destructive",
              )}>
                {layer.score != null ? `${layer.score.toFixed(0)}/100` : "N/A"}
              </span>
            </div>
            <div className="h-1.5 bg-secondary rounded-full overflow-hidden">
              <div
                className={cn(
                  "h-full rounded-full transition-all",
                  layer.score == null ? "bg-muted-foreground" :
                  layer.score >= 80 ? "bg-success" :
                  layer.score >= 50 ? "bg-warning" : "bg-destructive",
                )}
                style={{ width: `${layer.score ?? 0}%` }}
              />
            </div>
          </div>
        ))}
      </div>

      {(scanResult.scan_duration_ms as number) > 0 && (
        <p className="text-xs text-muted-foreground">
          Scanned in {((scanResult.scan_duration_ms as number) / 1000).toFixed(1)}s
        </p>
      )}
    </div>
  );
}
