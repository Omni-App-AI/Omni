import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import { ChevronRight } from "lucide-react";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { PostContent } from "@/components/community/PostContent";
import { ReplyList } from "@/components/community/ReplyList";
import type { ForumPostFull, ForumReplyWithAuthor } from "@/lib/supabase/types";

interface Props {
  params: Promise<{ postId: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { postId } = await params;
  const supabase = createServiceClient();

  const { data } = await supabase
    .from("forum_posts")
    .select("title")
    .eq("id", postId)
    .single();

  if (!data) return { title: "Post Not Found" };

  return {
    title: `${(data as any).title} — Community`,
    description: `Discussion: ${(data as any).title}`,
    openGraph: {
      title: `${(data as any).title} — Omni Community`,
      description: `Discussion: ${(data as any).title}`,
      url: `/community/post/${postId}`,
    },
    alternates: { canonical: `/community/post/${postId}` },
  };
}

export default async function PostPage({ params }: Props) {
  const { postId } = await params;
  const service = createServiceClient();

  // Fetch post with author + category + extension
  const { data: postData } = await service
    .from("forum_posts")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator), category:forum_categories(*), extension:extensions(id, name, icon_url)",
    )
    .eq("id", postId)
    .single();

  const post = postData as ForumPostFull | null;
  if (!post) notFound();

  // Increment view count (fire and forget)
  try {
    await service
      .from("forum_posts")
      // @ts-expect-error -- Supabase generic chain infers Update as never
      .update({ view_count: post.view_count + 1 })
      .eq("id", postId);
  } catch { /* ignore */ }

  // Fetch replies
  const { data: repliesData } = await service
    .from("forum_replies")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator)",
    )
    .eq("post_id", postId)
    .order("is_accepted", { ascending: false })
    .order("vote_score", { ascending: false })
    .order("created_at", { ascending: true });

  const replies = (repliesData as ForumReplyWithAuthor[] | null) || [];

  // Get current user + their votes
  const supabase = await createClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  let userPostVote: number | null = null;
  const userReplyVotes: Record<string, number> = {};

  if (user) {
    // Get user's vote on this post
    const { data: postVoteData } = await supabase
      .from("forum_votes")
      .select("value")
      .eq("user_id", user.id)
      .eq("post_id", postId);

    if (postVoteData && postVoteData.length > 0) {
      userPostVote = (postVoteData[0] as any).value;
    }

    // Get user's votes on replies
    const replyIds = replies.map((r) => r.id);
    if (replyIds.length > 0) {
      const { data: replyVotesData } = await supabase
        .from("forum_votes")
        .select("reply_id, value")
        .eq("user_id", user.id)
        .in("reply_id", replyIds);

      if (replyVotesData) {
        for (const v of replyVotesData) {
          const vote = v as any;
          if (vote.reply_id) {
            userReplyVotes[vote.reply_id] = vote.value;
          }
        }
      }
    }
  }

  const isAuthor = user?.id === post.author.id;

  return (
    <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-10">
      {/* Breadcrumb */}
      <nav className="flex items-center gap-1 text-sm text-muted-foreground mb-6">
        <Link href="/community" className="hover:text-foreground">
          Community
        </Link>
        <ChevronRight className="h-3 w-3" />
        {post.category && (
          <>
            <Link
              href={`/community/${post.category.id}`}
              className="hover:text-foreground"
            >
              {post.category.name}
            </Link>
            <ChevronRight className="h-3 w-3" />
          </>
        )}
        {post.extension && (
          <>
            <Link
              href={`/extensions/${post.extension.id}/discussions`}
              className="hover:text-foreground"
            >
              {post.extension.name}
            </Link>
            <ChevronRight className="h-3 w-3" />
          </>
        )}
        <span className="text-foreground truncate max-w-[200px]">{post.title}</span>
      </nav>

      {/* Post */}
      <div className="border border-border/50 rounded-lg bg-card/30 p-6 mb-8">
        <PostContent
          post={post}
          isAuthor={isAuthor}
          userVote={userPostVote}
        />
      </div>

      {/* Replies */}
      <div className="border border-border/50 rounded-lg bg-card/30 p-6">
        <ReplyList
          postId={postId}
          postAuthorId={post.author.id}
          replies={replies}
          isPostAuthor={isAuthor}
          currentUserId={user?.id}
          userVotes={userReplyVotes}
          locked={post.locked}
        />
      </div>
    </div>
  );
}
