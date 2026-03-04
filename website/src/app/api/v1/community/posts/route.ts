import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient, createClient } from "@/lib/supabase/server";
import { withProtection } from "@/lib/anti-bot/with-protection";

export async function GET(request: NextRequest) {
  const supabase = createServiceClient();
  const { searchParams } = new URL(request.url);

  const category = searchParams.get("category");
  const extensionId = searchParams.get("extension_id");
  const sort = searchParams.get("sort") || "newest";
  const page = parseInt(searchParams.get("page") || "1", 10);
  const limit = Math.min(parseInt(searchParams.get("limit") || "20", 10), 50);
  const search = searchParams.get("q");
  const offset = (page - 1) * limit;

  // Get current user to allow shadow-banned users to see their own posts
  const authClient = await createClient();
  const { data: { user } } = await authClient.auth.getUser();

  let query = supabase
    .from("forum_posts")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator), category:forum_categories(*), extension:extensions(id, name, icon_url)",
      { count: "exact" },
    );

  // Filter out shadow-hidden posts (except for the author's own)
  if (user) {
    query = query.or(`shadow_hidden.eq.false,author_id.eq.${user.id}`);
  } else {
    query = query.eq("shadow_hidden", false);
  }

  if (category) {
    query = query.eq("category_id", category);
  }

  if (extensionId) {
    query = query.eq("extension_id", extensionId);
  }

  if (search) {
    query = query.or(`title.ilike.%${search}%,body.ilike.%${search}%`);
  }

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

  const { data: posts, error, count } = await query.range(offset, offset + limit - 1);

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({
    posts,
    total: count || 0,
    page,
    limit,
    pages: Math.ceil((count || 0) / limit),
  });
}

export const POST = withProtection(
  {
    turnstile: "trust-gated",
    rateLimit: "post_create",
    honeypot: true,
    spamCheck: true,
    contentType: "post",
    trustGate: "can_post",
  },
  async (_request, { user, profile, body, shadowBanned }) => {
    const supabase = await createClient();
    const { title, body: postBody, category_id, extension_id } = body as {
      title?: string;
      body?: string;
      category_id?: string;
      extension_id?: string;
    };

    if (!title || !postBody) {
      return NextResponse.json({ error: "Title and body are required" }, { status: 400 });
    }

    if (!category_id && !extension_id) {
      return NextResponse.json({ error: "Either category_id or extension_id is required" }, { status: 400 });
    }

    if (category_id && extension_id) {
      return NextResponse.json({ error: "Cannot set both category_id and extension_id" }, { status: 400 });
    }

    // Only moderators can post in the Announcements category
    if (category_id === "announcements" && !profile.is_moderator) {
      return NextResponse.json(
        { error: "Only moderators can post in the Announcements category" },
        { status: 403 }
      );
    }

    const { data: post, error } = await supabase
      .from("forum_posts")
      .insert({
        author_id: user.id,
        title,
        body: postBody,
        category_id: category_id || null,
        extension_id: extension_id || null,
        shadow_hidden: shadowBanned,
      } as any)
      .select("*")
      .single();

    if (error) {
      return NextResponse.json({ error: error.message }, { status: 500 });
    }

    // Increment post_count on profile (fire and forget, uses service_role to bypass protection trigger)
    try {
      const svc = createServiceClient();
      const { data: p } = await svc.from("profiles").select("post_count").eq("id", user.id).single();
      if (p) {
        await (svc.from("profiles") as any).update({ post_count: ((p as any).post_count || 0) + 1 }).eq("id", user.id);
      }
    } catch { /* ignore */ }

    // Increment post_count on forum_categories (fire and forget)
    if (category_id) {
      try {
        const serviceClient = createServiceClient();
        const { data: cat } = await serviceClient.from("forum_categories").select("post_count").eq("id", category_id).single();
        if (cat) {
          // @ts-expect-error -- Supabase type inference limitation with manual Database type
          await serviceClient.from("forum_categories").update({ post_count: ((cat as any).post_count || 0) + 1 }).eq("id", category_id);
        }
      } catch { /* ignore */ }
    }

    return NextResponse.json({ post }, { status: 201 });
  }
);
