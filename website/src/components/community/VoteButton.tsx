"use client";

import { useState, useCallback } from "react";
import { ChevronUp, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

interface VoteButtonProps {
  postId?: string;
  replyId?: string;
  initialScore: number;
  initialUserVote?: number | null; // 1, -1, or null
  vertical?: boolean;
}

export function VoteButton({ postId, replyId, initialScore, initialUserVote = null, vertical = true }: VoteButtonProps) {
  const [score, setScore] = useState(initialScore);
  const [userVote, setUserVote] = useState<number | null>(initialUserVote);
  const [loading, setLoading] = useState(false);

  const handleVote = useCallback(async (value: 1 | -1) => {
    if (loading) return;
    setLoading(true);

    // Optimistic update
    const prevScore = score;
    const prevVote = userVote;

    if (userVote === value) {
      // Toggle off
      setScore(score - value);
      setUserVote(null);
    } else if (userVote) {
      // Switch direction
      setScore(score + value * 2);
      setUserVote(value);
    } else {
      // New vote
      setScore(score + value);
      setUserVote(value);
    }

    try {
      const res = await fetch("/api/v1/community/votes", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          post_id: postId || undefined,
          reply_id: replyId || undefined,
          value,
        }),
      });

      if (!res.ok) {
        // Revert on error
        setScore(prevScore);
        setUserVote(prevVote);
      }
    } catch {
      setScore(prevScore);
      setUserVote(prevVote);
    } finally {
      setLoading(false);
    }
  }, [score, userVote, loading, postId, replyId]);

  return (
    <div className={cn("flex items-center gap-0.5", vertical ? "flex-col" : "flex-row")}>
      <button
        onClick={() => handleVote(1)}
        disabled={loading}
        className={cn(
          "p-1 rounded transition-colors",
          userVote === 1
            ? "text-primary bg-primary/10"
            : "text-muted-foreground hover:text-foreground hover:bg-secondary/50",
        )}
        aria-label="Upvote"
      >
        <ChevronUp className="h-5 w-5" />
      </button>
      <span
        className={cn(
          "text-sm font-medium tabular-nums min-w-[1.5rem] text-center",
          score > 0 && "text-primary",
          score < 0 && "text-destructive",
          score === 0 && "text-muted-foreground",
        )}
      >
        {score}
      </span>
      <button
        onClick={() => handleVote(-1)}
        disabled={loading}
        className={cn(
          "p-1 rounded transition-colors",
          userVote === -1
            ? "text-destructive bg-destructive/10"
            : "text-muted-foreground hover:text-foreground hover:bg-secondary/50",
        )}
        aria-label="Downvote"
      >
        <ChevronDown className="h-5 w-5" />
      </button>
    </div>
  );
}
