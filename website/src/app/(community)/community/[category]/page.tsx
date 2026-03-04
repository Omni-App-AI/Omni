import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import { Plus } from "lucide-react";
import { createServiceClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { PostList } from "@/components/community/PostList";
import { ForumSidebar } from "@/components/community/ForumSidebar";
import { FORUM_CATEGORIES } from "@/lib/constants";
import type { ForumPostFull } from "@/lib/supabase/types";

interface Props {
  params: Promise<{ category: string }>;
  searchParams: Promise<{ sort?: string; page?: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { category } = await params;
  const cat = FORUM_CATEGORIES.find((c) => c.id === category);
  if (!cat) return { title: "Category Not Found" };

  return {
    title: `${cat.name} — Community`,
    description: cat.description,
    openGraph: {
      title: `${cat.name} — Omni Community`,
      description: cat.description,
      url: `/community/${category}`,
    },
    alternates: { canonical: `/community/${category}` },
  };
}

export default async function CategoryPage({ params, searchParams }: Props) {
  const { category } = await params;
  const { sort = "newest", page: pageStr = "1" } = await searchParams;
  const page = parseInt(pageStr, 10) || 1;
  const limit = 20;

  const cat = FORUM_CATEGORIES.find((c) => c.id === category);
  if (!cat) notFound();

  const supabase = createServiceClient();

  let query = supabase
    .from("forum_posts")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator), category:forum_categories(*), extension:extensions(id, name, icon_url)",
      { count: "exact" },
    )
    .eq("category_id", category);

  switch (sort) {
    case "votes":
      query = query.order("vote_score", { ascending: false });
      break;
    case "activity":
      query = query.order("last_activity_at", { ascending: false });
      break;
    case "unanswered":
      query = query.eq("solved", false).eq("reply_count", 0).order("created_at", { ascending: false });
      break;
    default:
      query = query.order("pinned", { ascending: false }).order("created_at", { ascending: false });
  }

  const offset = (page - 1) * limit;
  const { data: postsData, count } = await query.range(offset, offset + limit - 1);
  const posts = (postsData as ForumPostFull[] | null) || [];
  const total = count || 0;

  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-10">
      <div className="flex gap-8">
        <ForumSidebar />

        <div className="flex-1 min-w-0">
          {/* Header */}
          <div className="flex items-center justify-between mb-6">
            <div>
              <h1 className="text-2xl font-bold">{cat.name}</h1>
              <p className="text-sm text-muted-foreground mt-1">{cat.description}</p>
            </div>
            <Link href={`/community/new?category=${category}`}>
              <Button size="sm" className="gap-2">
                <Plus className="h-3.5 w-3.5" />
                New Post
              </Button>
            </Link>
          </div>

          {/* Posts */}
          <div className="border border-border/50 rounded-lg overflow-hidden bg-card/30">
            <PostList
              posts={posts}
              total={total}
              page={page}
              pages={Math.ceil(total / limit)}
              basePath={`/community/${category}`}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
