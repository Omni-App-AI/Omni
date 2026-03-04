import type { Metadata } from "next";
import { redirect } from "next/navigation";
import Link from "next/link";
import { Plus, MessageSquare, CheckCircle2, Eye } from "lucide-react";
import { createClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { timeAgo } from "@/lib/utils";

export const metadata: Metadata = {
  title: "My Posts",
  description: "Manage your forum posts.",
};

export default async function DashboardPostsPage() {
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) {
    redirect("/login?redirect=/dashboard/posts");
  }

  const { data: postsData } = await supabase
    .from("forum_posts")
    .select(
      "id, title, vote_score, reply_count, view_count, solved, locked, created_at, category:forum_categories(id, name), extension:extensions(id, name)",
    )
    .eq("author_id", user.id)
    .order("created_at", { ascending: false });

  const posts = (postsData as any[] | null) || [];

  return (
    <div className="p-6 lg:p-8 max-w-4xl">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-bold">My Posts</h1>
          <p className="text-sm text-muted-foreground mt-1">
            {posts.length} {posts.length === 1 ? "post" : "posts"}
          </p>
        </div>
        <Link href="/community/new">
          <Button size="sm" className="gap-2">
            <Plus className="h-3.5 w-3.5" />
            New Post
          </Button>
        </Link>
      </div>

      {posts.length === 0 ? (
        <div className="border border-border/50 rounded-lg bg-card/30 p-8 text-center">
          <MessageSquare className="h-8 w-8 text-muted-foreground/40 mx-auto mb-3" />
          <p className="text-sm text-muted-foreground mb-4">
            You haven&apos;t created any forum posts yet.
          </p>
          <Link href="/community/new">
            <Button size="sm" variant="outline" className="gap-2">
              <Plus className="h-3.5 w-3.5" />
              Create your first post
            </Button>
          </Link>
        </div>
      ) : (
        <div className="border border-border/50 rounded-lg overflow-hidden bg-card/30">
          {posts.map((post) => (
            <div
              key={post.id}
              className="flex items-start gap-4 p-4 border-b border-border/30 last:border-0 hover:bg-card/50 transition-colors"
            >
              {/* Votes */}
              <div className="flex flex-col items-center gap-0.5 w-10 shrink-0 pt-0.5">
                <span
                  className={`text-sm font-medium tabular-nums ${
                    post.vote_score > 0
                      ? "text-primary"
                      : post.vote_score < 0
                        ? "text-destructive"
                        : "text-muted-foreground"
                  }`}
                >
                  {post.vote_score}
                </span>
                <span className="text-[9px] text-muted-foreground/60">votes</span>
              </div>

              {/* Content */}
              <div className="flex-1 min-w-0">
                <Link
                  href={`/community/post/${post.id}`}
                  className="text-sm font-medium hover:text-primary transition-colors line-clamp-1"
                >
                  {post.title}
                </Link>
                <div className="flex items-center gap-2 mt-1.5 text-xs text-muted-foreground">
                  {post.category && (
                    <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                      {post.category.name}
                    </Badge>
                  )}
                  {post.extension && (
                    <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                      {post.extension.name}
                    </Badge>
                  )}
                  {post.solved && (
                    <span className="flex items-center gap-0.5 text-success">
                      <CheckCircle2 className="h-3 w-3" />
                      Solved
                    </span>
                  )}
                  {post.locked && (
                    <Badge variant="destructive" className="text-[10px] px-1.5 py-0">
                      Locked
                    </Badge>
                  )}
                  <span className="flex items-center gap-0.5">
                    <MessageSquare className="h-3 w-3" />
                    {post.reply_count}
                  </span>
                  <span className="flex items-center gap-0.5">
                    <Eye className="h-3 w-3" />
                    {post.view_count}
                  </span>
                  <span>{timeAgo(post.created_at)}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
