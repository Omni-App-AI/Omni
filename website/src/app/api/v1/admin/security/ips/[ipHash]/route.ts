import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { logSecurityEvent } from "@/lib/anti-bot/security-logger";

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ ipHash: string }> },
) {
  const { ipHash } = await params;

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

  const body = await request.json();
  const { blocked, notes } = body as { blocked?: boolean; notes?: string };

  if (blocked === undefined) {
    return NextResponse.json({ error: "blocked field is required" }, { status: 400 });
  }

  // Upsert the IP reputation record
  const { data: existing } = await (service
    .from("ip_reputation") as any)
    .select("ip_hash")
    .eq("ip_hash", ipHash)
    .single();

  if (existing) {
    await (service
      .from("ip_reputation") as any)
      .update({
        blocked,
        notes: notes || null,
        last_seen_at: new Date().toISOString(),
      })
      .eq("ip_hash", ipHash);
  } else {
    await (service.from("ip_reputation") as any).insert({
      ip_hash: ipHash,
      blocked,
      notes: notes || null,
      risk_score: blocked ? 100 : 0,
    });
  }

  await logSecurityEvent({
    eventType: blocked ? "ip_blocked" : "ip_unblocked",
    actorId: user.id,
    metadata: { ip_hash: ipHash, notes },
  });

  return NextResponse.json({ success: true, ip_hash: ipHash, blocked });
}
