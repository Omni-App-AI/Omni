import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const q = searchParams.get("q");
  const category = searchParams.get("category");
  const sort = searchParams.get("sort") || "downloads";
  const trust = searchParams.get("trust");
  const page = Math.max(1, parseInt(searchParams.get("page") || "1", 10));
  const limit = Math.min(50, parseInt(searchParams.get("limit") || "20", 10));
  const offset = (page - 1) * limit;

  const supabase = createServiceClient();

  let query = supabase
    .from("extensions")
    .select(
      "id, name, short_description, icon_url, categories, tags, trust_level, total_downloads, average_rating, review_count, latest_version, created_at, updated_at, publisher:profiles(username, display_name, avatar_url)",
      { count: "exact" },
    )
    .eq("published", true);

  if (q) {
    query = query.textSearch("fts", q, { type: "websearch" });
  }
  if (category) {
    query = query.contains("categories", [category]);
  }
  if (trust) {
    query = query.eq("trust_level", trust);
  }

  switch (sort) {
    case "rating":
      query = query.order("average_rating", { ascending: false });
      break;
    case "newest":
      query = query.order("created_at", { ascending: false });
      break;
    case "updated":
      query = query.order("updated_at", { ascending: false });
      break;
    case "name":
      query = query.order("name", { ascending: true });
      break;
    default:
      query = query.order("total_downloads", { ascending: false });
  }

  query = query.range(offset, offset + limit - 1);

  const { data, count, error } = await query;

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({
    extensions: data,
    total: count,
    page,
    limit,
    total_pages: Math.ceil((count || 0) / limit),
  });
}
