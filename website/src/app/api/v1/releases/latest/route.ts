import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";
import { extractIP, hashIP } from "@/lib/anti-bot/ip-utils";
import {
  RATE_LIMITS,
  checkRateLimit,
  recordRateLimitHit,
} from "@/lib/anti-bot/rate-limits";

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const channel = searchParams.get("channel") || "stable";
  const platform = searchParams.get("platform");
  const download = searchParams.get("download");

  const supabase = createServiceClient();

  // Fetch latest non-draft release for the channel
  const { data, error } = await (supabase.from("app_releases") as any)
    .select("*")
    .eq("channel", channel)
    .eq("is_draft", false)
    .order("published_at", { ascending: false })
    .limit(1)
    .single();

  if (error || !data) {
    return NextResponse.json(
      { available: false, message: "No releases available yet" },
      { status: 404 },
    );
  }

  // Track download if requested (fire-and-forget, never blocks the response)
  if (download === "true" && platform) {
    const ip = extractIP(request);
    const ipHash = await hashIP(ip);

    const config = RATE_LIMITS.app_download!;
    const rlResult = await checkRateLimit(
      `ip:${ipHash}`,
      "app_download",
      config.max as number,
      config.window,
    );

    if (rlResult.allowed) {
      await recordRateLimitHit(`ip:${ipHash}`, "app_download");
      await (supabase.from("release_downloads") as any).insert({
        release_id: data.id,
        platform,
        ip_hash: ipHash,
        user_agent: request.headers.get("user-agent") || null,
      });
    }
  }

  // If a platform filter is specified, verify it exists
  if (platform && !data.platforms[platform]) {
    return NextResponse.json(
      {
        available: false,
        message: `No build available for platform: ${platform}`,
        latest_version: data.version,
      },
      { status: 404 },
    );
  }

  return NextResponse.json(
    {
      version: data.version,
      channel: data.channel,
      release_notes: data.release_notes,
      published_at: data.published_at,
      is_prerelease: data.is_prerelease,
      platforms: data.platforms,
    },
    {
      headers: {
        "Cache-Control": "public, s-maxage=300, stale-while-revalidate=3600",
      },
    },
  );
}
