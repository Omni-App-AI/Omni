import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient, createClient } from "@/lib/supabase/server";
import { slugify } from "@/lib/utils";

export async function GET(request: NextRequest) {
  const supabase = createServiceClient();
  const { searchParams } = new URL(request.url);

  const category = searchParams.get("category");
  const tag = searchParams.get("tag");
  const featured = searchParams.get("featured");
  const search = searchParams.get("q");
  const page = parseInt(searchParams.get("page") || "1", 10);
  const limit = Math.min(parseInt(searchParams.get("limit") || "20", 10), 50);
  const offset = (page - 1) * limit;

  let query = supabase
    .from("blog_posts")
    .select(
      "*, author:profiles(id, username, display_name, avatar_url)",
      { count: "exact" },
    )
    .eq("published", true)
    .order("published_at", { ascending: false });

  if (category) {
    query = query.eq("category", category);
  }

  if (tag) {
    query = query.contains("tags", [tag]);
  }

  if (featured === "true") {
    query = query.eq("featured", true);
  }

  if (search) {
    query = query.or(`title.ilike.%${search}%,excerpt.ilike.%${search}%`);
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

export async function POST(request: NextRequest) {
  // Auth check
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) {
    return NextResponse.json({ error: "Authentication required" }, { status: 401 });
  }

  // Moderator check
  const service = createServiceClient();
  const { data: profile } = await service
    .from("profiles")
    .select("is_moderator")
    .eq("id", user.id)
    .single();

  if (!profile || !(profile as any).is_moderator) {
    return NextResponse.json({ error: "Moderator access required" }, { status: 403 });
  }

  let body: Record<string, unknown>;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  const {
    title,
    body: postBody,
    excerpt,
    slug: providedSlug,
    category,
    tags,
    cover_image_url,
    meta_title,
    meta_description,
    og_image_url,
    canonical_url,
    published,
    featured,
    read_time_minutes,
  } = body as {
    title?: string;
    body?: string;
    excerpt?: string;
    slug?: string;
    category?: string;
    tags?: string[];
    cover_image_url?: string;
    meta_title?: string;
    meta_description?: string;
    og_image_url?: string;
    canonical_url?: string;
    published?: boolean;
    featured?: boolean;
    read_time_minutes?: number;
  };

  if (!title || !postBody) {
    return NextResponse.json({ error: "Title and body are required" }, { status: 400 });
  }

  const slug = providedSlug || slugify(title);

  if (!slug) {
    return NextResponse.json({ error: "Could not generate a valid slug" }, { status: 400 });
  }

  // Check slug uniqueness
  const { data: existing } = await service
    .from("blog_posts")
    .select("id")
    .eq("slug", slug)
    .single();

  if (existing) {
    return NextResponse.json({ error: "A post with this slug already exists" }, { status: 409 });
  }

  // If featuring this post, unfeature any existing featured post
  if (featured) {
    await (service.from("blog_posts") as any).update({ featured: false }).eq("featured", true);
  }

  const { data: post, error } = await (service
    .from("blog_posts") as any)
    .insert({
      author_id: user.id,
      slug,
      title,
      body: postBody,
      excerpt: excerpt || null,
      category: category || "general",
      tags: tags || [],
      cover_image_url: cover_image_url || null,
      meta_title: meta_title || null,
      meta_description: meta_description || null,
      og_image_url: og_image_url || null,
      canonical_url: canonical_url || null,
      published: published || false,
      featured: featured || false,
      read_time_minutes: read_time_minutes || Math.max(1, Math.ceil((postBody.length / 1500))),
      published_at: published ? new Date().toISOString() : null,
    })
    .select("*")
    .single();

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({ post }, { status: 201 });
}
