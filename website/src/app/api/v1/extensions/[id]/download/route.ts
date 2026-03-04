import { NextResponse } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";
import { extractIP, hashIP } from "@/lib/anti-bot/ip-utils";
import { RATE_LIMITS, checkRateLimit, recordRateLimitHit, rateLimitHeaders } from "@/lib/anti-bot/rate-limits";

export async function GET(
  request: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const { searchParams } = new URL(request.url);

  // Rate limit downloads by IP
  const ip = extractIP(request);
  const ipHash = await hashIP(ip);
  const config = RATE_LIMITS.download!;
  const rlResult = await checkRateLimit(`ip:${ipHash}`, "download", config.max as number, config.window);
  if (!rlResult.allowed) {
    return NextResponse.json(
      { error: "Too many download requests. Please try again later." },
      { status: 429, headers: rateLimitHeaders(rlResult) }
    );
  }
  await recordRateLimitHit(`ip:${ipHash}`, "download");
  const version = searchParams.get("version");

  const supabase = createServiceClient();

  // Determine which version to serve
  let targetVersion = version;

  if (!targetVersion) {
    // Use the extensions table's latest_version field (explicitly set by publish endpoint)
    // instead of relying on created_at ordering which can be unreliable
    const { data: extData } = await supabase
      .from("extensions")
      .select("latest_version")
      .eq("id", id)
      .single();

    targetVersion = (extData as { latest_version: string | null } | null)?.latest_version ?? null;
  }

  let query = supabase
    .from("extension_versions")
    .select("id, version, wasm_url, extension_id, scan_status")
    .eq("extension_id", id)
    .eq("published", true);

  if (targetVersion) {
    query = query.eq("version", targetVersion);
  } else {
    // Final fallback if latest_version is unset
    query = query.order("created_at", { ascending: false }).limit(1);
  }

  const { data: verData, error } = await query.single();
  const ver = verData as { id: string; version: string; wasm_url: string; extension_id: string; scan_status: string } | null;

  if (error || !ver) {
    return NextResponse.json({ error: "Version not found" }, { status: 404 });
  }

  // Block downloads of versions that failed security scan
  if (ver.scan_status === "failed") {
    return NextResponse.json({ error: "This version failed security review and cannot be downloaded" }, { status: 403 });
  }

  // Track download (reuse ipHash from rate-limit block above)
  await supabase.from("downloads").insert({
    extension_id: id,
    version: ver.version,
    ip_hash: ipHash,
    source: "website",
  } as any);

  // Extract storage path from the public URL and create a signed URL
  // wasm_url format: https://{ref}.supabase.co/storage/v1/object/public/extension-wasm/{path}
  const bucketPrefix = "/storage/v1/object/public/extension-wasm/";
  const urlObj = new URL(ver.wasm_url);
  const pathIndex = urlObj.pathname.indexOf(bucketPrefix);

  if (pathIndex !== -1) {
    const storagePath = decodeURIComponent(urlObj.pathname.substring(pathIndex + bucketPrefix.length));
    const { data: signedData, error: signError } = await supabase.storage
      .from("extension-wasm")
      .createSignedUrl(storagePath, 300); // 5 min expiry

    if (!signError && signedData?.signedUrl) {
      return NextResponse.redirect(signedData.signedUrl);
    }
  }

  // Fallback: try the public URL directly (works if bucket is public)
  return NextResponse.redirect(ver.wasm_url);
}
