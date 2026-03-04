"use client";

import { useState } from "react";
import { Download, Copy, Check, Loader2, Terminal, AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";

interface InstallButtonProps {
  extensionId: string;
  hasPassedScan: boolean;
}

export function InstallButton({ extensionId, hasPassedScan }: InstallButtonProps) {
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const cliCommand = `omni ext install ${extensionId}`;
  const downloadUrl = `/api/v1/extensions/${extensionId}/download`;

  const handleDownload = () => {
    setError(null);
    setDownloading(true);

    // Use a hidden anchor to trigger the download via the redirect-based API
    const a = document.createElement("a");
    a.href = downloadUrl;
    a.download = `${extensionId}.wasm`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);

    // Reset after a short delay (we can't track redirect completion)
    setTimeout(() => setDownloading(false), 2000);
  };

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(cliCommand);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback for older browsers
      const el = document.createElement("textarea");
      el.value = cliCommand;
      document.body.appendChild(el);
      el.select();
      document.execCommand("copy");
      document.body.removeChild(el);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="space-y-3">
      <Button
        className="w-full gap-2"
        size="lg"
        onClick={handleDownload}
        disabled={downloading}
      >
        {downloading ? (
          <>
            <Loader2 className="h-5 w-5 animate-spin" />
            Downloading...
          </>
        ) : (
          <>
            <Download className="h-5 w-5" />
            Install Extension
          </>
        )}
      </Button>

      {error && (
        <div className="flex items-start gap-2 text-xs text-destructive">
          <AlertCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
          <span>{error}</span>
        </div>
      )}

      {/* CLI install command */}
      <div>
        <p className="text-[11px] text-muted-foreground/60 mb-1.5 flex items-center gap-1">
          <Terminal className="h-3 w-3" />
          Or install via CLI
        </p>
        <div className="flex items-center gap-1">
          <code className="flex-1 text-[11px] bg-secondary px-2 py-1.5 rounded font-mono truncate">
            {cliCommand}
          </code>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0 shrink-0"
            onClick={handleCopy}
          >
            {copied ? (
              <Check className="h-3 w-3 text-success" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </Button>
        </div>
      </div>

      {!hasPassedScan && (
        <p className="text-[10px] text-warning">
          This version is pending security review. Download at your own risk.
        </p>
      )}
    </div>
  );
}
