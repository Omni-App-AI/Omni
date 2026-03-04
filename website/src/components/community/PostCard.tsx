import Link from "next/link";
import { MessageSquare, CheckCircle2, Eye, Pin, Shield } from "lucide-react";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { timeAgo } from "@/lib/utils";
import type { ForumPostFull } from "@/lib/supabase/types";

interface PostCardProps {
  post: ForumPostFull;
}

export function PostCard({ post }: PostCardProps) {
  return (
    <div className="flex gap-4 p-4 border-b border-border/50 last:border-0 hover:bg-card/50 transition-colors">
      {/* Vote score */}
      <div className="flex flex-col items-center gap-1 pt-1 shrink-0 w-12">
        <span className={`text-sm font-medium tabular-nums ${post.vote_score > 0 ? "text-primary" : post.vote_score < 0 ? "text-destructive" : "text-muted-foreground"}`}>
          {post.vote_score}
        </span>
        <span className="text-[10px] text-muted-foreground/60">votes</span>
      </div>

      {/* Replies count */}
      <div className={`flex flex-col items-center gap-1 pt-1 shrink-0 w-12 ${post.solved ? "text-success" : ""}`}>
        <span className={`text-sm font-medium tabular-nums ${post.solved ? "text-success" : "text-muted-foreground"}`}>
          {post.reply_count}
        </span>
        <span className={`text-[10px] ${post.solved ? "text-success/60" : "text-muted-foreground/60"}`}>
          {post.solved ? "solved" : "replies"}
        </span>
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-start gap-2">
          {post.pinned && <Pin className="h-3.5 w-3.5 text-primary shrink-0 mt-1" />}
          <Link
            href={`/community/post/${post.id}`}
            className="text-[15px] font-medium hover:text-primary transition-colors line-clamp-1"
          >
            {post.title}
          </Link>
          {post.solved && (
            <CheckCircle2 className="h-4 w-4 text-success shrink-0 mt-0.5" />
          )}
        </div>

        <div className="flex items-center gap-3 mt-1.5 text-xs text-muted-foreground">
          {/* Category or Extension badge */}
          {post.category && (
            <Link
              href={`/community/${post.category.id}`}
              className="hover:text-foreground"
            >
              <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                {post.category.name}
              </Badge>
            </Link>
          )}
          {post.extension && (
            <Link
              href={`/extensions/${post.extension.id}/discussions`}
              className="hover:text-foreground"
            >
              <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                {post.extension.name}
              </Badge>
            </Link>
          )}

          {/* Author */}
          <Link
            href={`/publishers/${post.author.username}`}
            className="flex items-center gap-1.5 hover:text-foreground"
          >
            <Avatar src={post.author.avatar_url} fallback={post.author.display_name} size="xs" />
            <span>{post.author.display_name}</span>
            {post.author.is_moderator && (
              <span className="inline-flex items-center gap-0.5 px-1.5 py-0 rounded bg-primary/10 text-primary text-[10px] font-medium">
                <Shield className="h-2.5 w-2.5" />
                Mod
              </span>
            )}
          </Link>

          {/* Time */}
          <span>{timeAgo(post.created_at)}</span>

          {/* Views */}
          <span className="flex items-center gap-1">
            <Eye className="h-3 w-3" />
            {post.view_count}
          </span>
        </div>
      </div>
    </div>
  );
}
