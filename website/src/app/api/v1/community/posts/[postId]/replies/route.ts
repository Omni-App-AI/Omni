import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient, createClient } from "@/lib/supabase/server";
import { withProtection } from "@/lib/anti-bot/with-protection";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId } = await params;
  const supabase = createServiceClient();

  // Get current user to allow shadow-banned users to see their own replies
  const authClient = await createClient();
  const { data: { user } } = await authClient.auth.getUser();

  let query = supabase
    .from("forum_replies")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator)",
    )
    .eq("post_id", postId);

  // Filter out shadow-hidden replies (except for the author's own)
  if (user) {
    query = query.or(`shadow_hidden.eq.false,author_id.eq.${user.id}`);
  } else {
    query = query.eq("shadow_hidden", false);
  }

  const { data: replies, error } = await query
    .order("is_accepted", { ascending: false })
    .order("vote_score", { ascending: false })
    .order("created_at", { ascending: true });

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({ replies });
}

export const POST = withProtection(
  {
    turnstile: "trust-gated",
    rateLimit: "reply_create",
    honeypot: true,
    spamCheck: true,
    contentType: "reply",
    trustGate: "can_reply",
  },
  async (_request, { user, body: reqBody, shadowBanned }, routeContext) => {
    const { postId } = await routeContext!.params;
    const supabase = await createClient();

    // Check post exists and is not locked
    const { data: post } = await supabase
      .from("forum_posts")
      .select("id, locked, reply_count")
      .eq("id", postId)
      .single();

    if (!post) {
      return NextResponse.json({ error: "Post not found" }, { status: 404 });
    }

    if ((post as any).locked) {
      return NextResponse.json({ error: "This thread is locked" }, { status: 403 });
    }

    const { body: replyBody, parent_reply_id } = reqBody as {
      body?: string;
      parent_reply_id?: string;
    };

    if (!replyBody) {
      return NextResponse.json({ error: "Reply body is required" }, { status: 400 });
    }

    const { data: reply, error } = await supabase
      .from("forum_replies")
      // @ts-expect-error -- Supabase type inference limitation with manual Database type
      .insert({
        post_id: postId,
        author_id: user.id,
        body: replyBody,
        parent_reply_id: parent_reply_id || null,
        shadow_hidden: shadowBanned,
      })
      .select("*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator)")
      .single();

    if (error) {
      return NextResponse.json({ error: error.message }, { status: 500 });
    }

    // Update post reply_count and last_activity_at (uses service_role to bypass protection trigger)
    try {
      const svc = createServiceClient();
      await (svc
        .from("forum_posts") as any)
        .update({
          reply_count: ((post as any).reply_count || 0) + 1,
          last_activity_at: new Date().toISOString(),
        })
        .eq("id", postId);
    } catch { /* ignore */ }

    return NextResponse.json({ reply }, { status: 201 });
  }
);
