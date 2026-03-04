import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";

export async function GET(request: NextRequest) {
  // Auth + moderator check
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const service = createServiceClient();
  const { data: profile } = await service
    .from("profiles")
    .select("is_moderator")
    .eq("id", user.id)
    .single();

  if (!profile || !(profile as any).is_moderator) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const { searchParams } = new URL(request.url);
  const status = searchParams.get("moderation_status");
  const search = searchParams.get("q");
  const page = parseInt(searchParams.get("page") || "1", 10);
  const limit = Math.min(parseInt(searchParams.get("limit") || "20", 10), 50);
  const offset = (page - 1) * limit;

  let query = service
    .from("extensions")
    .select(
      "id, name, short_description, icon_url, publisher_id, trust_level, published, total_downloads, average_rating, moderation_status, moderation_note, moderated_at, created_at, publisher:profiles(username, display_name)",
      { count: "exact" },
    );

  if (status) {
    query = query.eq("moderation_status", status);
  }

  if (search) {
    query = query.ilike("name", `%${search}%`);
  }

  const { data: extensions, error, count } = await query
    .order("moderation_status", { ascending: true })
    .order("created_at", { ascending: false })
    .range(offset, offset + limit - 1);

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({
    extensions,
    total: count || 0,
    page,
    limit,
    pages: Math.ceil((count || 0) / limit),
  });
}
