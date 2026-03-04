"use client";

import { useState } from "react";
import Link from "next/link";
import { CheckCircle2, Reply, MessageSquare, Shield } from "lucide-react";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { VoteButton } from "./VoteButton";
import { ReplyForm } from "./ReplyForm";
import { timeAgo } from "@/lib/utils";
import type { ForumReplyWithAuthor } from "@/lib/supabase/types";

interface ReplyListProps {
  postId: string;
  postAuthorId: string;
  replies: ForumReplyWithAuthor[];
  isPostAuthor: boolean;
  currentUserId?: string;
  userVotes?: Record<string, number>;
  locked?: boolean;
}

export function ReplyList({ postId, postAuthorId, replies, isPostAuthor, currentUserId, userVotes = {}, locked }: ReplyListProps) {
  const [replyingTo, setReplyingTo] = useState<string | null>(null);

  const handleAccept = async (replyId: string) => {
    await fetch(`/api/v1/community/posts/${postId}/accept`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ reply_id: replyId }),
    });
    window.location.reload();
  };

  // Separate top-level and nested replies
  const topLevel = replies.filter((r) => !r.parent_reply_id);
  const nested = replies.filter((r) => r.parent_reply_id);

  const getRepliesFor = (parentId: string) =>
    nested.filter((r) => r.parent_reply_id === parentId);

  const renderReply = (reply: ForumReplyWithAuthor, isNested = false) => (
    <div
      key={reply.id}
      className={`flex gap-3 py-4 ${isNested ? "ml-12 border-l-2 border-border/30 pl-4" : "border-b border-border/50 last:border-0"} ${reply.is_accepted ? "bg-success/5 -mx-4 px-4 rounded-lg" : ""}`}
    >
      {/* Vote */}
      <div className="shrink-0">
        <VoteButton
          replyId={reply.id}
          initialScore={reply.vote_score}
          initialUserVote={userVotes[reply.id] || null}
        />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-2">
          <Link
            href={`/publishers/${reply.author.username}`}
            className="flex items-center gap-1.5 text-xs hover:text-foreground"
          >
            <Avatar src={reply.author.avatar_url} fallback={reply.author.display_name} size="xs" />
            <span className="font-medium">{reply.author.display_name}</span>
          </Link>
          {reply.author.id === postAuthorId && (
            <Badge variant="secondary" className="text-[10px] px-1.5 py-0">OP</Badge>
          )}
          {reply.author.is_moderator && (
            <Badge variant="default" className="text-[10px] px-1.5 py-0 gap-0.5">
              <Shield className="h-2.5 w-2.5" />
              Mod
            </Badge>
          )}
          {reply.author.verified_publisher && (
            <Badge variant="success" className="text-[10px] px-1.5 py-0">Verified</Badge>
          )}
          <span className="text-[10px] text-muted-foreground">{timeAgo(reply.created_at)}</span>
          {reply.is_accepted && (
            <span className="flex items-center gap-1 text-[10px] text-success font-medium">
              <CheckCircle2 className="h-3 w-3" />
              Accepted Answer
            </span>
          )}
        </div>

        <div className="prose prose-invert prose-sm max-w-none">
          <p className="whitespace-pre-wrap text-sm">{reply.body}</p>
        </div>

        <div className="flex items-center gap-3 mt-2">
          {/* Accept button (post author only, not for own replies) */}
          {isPostAuthor && !reply.is_accepted && reply.author.id !== currentUserId && (
            <button
              onClick={() => handleAccept(reply.id)}
              className="flex items-center gap-1 text-xs text-muted-foreground hover:text-success transition-colors"
            >
              <CheckCircle2 className="h-3 w-3" />
              Accept answer
            </button>
          )}

          {/* Reply button */}
          {!locked && !isNested && (
            <button
              onClick={() => setReplyingTo(replyingTo === reply.id ? null : reply.id)}
              className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              <Reply className="h-3 w-3" />
              Reply
            </button>
          )}
        </div>

        {/* Nested reply form */}
        {replyingTo === reply.id && (
          <div className="mt-3">
            <ReplyForm
              postId={postId}
              parentReplyId={reply.id}
              onCancel={() => setReplyingTo(null)}
              placeholder={`Reply to ${reply.author.display_name}...`}
            />
          </div>
        )}
      </div>
    </div>
  );

  return (
    <div>
      <h3 className="flex items-center gap-2 text-sm font-medium mb-4">
        <MessageSquare className="h-4 w-4" />
        {replies.length} {replies.length === 1 ? "Reply" : "Replies"}
      </h3>

      {topLevel.map((reply) => (
        <div key={reply.id}>
          {renderReply(reply)}
          {getRepliesFor(reply.id).map((nested) => renderReply(nested, true))}
        </div>
      ))}

      {/* Main reply form */}
      {!locked && (
        <div className="mt-6 pt-6 border-t border-border/50">
          <h4 className="text-sm font-medium mb-3">Your Reply</h4>
          <ReplyForm postId={postId} />
        </div>
      )}

      {locked && (
        <div className="mt-6 p-4 text-center text-sm text-muted-foreground border border-border/50 rounded-lg">
          This thread is locked. No new replies can be posted.
        </div>
      )}
    </div>
  );
}
