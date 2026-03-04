import type { Metadata } from "next";
import Link from "next/link";
import { Plus, TrendingUp } from "lucide-react";
import { createServiceClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { CategoryCard } from "@/components/community/CategoryCard";
import { PostCard } from "@/components/community/PostCard";
import { ForumSidebar } from "@/components/community/ForumSidebar";
import { FORUM_CATEGORIES } from "@/lib/constants";
import type { ForumPostFull } from "@/lib/supabase/types";

export const metadata: Metadata = {
  title: "Community — Discussions, Help & Showcase",
  description:
    "Join the Omni AI agent community to ask questions, share projects, discuss WASM extension development, request features, and connect with other developers building AI tools.",
  openGraph: {
    title: "Omni Community — AI Agent Discussions, Help & Showcase",
    description:
      "Join the Omni community to ask questions, share AI agent projects, discuss WASM extension development, request features, and connect with other developers.",
    url: "/community",
  },
  alternates: { canonical: "/community" },
};

export default async function CommunityPage() {
  const supabase = createServiceClient();

  // Fetch real post counts by counting actual non-hidden posts per category
  const { data: postCountData } = await supabase
    .from("forum_posts")
    .select("category_id")
    .eq("shadow_hidden", false)
    .not("category_id", "is", null);

  const categoryMap = new Map<string, number>();
  if (postCountData) {
    for (const row of postCountData) {
      const catId = (row as any).category_id as string;
      if (catId) {
        categoryMap.set(catId, (categoryMap.get(catId) || 0) + 1);
      }
    }
  }

  // Fetch recent posts
  const { data: postsData } = await supabase
    .from("forum_posts")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator), category:forum_categories(*), extension:extensions(id, name, icon_url)",
    )
    .order("pinned", { ascending: false })
    .order("last_activity_at", { ascending: false })
    .limit(15);

  const posts = (postsData as ForumPostFull[] | null) || [];

  // Fetch trending posts (most votes in recent period)
  const { data: trendingData } = await supabase
    .from("forum_posts")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator), category:forum_categories(*), extension:extensions(id, name, icon_url)",
    )
    .order("vote_score", { ascending: false })
    .limit(5);

  const trending = (trendingData as ForumPostFull[] | null) || [];

  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-10">
      <div className="flex gap-8">
        <ForumSidebar />

        <div className="flex-1 min-w-0">
          {/* Header */}
          <div className="flex items-center justify-between mb-8">
            <div>
              <h1 className="text-2xl font-bold">Community</h1>
              <p className="text-sm text-muted-foreground mt-1">
                Ask questions, share your projects, and connect with other developers.
              </p>
            </div>
            <Link href="/community/new">
              <Button size="sm" className="gap-2">
                <Plus className="h-3.5 w-3.5" />
                New Post
              </Button>
            </Link>
          </div>

          {/* Categories grid */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 mb-10">
            {FORUM_CATEGORIES.map((cat) => (
              <CategoryCard
                key={cat.id}
                id={cat.id}
                name={cat.name}
                description={cat.description}
                icon={cat.icon}
                postCount={categoryMap.get(cat.id) || 0}
              />
            ))}
          </div>

          {/* Trending */}
          {trending.length > 0 && (
            <div className="mb-10">
              <h2 className="flex items-center gap-2 text-lg font-medium mb-4">
                <TrendingUp className="h-5 w-5 text-primary" />
                Trending
              </h2>
              <div className="border border-border/50 rounded-lg overflow-hidden bg-card/30">
                {trending.map((post) => (
                  <PostCard key={post.id} post={post} />
                ))}
              </div>
            </div>
          )}

          {/* Recent Posts */}
          <div>
            <h2 className="text-lg font-medium mb-4">Recent Posts</h2>
            <div className="border border-border/50 rounded-lg overflow-hidden bg-card/30">
              {posts.length === 0 ? (
                <div className="p-8 text-center text-muted-foreground text-sm">
                  No posts yet. Be the first to start a discussion.
                </div>
              ) : (
                posts.map((post) => (
                  <PostCard key={post.id} post={post} />
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
