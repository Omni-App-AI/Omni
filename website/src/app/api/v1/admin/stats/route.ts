import { NextResponse } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";

export async function GET() {
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

  // Gather stats
  const last24h = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString();
  const last7d = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString();

  const [
    pendingFlags,
    recentEvents,
    blockedIPs,
    activeBans,
    rateLimitsToday,
    turnstileFailsToday,
    honeypotsCaughtToday,
    spamBlockedToday,
    extensionsUnderReview,
    extensionsTakenDown,
  ] = await Promise.all([
    service.from("content_flags").select("*", { count: "exact", head: true }).eq("status", "pending"),
    service.from("security_events").select("*", { count: "exact", head: true }).gte("created_at", last24h),
    service.from("ip_reputation").select("*", { count: "exact", head: true }).eq("blocked", true),
    service.from("user_bans").select("*", { count: "exact", head: true }).is("revoked_at", null),
    service.from("security_events").select("*", { count: "exact", head: true }).eq("event_type", "rate_limited").gte("created_at", last24h),
    service.from("security_events").select("*", { count: "exact", head: true }).eq("event_type", "turnstile_fail").gte("created_at", last24h),
    service.from("security_events").select("*", { count: "exact", head: true }).eq("event_type", "honeypot_caught").gte("created_at", last24h),
    service.from("security_events").select("*", { count: "exact", head: true }).eq("event_type", "spam_blocked").gte("created_at", last24h),
    service.from("extensions").select("*", { count: "exact", head: true }).eq("moderation_status", "under_review"),
    service.from("extensions").select("*", { count: "exact", head: true }).eq("moderation_status", "taken_down"),
  ]);

  return NextResponse.json({
    pending_flags: pendingFlags.count || 0,
    security_events_24h: recentEvents.count || 0,
    blocked_ips: blockedIPs.count || 0,
    active_bans: activeBans.count || 0,
    rate_limits_24h: rateLimitsToday.count || 0,
    turnstile_fails_24h: turnstileFailsToday.count || 0,
    honeypots_caught_24h: honeypotsCaughtToday.count || 0,
    spam_blocked_24h: spamBlockedToday.count || 0,
    extensions_under_review: extensionsUnderReview.count || 0,
    extensions_taken_down: extensionsTakenDown.count || 0,
  });
}
