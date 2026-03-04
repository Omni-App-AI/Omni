import { NextResponse } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ username: string }> },
) {
  const { username } = await params;
  const supabase = createServiceClient();

  // Get profile
  const { data: profile, error: profileError } = await supabase
    .from("profiles")
    .select("*")
    .eq("username", username)
    .single();

  if (profileError || !profile) {
    return NextResponse.json({ error: "User not found" }, { status: 404 });
  }

  const userId = (profile as any).id as string;

  // Get published extensions
  const { data: extensions } = await supabase
    .from("extensions")
    .select("id, name, short_description, icon_url, total_downloads, average_rating, trust_level, categories, latest_version")
    .eq("publisher_id", userId)
    .eq("published", true)
    .order("total_downloads", { ascending: false });

  // Get recent forum posts
  const { data: posts } = await supabase
    .from("forum_posts")
    .select("id, title, vote_score, reply_count, solved, created_at, category:forum_categories(id, name), extension:extensions(id, name)")
    .eq("author_id", userId)
    .order("created_at", { ascending: false })
    .limit(20);

  // Get reviews
  const { data: reviews } = await supabase
    .from("reviews")
    .select("id, rating, title, body, created_at, extension:extensions(id, name, icon_url)")
    .eq("user_id", userId)
    .order("created_at", { ascending: false })
    .limit(20);

  // Get badges
  const { data: badges } = await supabase
    .from("user_badges")
    .select("badge_id, earned_at")
    .eq("user_id", userId)
    .order("earned_at", { ascending: false });

  return NextResponse.json({
    profile,
    extensions: extensions || [],
    posts: posts || [],
    reviews: reviews || [],
    badges: badges || [],
  });
}
