import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient, createClient } from "@/lib/supabase/server";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId } = await params;
  const supabase = createServiceClient();

  const { data: post, error } = await supabase
    .from("forum_posts")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url, reputation, verified_publisher, is_moderator), category:forum_categories(*), extension:extensions(id, name, icon_url)",
    )
    .eq("id", postId)
    .single();

  if (error || !post) {
    return NextResponse.json({ error: "Post not found" }, { status: 404 });
  }

  // Increment view count (fire and forget)
  try {
    await supabase
      .from("forum_posts")
      // @ts-expect-error -- Supabase type inference limitation with manual Database type
      .update({ view_count: ((post as any).view_count || 0) + 1 })
      .eq("id", postId);
  } catch { /* ignore */ }

  return NextResponse.json({ post });
}

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId } = await params;
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  // Check ownership
  const { data: existing } = await supabase
    .from("forum_posts")
    .select("author_id")
    .eq("id", postId)
    .single();

  if (!existing || (existing as any).author_id !== user.id) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json();
  const updates: Record<string, unknown> = {};

  if (body.title) updates.title = body.title;
  if (body.body) updates.body = body.body;
  updates.updated_at = new Date().toISOString();

  const { data: post, error } = await supabase
    .from("forum_posts")
    // @ts-expect-error -- Supabase type inference limitation with manual Database type
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
  _request: Request,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId } = await params;
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  // Check ownership
  const { data: existing } = await supabase
    .from("forum_posts")
    .select("author_id")
    .eq("id", postId)
    .single();

  if (!existing || (existing as any).author_id !== user.id) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const { error } = await supabase
    .from("forum_posts")
    .delete()
    .eq("id", postId);

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({ success: true });
}
