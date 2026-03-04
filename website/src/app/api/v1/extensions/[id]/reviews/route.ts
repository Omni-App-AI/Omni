import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient, createClient } from "@/lib/supabase/server";
import { withProtection } from "@/lib/anti-bot/with-protection";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const supabase = createServiceClient();

  // Get current user to allow shadow-banned users to see their own reviews
  const authClient = await createClient();
  const { data: { user } } = await authClient.auth.getUser();

  let query = supabase
    .from("reviews")
    .select("id, rating, title, body, version, created_at, user:profiles(username, display_name, avatar_url)")
    .eq("extension_id", id);

  // Filter out shadow-hidden reviews (except for the author's own)
  if (user) {
    query = query.or(`shadow_hidden.eq.false,user_id.eq.${user.id}`);
  } else {
    query = query.eq("shadow_hidden", false);
  }

  const { data: reviews, error } = await query
    .order("created_at", { ascending: false })
    .limit(50);

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({ reviews });
}

export const POST = withProtection(
  {
    turnstile: "trust-gated",
    rateLimit: "review_create",
    honeypot: true,
    spamCheck: true,
    contentType: "review",
    trustGate: "can_review",
  },
  async (_request, { user, body, shadowBanned }, routeContext) => {
    const { id } = await routeContext!.params;
    const supabase = await createClient();

    const { rating, title, body: reviewBody } = body as {
      rating?: number;
      title?: string;
      body?: string;
    };

    if (!rating || rating < 1 || rating > 5) {
      return NextResponse.json({ error: "Rating must be 1-5" }, { status: 400 });
    }

    const { data, error } = await supabase.from("reviews").upsert({
      extension_id: id,
      user_id: user.id,
      rating,
      title: title || null,
      body: reviewBody || null,
      shadow_hidden: shadowBanned,
    } as any);

    if (error) {
      return NextResponse.json({ error: error.message }, { status: 500 });
    }

    return NextResponse.json({ review: data }, { status: 201 });
  }
);
