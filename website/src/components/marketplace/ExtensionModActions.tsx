"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Shield, Eye, EyeOff, CheckCircle, AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

interface ExtensionModActionsProps {
  extensionId: string;
  moderationStatus: string;
  moderationNote: string | null;
  moderatedAt: string | null;
}

export function ExtensionModActions({
  extensionId,
  moderationStatus,
  moderationNote,
  moderatedAt,
}: ExtensionModActionsProps) {
  const router = useRouter();
  const [acting, setActing] = useState(false);
  const [note, setNote] = useState("");

  const handleAction = async (action: string) => {
    setActing(true);
    try {
      const res = await fetch(`/api/v1/admin/extensions/${extensionId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action, note: note || undefined }),
      });
      if (res.ok) {
        router.refresh();
      }
    } catch {
      // Ignore
    }
    setActing(false);
  };

  const statusBadge: Record<string, { variant: string; label: string }> = {
    active: { variant: "success", label: "Active" },
    under_review: { variant: "warning", label: "Under Review" },
    taken_down: { variant: "destructive", label: "Taken Down" },
  };

  const badge = statusBadge[moderationStatus] || statusBadge.active;

  return (
    <Card className="border-warning/30">
      <CardHeader className="pb-3">
        <CardTitle className="text-base flex items-center gap-2">
          <Shield className="h-4 w-4" />
          Moderation
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">Status:</span>
          <Badge variant={badge.variant as any} className="text-xs">
            {badge.label}
          </Badge>
        </div>

        {moderationNote && (
          <div className="text-xs">
            <span className="text-muted-foreground">Note:</span>{" "}
            <span>{moderationNote}</span>
          </div>
        )}

        {moderatedAt && (
          <div className="text-xs text-muted-foreground">
            Last action: {new Date(moderatedAt).toLocaleString()}
          </div>
        )}

        <textarea
          value={note}
          onChange={(e) => setNote(e.target.value)}
          placeholder="Moderator note (optional)"
          className="w-full text-xs bg-secondary rounded-md px-3 py-2 resize-none border border-border focus:outline-none focus:ring-1 focus:ring-primary"
          rows={2}
        />

        <div className="flex flex-col gap-2">
          {moderationStatus === "active" && (
            <>
              <Button
                variant="outline"
                size="sm"
                className="w-full justify-start"
                disabled={acting}
                onClick={() => handleAction("request_review")}
              >
                <Eye className="h-3.5 w-3.5 mr-2" />
                Flag for Review
              </Button>
              <Button
                variant="destructive"
                size="sm"
                className="w-full justify-start"
                disabled={acting}
                onClick={() => handleAction("take_down")}
              >
                <EyeOff className="h-3.5 w-3.5 mr-2" />
                Take Down
              </Button>
            </>
          )}
          {moderationStatus === "under_review" && (
            <>
              <Button
                variant="default"
                size="sm"
                className="w-full justify-start"
                disabled={acting}
                onClick={() => handleAction("approve")}
              >
                <CheckCircle className="h-3.5 w-3.5 mr-2" />
                Approve
              </Button>
              <Button
                variant="destructive"
                size="sm"
                className="w-full justify-start"
                disabled={acting}
                onClick={() => handleAction("take_down")}
              >
                <EyeOff className="h-3.5 w-3.5 mr-2" />
                Take Down
              </Button>
            </>
          )}
          {moderationStatus === "taken_down" && (
            <Button
              variant="default"
              size="sm"
              className="w-full justify-start"
              disabled={acting}
              onClick={() => handleAction("approve")}
            >
              <CheckCircle className="h-3.5 w-3.5 mr-2" />
              Restore Listing
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export function ExtensionModerationBanner({
  status,
}: {
  status: string;
}) {
  if (status === "taken_down") {
    return (
      <div className="bg-destructive/10 border border-destructive/30 rounded-lg p-4 flex items-center gap-3">
        <AlertTriangle className="h-5 w-5 text-destructive shrink-0" />
        <div>
          <p className="text-sm font-medium text-destructive">This extension has been taken down</p>
          <p className="text-xs text-muted-foreground">
            This listing has been removed by a moderator and is no longer available for download.
          </p>
        </div>
      </div>
    );
  }

  if (status === "under_review") {
    return (
      <div className="bg-warning/10 border border-warning/30 rounded-lg p-4 flex items-center gap-3">
        <Eye className="h-5 w-5 text-warning shrink-0" />
        <div>
          <p className="text-sm font-medium text-warning">This extension is under review</p>
          <p className="text-xs text-muted-foreground">
            This listing is being reviewed by the moderation team.
          </p>
        </div>
      </div>
    );
  }

  return null;
}
