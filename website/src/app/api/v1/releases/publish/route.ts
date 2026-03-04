import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";
import { extractIP, hashIP } from "@/lib/anti-bot/ip-utils";
import {
  RATE_LIMITS,
  checkRateLimit,
  recordRateLimitHit,
  rateLimitHeaders,
} from "@/lib/anti-bot/rate-limits";

const VALID_PLATFORMS = [
  "windows-x86_64",
  "darwin-x86_64",
  "darwin-aarch64",
  "linux-x86_64",
];

const VALID_CHANNELS = ["stable", "beta", "nightly"];

const SEMVER_RE = /^\d+\.\d+\.\d+(-[\w.]+)?$/;

interface PlatformAsset {
  url: string;
  signature: string;
  size_bytes: number;
  asset_name: string;
  installer_type?: string;
}

interface PublishBody {
  version: string;
  channel?: string;
  release_notes?: string;
  platforms: Record<string, PlatformAsset>;
  github_release_id?: number;
  is_prerelease?: boolean;
  min_supported_version?: string;
}

export async function POST(request: NextRequest) {
  // Auth: dedicated CI API key
  const authHeader = request.headers.get("authorization");
  if (!authHeader?.startsWith("Bearer ")) {
    return NextResponse.json({ error: "Missing API key" }, { status: 401 });
  }

  const apiKey = authHeader.substring(7);
  const expectedKey = process.env.RELEASE_PUBLISH_API_KEY;
  if (!expectedKey || apiKey !== expectedKey) {
    return NextResponse.json({ error: "Invalid API key" }, { status: 401 });
  }

  // Rate limit
  const ip = extractIP(request);
  const ipHash = await hashIP(ip);
  const config = RATE_LIMITS.release_publish!;
  const rlResult = await checkRateLimit(
    `ip:${ipHash}`,
    "release_publish",
    config.max as number,
    config.window,
  );
  if (!rlResult.allowed) {
    return NextResponse.json(
      { error: "Rate limit exceeded" },
      { status: 429, headers: rateLimitHeaders(rlResult) },
    );
  }
  await recordRateLimitHit(`ip:${ipHash}`, "release_publish");

  // Parse body
  let body: PublishBody;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  // Validate version
  if (!body.version || !SEMVER_RE.test(body.version)) {
    return NextResponse.json(
      { error: "Invalid semver version" },
      { status: 400 },
    );
  }

  // Validate channel
  const channel = body.channel || "stable";
  if (!VALID_CHANNELS.includes(channel)) {
    return NextResponse.json({ error: "Invalid channel" }, { status: 400 });
  }

  // Validate platforms
  if (!body.platforms || Object.keys(body.platforms).length === 0) {
    return NextResponse.json(
      { error: "At least one platform required" },
      { status: 400 },
    );
  }

  for (const [platform, asset] of Object.entries(body.platforms)) {
    if (!VALID_PLATFORMS.includes(platform)) {
      return NextResponse.json(
        { error: `Unknown platform: ${platform}` },
        { status: 400 },
      );
    }
    if (!asset.url || !asset.signature) {
      return NextResponse.json(
        { error: `Platform ${platform} missing url or signature` },
        { status: 400 },
      );
    }
  }

  // Upsert into app_releases (idempotent for CI re-runs)
  const supabase = createServiceClient();

  const { data, error } = await (supabase.from("app_releases") as any)
    .upsert(
      {
        version: body.version,
        channel,
        release_notes: body.release_notes || "",
        platforms: body.platforms,
        github_release_id: body.github_release_id || null,
        is_draft: false,
        is_prerelease: body.is_prerelease || false,
        published_at: new Date().toISOString(),
        min_supported_version: body.min_supported_version || null,
      },
      { onConflict: "version" },
    )
    .select()
    .single();

  if (error) {
    return NextResponse.json(
      { error: `Failed to publish release: ${error.message}` },
      { status: 500 },
    );
  }

  return NextResponse.json(
    {
      message: "Release published",
      version: body.version,
      channel,
      id: data.id,
    },
    { status: 201 },
  );
}
