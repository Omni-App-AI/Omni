"use client";

import { useState } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

interface ReplyFormProps {
  postId: string;
  parentReplyId?: string;
  onSubmitted?: () => void;
  onCancel?: () => void;
  placeholder?: string;
}

export function ReplyForm({ postId, parentReplyId, onSubmitted, onCancel, placeholder }: ReplyFormProps) {
  const [body, setBody] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!body.trim()) return;

    setSubmitting(true);
    setError(null);

    try {
      const res = await fetch(`/api/v1/community/posts/${postId}/replies`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          body: body.trim(),
          parent_reply_id: parentReplyId || undefined,
        }),
      });

      if (!res.ok) {
        const data = await res.json();
        setError(data.error || "Failed to post reply");
        return;
      }

      setBody("");
      if (onSubmitted) {
        onSubmitted();
      } else {
        window.location.reload();
      }
    } catch {
      setError("Failed to post reply");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      <Textarea
        value={body}
        onChange={(e) => setBody(e.target.value)}
        placeholder={placeholder || "Write your reply..."}
        rows={4}
        className="font-mono text-sm"
      />
      {error && <p className="text-xs text-destructive">{error}</p>}
      <div className="flex gap-2">
        <Button type="submit" size="sm" disabled={submitting || !body.trim()}>
          {submitting ? (
            <>
              <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
              Posting...
            </>
          ) : (
            "Post Reply"
          )}
        </Button>
        {onCancel && (
          <Button type="button" size="sm" variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
        )}
      </div>
    </form>
  );
}
