import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient, createClient } from "@/lib/supabase/server";

async function checkModerator() {
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) return { user: null, isModerator: false };

  const service = createServiceClient();
  const { data: profile } = await service
    .from("profiles")
    .select("is_moderator")
    .eq("id", user.id)
    .single();

  return { user, isModerator: !!(profile as any)?.is_moderator };
}

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId } = await params;
  const service = createServiceClient();

  // Check if requester is moderator (to allow viewing drafts)
  const { isModerator } = await checkModerator();

  let query = service
    .from("blog_posts")
    .select("*, author:profiles(id, username, display_name, avatar_url)")
    .eq("id", postId);

  if (!isModerator) {
    query = query.eq("published", true);
  }

  const { data: post, error } = await query.single();

  if (error || !post) {
    return NextResponse.json({ error: "Post not found" }, { status: 404 });
  }

  return NextResponse.json({ post });
}

export async function PUT(
  request: NextRequest,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId } = await params;
  const { user, isModerator } = await checkModerator();

  if (!user || !isModerator) {
    return NextResponse.json({ error: "Moderator access required" }, { status: 403 });
  }

  let body: Record<string, unknown>;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  const service = createServiceClient();

  // Fetch current post
  const { data: current } = await service
    .from("blog_posts")
    .select("published, published_at, featured")
    .eq("id", postId)
    .single();

  if (!current) {
    return NextResponse.json({ error: "Post not found" }, { status: 404 });
  }

  const updates: Record<string, unknown> = {};
  const allowedFields = [
    "title", "slug", "body", "excerpt", "category", "tags",
    "cover_image_url", "meta_title", "meta_description", "og_image_url",
    "canonical_url", "published", "featured", "read_time_minutes",
  ];

  for (const field of allowedFields) {
    if (body[field] !== undefined) {
      updates[field] = body[field];
    }
  }

  // Handle publish state transitions
  if (updates.published === true && !(current as any).published) {
    updates.published_at = new Date().toISOString();
  }

  // If featuring this post, unfeature any existing featured post
  if (updates.featured === true && !(current as any).featured) {
    await (service.from("blog_posts") as any).update({ featured: false }).eq("featured", true);
  }

  // Auto-calculate read time if body changed
  if (updates.body && typeof updates.body === "string") {
    updates.read_time_minutes = Math.max(1, Math.ceil(updates.body.length / 1500));
  }

  updates.updated_at = new Date().toISOString();

  const { data: post, error } = await (service
    .from("blog_posts") as any)
    .update(updates)
    .eq("id", postId)
    .select("*")
    .single();

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({ post });
}

export async function DELETE(
  _request: NextRequest,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId } = await params;
  const { user, isModerator } = await checkModerator();

  if (!user || !isModerator) {
    return NextResponse.json({ error: "Moderator access required" }, { status: 403 });
  }

  const service = createServiceClient();
  const { error } = await service
    .from("blog_posts")
    .delete()
    .eq("id", postId);

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({ success: true });
}
