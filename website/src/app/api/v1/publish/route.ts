import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";
import { createHash } from "crypto";
import { extractIP, hashIP, incrementIPCounter } from "@/lib/anti-bot/ip-utils";
import { RATE_LIMITS, checkRateLimit, recordRateLimitHit, rateLimitHeaders } from "@/lib/anti-bot/rate-limits";
import { logSecurityEvent } from "@/lib/anti-bot/security-logger";

export async function POST(request: NextRequest) {
  // Authenticate via API key
  const authHeader = request.headers.get("authorization");
  if (!authHeader?.startsWith("Bearer ")) {
    return NextResponse.json({ error: "Missing API key" }, { status: 401 });
  }

  const apiKey = authHeader.substring(7);
  if (!apiKey.startsWith("omni_pk_")) {
    return NextResponse.json({ error: "Invalid API key format" }, { status: 401 });
  }

  const supabase = createServiceClient();

  // Hash the provided key and look it up
  const keyHash = createHash("sha256").update(apiKey).digest("hex");
  const keyPrefix = apiKey.substring(0, 16);

  const { data: keyData } = await supabase
    .from("api_keys")
    .select("id, user_id, permissions, revoked, expires_at")
    .eq("key_hash", keyHash)
    .eq("key_prefix", keyPrefix)
    .single();
  const keyRecord = keyData as { id: string; user_id: string; permissions: string[]; revoked: boolean; expires_at: string | null } | null;

  if (!keyRecord || keyRecord.revoked) {
    return NextResponse.json({ error: "Invalid or revoked API key" }, { status: 401 });
  }

  if (keyRecord.expires_at && new Date(keyRecord.expires_at) < new Date()) {
    return NextResponse.json({ error: "API key expired" }, { status: 401 });
  }

  if (!keyRecord.permissions.includes("publish")) {
    return NextResponse.json({ error: "API key lacks publish permission" }, { status: 403 });
  }

  // Rate limit publishing per user
  const ip = extractIP(request);
  const ipHash = await hashIP(ip);
  const rlConfig = RATE_LIMITS.extension_publish!;
  const rlResult = await checkRateLimit(
    `user:${keyRecord.user_id}`,
    "extension_publish",
    rlConfig.max as number,
    rlConfig.window
  );
  if (!rlResult.allowed) {
    await incrementIPCounter(ipHash, "total_rate_limits");
    await logSecurityEvent({
      eventType: "rate_limited",
      actorId: keyRecord.user_id,
      ip,
      metadata: { action: "extension_publish" },
    });
    return NextResponse.json(
      { error: "Publishing rate limit exceeded. Please try again later." },
      { status: 429, headers: rateLimitHeaders(rlResult) }
    );
  }
  await recordRateLimitHit(`user:${keyRecord.user_id}`, "extension_publish");

  // Update last_used_at
  // @ts-expect-error -- Supabase type inference limitation with manual Database type
  await supabase.from("api_keys").update({ last_used_at: new Date().toISOString() }).eq("id", keyRecord.id);

  // Parse multipart form data
  const formData = await request.formData();
  const wasmFile = formData.get("wasm") as File | null;
  const manifestJson = formData.get("manifest") as string | null;

  if (!wasmFile || !manifestJson) {
    return NextResponse.json(
      { error: "Missing required fields: wasm (file), manifest (JSON string)" },
      { status: 400 },
    );
  }

  let manifest: Record<string, unknown>;
  try {
    manifest = JSON.parse(manifestJson);
  } catch {
    return NextResponse.json({ error: "Invalid manifest JSON" }, { status: 400 });
  }

  const ext = manifest.extension as Record<string, string> | undefined;
  if (!ext?.id || !ext?.version || !ext?.name) {
    return NextResponse.json(
      { error: "Manifest must include extension.id, extension.version, and extension.name" },
      { status: 400 },
    );
  }

  // Normalize: ensure the manifest blob's extension.version matches the
  // authoritative ext.version used for the version column and storage path.
  // Publishers may bump the version field in the CLI but forget to regenerate
  // the full manifest JSON, causing a stale version inside the JSONB.
  if (ext.version) {
    (manifest.extension as Record<string, string>).version = ext.version;
  }

  // Check extension ownership
  const { data: existingExtData } = await supabase
    .from("extensions")
    .select("publisher_id")
    .eq("id", ext.id)
    .single();
  const existingExt = existingExtData as { publisher_id: string } | null;

  if (existingExt && existingExt.publisher_id !== keyRecord.user_id) {
    return NextResponse.json({ error: "Extension belongs to another publisher" }, { status: 403 });
  }

  // Upload WASM to storage
  const wasmBuffer = await wasmFile.arrayBuffer();
  const checksum = "sha256:" + createHash("sha256").update(Buffer.from(wasmBuffer)).digest("hex");
  const wasmPath = `${ext.id}/${ext.version}/${wasmFile.name}`;

  const { error: uploadError } = await supabase.storage
    .from("extension-wasm")
    .upload(wasmPath, wasmFile);

  if (uploadError) {
    return NextResponse.json({ error: `Upload failed: ${uploadError.message}` }, { status: 500 });
  }

  const { data: urlData } = supabase.storage
    .from("extension-wasm")
    .getPublicUrl(wasmPath);

  // Create or update extension
  if (!existingExt) {
    const { error: extError } = await supabase.from("extensions").insert({
      id: ext.id,
      publisher_id: keyRecord.user_id,
      name: ext.name,
      description: ext.description || ext.name,
      short_description: (ext.description || ext.name).substring(0, 160),
      categories: (manifest.extension as Record<string, unknown>)?.categories as string[] || [],
      published: true,
      license: ext.license || null,
      tags: (manifest.extension as Record<string, unknown>)?.tags as string[] || [],
      latest_version: ext.version,
    } as any);

    if (extError) {
      return NextResponse.json({ error: `Extension creation failed: ${extError.message}` }, { status: 500 });
    }
  } else {
    // Update existing extension metadata on new version publish
    const { error: updateError } = await (supabase.from("extensions") as any)
      .update({
        name: ext.name,
        description: ext.description || ext.name,
        short_description: (ext.description || ext.name).substring(0, 160),
        latest_version: ext.version,
        published: true,
        license: ext.license || null,
      })
      .eq("id", ext.id);

    if (updateError) {
      return NextResponse.json({ error: `Extension update failed: ${updateError.message}` }, { status: 500 });
    }
  }

  // Create version (published immediately; scan can revoke later)
  const { data: version, error: verError } = await supabase
    .from("extension_versions")
    .insert({
      extension_id: ext.id,
      version: ext.version,
      wasm_url: urlData.publicUrl,
      wasm_size_bytes: wasmFile.size,
      checksum,
      manifest,
      permissions: manifest.permissions || [],
      tools: manifest.tools || [],
      published: true,
    } as any)
    .select()
    .single();

  if (verError) {
    return NextResponse.json({ error: `Version creation failed: ${verError.message}` }, { status: 500 });
  }

  return NextResponse.json({
    message: "Extension published. Security scan queued.",
    extension_id: ext.id,
    version: ext.version,
    version_id: (version as any).id,
    scan_status: "pending",
  }, { status: 201 });
}
