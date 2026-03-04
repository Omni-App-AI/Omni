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

  // Query params
  const { searchParams } = new URL(request.url);
  const status = searchParams.get("status") || "pending";
  const contentType = searchParams.get("content_type");
  const page = parseInt(searchParams.get("page") || "1", 10);
  const limit = Math.min(parseInt(searchParams.get("limit") || "20", 10), 50);
  const offset = (page - 1) * limit;

  // Fetch flags without FK join (content_flags FKs point to auth.users, not profiles)
  let query = service
    .from("content_flags")
    .select("*", { count: "exact" })
    .eq("status", status)
    .order("spam_score", { ascending: false, nullsFirst: false })
    .order("created_at", { ascending: false });

  if (contentType) {
    query = query.eq("content_type", contentType);
  }

  const { data: flags, error, count } = await query.range(offset, offset + limit - 1);

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  // Collect unique reporter/moderator IDs and look up their profiles
  const userIds = new Set<string>();
  for (const flag of (flags || [])) {
    const f = flag as any;
    if (f.reporter_id) userIds.add(f.reporter_id);
    if (f.moderator_id) userIds.add(f.moderator_id);
  }

  const profileMap = new Map<string, { username: string; display_name: string }>();
  if (userIds.size > 0) {
    const { data: profiles } = await service
      .from("profiles")
      .select("id, username, display_name")
      .in("id", Array.from(userIds));

    for (const p of (profiles || [])) {
      profileMap.set((p as any).id, {
        username: (p as any).username,
        display_name: (p as any).display_name,
      });
    }
  }

  // Attach reporter/moderator profile info to each flag
  const enrichedFlags = (flags || []).map((flag: any) => ({
    ...flag,
    reporter: flag.reporter_id ? profileMap.get(flag.reporter_id) || null : null,
    moderator: flag.moderator_id ? profileMap.get(flag.moderator_id) || null : null,
  }));

  return NextResponse.json({
    flags: enrichedFlags,
    total: count || 0,
    page,
    limit,
    pages: Math.ceil((count || 0) / limit),
  });
}
