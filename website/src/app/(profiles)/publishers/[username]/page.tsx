import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { ProfileHeader } from "@/components/profile/ProfileHeader";
import { ProfileTabsClient } from "./ProfileTabsClient";
import type {
  Profile,
  Extension,
  ForumPost,
  ExtensionWithPublisher,
} from "@/lib/supabase/types";

interface Props {
  params: Promise<{ username: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { username } = await params;
  const supabase = createServiceClient();

  const { data: profileData } = await supabase
    .from("profiles")
    .select("display_name, bio, avatar_url")
    .eq("username", username)
    .single();

  const profile = profileData as Pick<Profile, "display_name" | "bio" | "avatar_url"> | null;

  if (!profile) return { title: "User Not Found" };

  return {
    title: `${profile.display_name} — Profile`,
    description: profile.bio || `${profile.display_name}'s profile on Omni Marketplace`,
    openGraph: {
      title: `${profile.display_name} — Omni Publisher`,
      description: profile.bio || `${profile.display_name}'s profile on Omni Marketplace`,
      url: `/publishers/${username}`,
      ...(profile.avatar_url ? { images: [profile.avatar_url] } : {}),
    },
    alternates: {
      canonical: `/publishers/${username}`,
    },
  };
}

export default async function PublisherProfilePage({ params }: Props) {
  const { username } = await params;
  const service = createServiceClient();

  // Fetch profile
  const { data: profileData } = await service
    .from("profiles")
    .select("*")
    .eq("username", username)
    .single();

  const profile = profileData as Profile | null;
  if (!profile) notFound();

  // Fetch published extensions
  const { data: extensionsData } = await service
    .from("extensions")
    .select("*, publisher:profiles(*)")
    .eq("publisher_id", profile.id)
    .eq("published", true)
    .order("total_downloads", { ascending: false });

  const extensions = (extensionsData as ExtensionWithPublisher[] | null) || [];

  // Fetch forum posts
  const { data: postsData } = await service
    .from("forum_posts")
    .select(
      "id, title, vote_score, reply_count, solved, created_at, category:forum_categories(id, name), extension:extensions(id, name)",
    )
    .eq("author_id", profile.id)
    .order("created_at", { ascending: false })
    .limit(20);

  const posts = (postsData as any[] | null) || [];

  // Fetch reviews
  const { data: reviewsData } = await service
    .from("reviews")
    .select("id, rating, title, body, created_at, extension:extensions(id, name, icon_url)")
    .eq("user_id", profile.id)
    .order("created_at", { ascending: false })
    .limit(20);

  const reviews = (reviewsData as any[] | null) || [];

  // Fetch badges
  const { data: badgesData } = await service
    .from("user_badges")
    .select("badge_id, earned_at")
    .eq("user_id", profile.id)
    .order("earned_at", { ascending: false });

  const badges = (badgesData as { badge_id: string; earned_at: string }[] | null) || [];

  // Check if current user follows this profile
  const supabase = await createClient();
  const {
    data: { user },
  } = await supabase.auth.getUser();

  let isFollowing = false;
  const isOwnProfile = user?.id === profile.id;

  if (user && !isOwnProfile) {
    const { data: followData } = await supabase
      .from("user_followers")
      .select("follower_id")
      .eq("follower_id", user.id)
      .eq("following_id", profile.id);

    isFollowing = (followData && followData.length > 0) || false;
  }

  // Fetch pinned items
  let pinnedExtension = null;
  let pinnedPost = null;

  if (profile.pinned_extension_id) {
    const { data: pe } = await service
      .from("extensions")
      .select("id, name, icon_url, short_description")
      .eq("id", profile.pinned_extension_id)
      .single();
    pinnedExtension = pe as Pick<Extension, "id" | "name" | "icon_url" | "short_description"> | null;
  }

  if (profile.pinned_post_id) {
    const { data: pp } = await service
      .from("forum_posts")
      .select("id, title")
      .eq("id", profile.pinned_post_id)
      .single();
    pinnedPost = pp as Pick<ForumPost, "id" | "title"> | null;
  }

  // Build activity feed
  const activityItems = [
    ...posts.map((p: any) => ({
      type: "post" as const,
      id: p.id,
      title: p.title,
      href: `/community/post/${p.id}`,
      date: p.created_at,
      meta: p.solved ? "Solved" : `${p.reply_count} replies`,
    })),
    ...reviews.map((r: any) => ({
      type: "review" as const,
      id: r.id,
      title: r.extension?.name || "Extension",
      href: `/extensions/${r.extension?.id}`,
      date: r.created_at,
      meta: `${"★".repeat(r.rating)}`,
    })),
    ...extensions.map((e) => ({
      type: "extension" as const,
      id: e.id,
      title: e.name,
      href: `/extensions/${e.id}`,
      date: e.created_at,
    })),
  ].sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());

  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-12">
      {/* Profile Header */}
      <ProfileHeader
        profile={profile}
        isOwnProfile={isOwnProfile}
        isFollowing={isFollowing}
        pinnedExtension={pinnedExtension}
        pinnedPost={pinnedPost}
        badges={badges}
      />

      {/* Tabbed content */}
      <div className="mt-8">
        <ProfileTabsClient
          extensions={extensions}
          posts={posts}
          reviews={reviews}
          badges={badges}
          activityItems={activityItems}
          extensionCount={extensions.length}
          postCount={posts.length}
          reviewCount={reviews.length}
        />
      </div>
    </div>
  );
}
