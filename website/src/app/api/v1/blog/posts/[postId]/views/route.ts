import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";

export async function POST(
  _request: NextRequest,
  { params }: { params: Promise<{ postId: string }> },
) {
  const { postId: slug } = await params;
  const service = createServiceClient();

  // Fetch current view count and increment
  const { data: post } = await service
    .from("blog_posts")
    .select("id, view_count")
    .eq("slug", slug)
    .eq("published", true)
    .single();

  if (!post) {
    return NextResponse.json({ error: "Post not found" }, { status: 404 });
  }

  await (service.from("blog_posts") as any)
    .update({ view_count: ((post as any).view_count || 0) + 1 })
    .eq("id", (post as any).id);

  return NextResponse.json({ success: true });
}
