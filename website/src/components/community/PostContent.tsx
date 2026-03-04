"use client";

import { useState } from "react";
import Link from "next/link";
import { Pencil, Calendar, Eye, MessageSquare, Shield } from "lucide-react";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { VoteButton } from "./VoteButton";
import { timeAgo } from "@/lib/utils";
import type { ForumPostFull } from "@/lib/supabase/types";

interface PostContentProps {
  post: ForumPostFull;
  isAuthor: boolean;
  userVote?: number | null;
}

export function PostContent({ post, isAuthor, userVote }: PostContentProps) {
  const [editing, setEditing] = useState(false);
  const [editBody, setEditBody] = useState(post.body);
  const [saving, setSaving] = useState(false);
  const [body, setBody] = useState(post.body);

  const handleSave = async () => {
    setSaving(true);
    try {
      const res = await fetch(`/api/v1/community/posts/${post.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ body: editBody }),
      });
      if (res.ok) {
        setBody(editBody);
        setEditing(false);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex gap-4">
      {/* Vote column */}
      <div className="shrink-0 pt-1">
        <VoteButton postId={post.id} initialScore={post.vote_score} initialUserVote={userVote} />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        {/* Header */}
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-xl font-bold">{post.title}</h1>
            <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
              <Link
                href={`/publishers/${post.author.username}`}
                className="flex items-center gap-1.5 hover:text-foreground"
              >
                <Avatar src={post.author.avatar_url} fallback={post.author.display_name} size="xs" />
                <span className="font-medium">{post.author.display_name}</span>
              </Link>
              {post.author.is_moderator && (
                <Badge variant="default" className="text-[10px] gap-0.5">
                  <Shield className="h-2.5 w-2.5" />
                  Mod
                </Badge>
              )}
              {post.author.verified_publisher && (
                <Badge variant="success" className="text-[10px]">Verified</Badge>
              )}
              <span className="flex items-center gap-1">
                <Calendar className="h-3 w-3" />
                {timeAgo(post.created_at)}
              </span>
              <span className="flex items-center gap-1">
                <Eye className="h-3 w-3" />
                {post.view_count} views
              </span>
              <span className="flex items-center gap-1">
                <MessageSquare className="h-3 w-3" />
                {post.reply_count} replies
              </span>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {post.category && (
              <Link href={`/community/${post.category.id}`}>
                <Badge variant="secondary">{post.category.name}</Badge>
              </Link>
            )}
            {post.extension && (
              <Link href={`/extensions/${post.extension.id}`}>
                <Badge variant="outline">{post.extension.name}</Badge>
              </Link>
            )}
            {post.solved && (
              <Badge variant="success">Solved</Badge>
            )}
            {post.locked && (
              <Badge variant="destructive">Locked</Badge>
            )}
          </div>
        </div>

        {/* Body */}
        {editing ? (
          <div className="space-y-3">
            <Textarea
              value={editBody}
              onChange={(e) => setEditBody(e.target.value)}
              rows={8}
              className="font-mono text-sm"
            />
            <div className="flex gap-2">
              <Button size="sm" onClick={handleSave} disabled={saving}>
                {saving ? "Saving..." : "Save"}
              </Button>
              <Button size="sm" variant="ghost" onClick={() => { setEditing(false); setEditBody(body); }}>
                Cancel
              </Button>
            </div>
          </div>
        ) : (
          <div className="prose prose-invert prose-sm max-w-none">
            <p className="whitespace-pre-wrap">{body}</p>
          </div>
        )}

        {/* Edit button */}
        {isAuthor && !editing && (
          <button
            onClick={() => setEditing(true)}
            className="flex items-center gap-1 mt-4 text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            <Pencil className="h-3 w-3" />
            Edit
          </button>
        )}
      </div>
    </div>
  );
}
