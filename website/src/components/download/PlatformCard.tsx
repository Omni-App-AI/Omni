"use client";

import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Download } from "lucide-react";
import { formatBytes, cn } from "@/lib/utils";

interface PlatformAsset {
  url: string;
  signature: string;
  size_bytes: number;
  asset_name: string;
  installer_type?: string;
}

interface PlatformCardProps {
  name: string;
  platformKey: string;
  asset: PlatformAsset | null;
  version: string;
  isDetected: boolean;
  icon: React.ReactNode;
  installerLabel: string;
  secondaryAsset?: {
    label: string;
    platformKey: string;
    asset: PlatformAsset | null;
  };
}

export function PlatformCard({
  name,
  platformKey,
  asset,
  version,
  isDetected,
  icon,
  installerLabel,
  secondaryAsset,
}: PlatformCardProps) {
  const handleDownload = (dlPlatform: string, dlUrl: string) => {
    window.open(dlUrl, "_blank", "noopener");
    // Fire-and-forget download tracking
    fetch(
      `/api/v1/releases/latest?download=true&platform=${dlPlatform}`,
    ).catch(() => {});
  };

  return (
    <Card
      className={cn(
        "relative overflow-hidden",
        isDetected && "border-primary/30 glow-sm",
      )}
    >
      {isDetected && (
        <div className="absolute top-3 right-3">
          <Badge className="text-[11px]">Recommended</Badge>
        </div>
      )}

      <div className="p-6">
        <div className="flex items-center gap-3 mb-4">
          <div className="text-muted-foreground">{icon}</div>
          <div>
            <h3 className="font-medium text-[15px]">{name}</h3>
            <p className="text-xs text-muted-foreground">{installerLabel}</p>
          </div>
        </div>

        {asset ? (
          <>
            <div className="flex items-baseline gap-2 mb-4">
              <span className="text-xs font-mono text-muted-foreground">
                v{version}
              </span>
              <span className="text-xs text-muted-foreground/60">
                {formatBytes(asset.size_bytes)}
              </span>
            </div>

            <Button
              variant={isDetected ? "default" : "outline"}
              size="lg"
              className="w-full"
              onClick={() => handleDownload(platformKey, asset.url)}
            >
              <Download className="h-4 w-4" />
              Download
            </Button>

            {secondaryAsset?.asset && (
              <button
                onClick={() =>
                  handleDownload(
                    secondaryAsset.platformKey,
                    secondaryAsset.asset!.url,
                  )
                }
                className="mt-2 w-full text-center text-xs text-muted-foreground hover:text-foreground transition-colors py-1"
              >
                {secondaryAsset.label} (
                {formatBytes(secondaryAsset.asset.size_bytes)})
              </button>
            )}
          </>
        ) : (
          <div className="text-sm text-muted-foreground/60 py-3">
            Coming soon
          </div>
        )}
      </div>
    </Card>
  );
}
