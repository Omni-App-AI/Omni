import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";

/**
 * Tauri v2 updater-compatible endpoint.
 *
 * - 200 + JSON when an update is available
 * - 204 No Content when no update is available
 *
 * Expected query params:
 *   current_version - the running app version (e.g. "0.1.0")
 *   target          - Tauri target triple (e.g. "windows-x86_64")
 *   channel         - release channel (default "stable")
 */
export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const currentVersion = searchParams.get("current_version");
  const target = searchParams.get("target") || searchParams.get("platform");
  const channel = searchParams.get("channel") || "stable";

  if (!currentVersion) {
    return NextResponse.json(
      { error: "current_version is required" },
      { status: 400 },
    );
  }

  const supabase = createServiceClient();

  const { data, error } = await (supabase.from("app_releases") as any)
    .select("*")
    .eq("channel", channel)
    .eq("is_draft", false)
    .order("published_at", { ascending: false })
    .limit(1)
    .single();

  if (error || !data) {
    return new NextResponse(null, { status: 204 });
  }

  // Compare versions
  if (compareSemver(data.version, currentVersion) <= 0) {
    return new NextResponse(null, { status: 204 });
  }

  // If target specified, verify a build exists for it
  if (target && !data.platforms[target]) {
    return new NextResponse(null, { status: 204 });
  }

  // Build Tauri-compatible response
  const platforms: Record<string, { signature: string; url: string }> = {};
  for (const [platformKey, asset] of Object.entries(
    data.platforms as Record<string, { signature: string; url: string }>,
  )) {
    platforms[platformKey] = {
      signature: asset.signature,
      url: asset.url,
    };
  }

  return NextResponse.json(
    {
      version: data.version,
      notes: data.release_notes || "",
      pub_date: data.published_at,
      platforms,
    },
    {
      headers: {
        "Cache-Control": "public, s-maxage=300, stale-while-revalidate=3600",
      },
    },
  );
}

/** Simple semver comparison: >0 if a > b, 0 if equal, <0 if a < b */
function compareSemver(a: string, b: string): number {
  const cleanA = a.replace(/^v/, "");
  const cleanB = b.replace(/^v/, "");

  const [coreA, preA] = cleanA.split("-", 2);
  const [coreB, preB] = cleanB.split("-", 2);

  const partsA = coreA!.split(".").map(Number);
  const partsB = coreB!.split(".").map(Number);

  for (let i = 0; i < 3; i++) {
    const diff = (partsA[i] || 0) - (partsB[i] || 0);
    if (diff !== 0) return diff;
  }

  // A release without pre-release tag is newer than one with
  if (!preA && preB) return 1;
  if (preA && !preB) return -1;
  if (preA && preB) return preA.localeCompare(preB);

  return 0;
}
