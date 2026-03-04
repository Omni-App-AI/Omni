import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import { Plus, ChevronRight, Package, MessageSquare } from "lucide-react";
import { createServiceClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { PostList } from "@/components/community/PostList";
import type { Extension, ForumPostFull } from "@/lib/supabase/types";

interface Props {
  params: Promise<{ extensionId: string }>;
  searchParams: Promise<{ sort?: string; page?: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { extensionId } = await params;
  const supabase = createServiceClient();

  const { data } = await supabase
    .from("extensions")
    .select("name")
    .eq("id", extensionId)
    .single();

  if (!data) return { title: "Extension Not Found" };

  return {
    title: `Discussions — ${(data as any).name}`,
    description: `Community discussions about ${(data as any).name}`,
    openGraph: {
      title: `${(data as any).name} Discussions — Omni Community`,
      description: `Community discussions about ${(data as any).name}`,
      url: `/extensions/${extensionId}/discussions`,
    },
  };
}

export default async function ExtensionDiscussionsPage({ params, searchParams }: Props) {
  const { extensionId } = await params;
  const { sort = "newest", page: pageStr = "1" } = await searchParams;
  const page = parseInt(pageStr, 10) || 1;
  const limit = 20;

  const supabase = createServiceClient();

  // Fetch extension info
  const { data: extData } = await supabase
    .from("extensions")
    .select("id, name, icon_url, short_description")
    .eq("id", extensionId)
    .single();

  const extension = extData as Pick<Extension, "id" | "name" | "icon_url" | "short_description"> | null;
  if (!extension) notFound();

  // Fetch posts for this extension
  let query = supabase
    .from("forum_posts")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator), category:forum_categories(*), extension:extensions(id, name, icon_url)",
      { count: "exact" },
    )
    .eq("extension_id", extensionId);

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
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-8">
      {/* Breadcrumb */}
      <nav className="flex items-center gap-1 text-sm text-muted-foreground mb-6">
        <Link href="/extensions" className="hover:text-foreground">
          Extensions
        </Link>
        <ChevronRight className="h-3 w-3" />
        <Link href={`/extensions/${extensionId}`} className="hover:text-foreground">
          {extension.name}
        </Link>
        <ChevronRight className="h-3 w-3" />
        <span className="text-foreground">Discussions</span>
      </nav>

      {/* Extension header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-4">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-primary/10">
            {extension.icon_url ? (
              <img src={extension.icon_url} alt={extension.name} className="h-8 w-8 rounded-lg" />
            ) : (
              <Package className="h-6 w-6 text-primary" />
            )}
          </div>
          <div>
            <h1 className="text-xl font-bold flex items-center gap-2">
              <MessageSquare className="h-5 w-5 text-muted-foreground" />
              {extension.name} Discussions
            </h1>
            <p className="text-sm text-muted-foreground">{extension.short_description}</p>
          </div>
        </div>
        <Link href={`/community/new?extension=${extensionId}`}>
          <Button size="sm" className="gap-2">
            <Plus className="h-3.5 w-3.5" />
            New Discussion
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
          basePath={`/extensions/${extensionId}/discussions`}
        />
      </div>
    </div>
  );
}
