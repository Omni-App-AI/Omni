import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { CATEGORIES } from "@/lib/constants";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const supabase = createServiceClient();

  const { data: extensionData, error } = await supabase
    .from("extensions")
    .select(
      "*, publisher:profiles(username, display_name, avatar_url, verified_publisher)",
    )
    .eq("id", id)
    .eq("published", true)
    .single();

  const extension = extensionData as Record<string, unknown> | null;
  if (error || !extension) {
    return NextResponse.json({ error: "Extension not found" }, { status: 404 });
  }

  // Get latest published version using the explicit latest_version field
  const latestVersionStr = extension.latest_version as string | null;
  let latestVersionQuery = supabase
    .from("extension_versions")
    .select("version, changelog, permissions, tools, manifest, min_omni_version, wasm_size_bytes, checksum, scan_status, scan_score, created_at")
    .eq("extension_id", id)
    .eq("published", true);

  if (latestVersionStr) {
    latestVersionQuery = latestVersionQuery.eq("version", latestVersionStr);
  } else {
    // Fallback if latest_version is unset
    latestVersionQuery = latestVersionQuery.order("created_at", { ascending: false }).limit(1);
  }

  const { data: latestVersion } = await latestVersionQuery.single();

  return NextResponse.json({
    ...extension,
    latest: latestVersion,
  });
}

const VALID_LICENSES = ["MIT", "Apache-2.0", "GPL-3.0", "BSD-3-Clause", "proprietary"];
const CATEGORY_IDS = CATEGORIES.map((c) => c.id);

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;

  // Auth check
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  // Verify ownership
  const service = createServiceClient();
  const { data: ext } = await service
    .from("extensions")
    .select("publisher_id")
    .eq("id", id)
    .single();

  if (!ext || (ext as any).publisher_id !== user.id) {
    return NextResponse.json({ error: "Extension not found" }, { status: 404 });
  }

  const body = await request.json();
  const update: Record<string, unknown> = {};

  // Whitelist and validate each field
  if ("name" in body) {
    if (typeof body.name !== "string" || body.name.length < 1 || body.name.length > 100) {
      return NextResponse.json({ error: "name must be 1-100 characters" }, { status: 400 });
    }
    update.name = body.name;
  }

  if ("short_description" in body) {
    if (typeof body.short_description !== "string" || body.short_description.length < 1 || body.short_description.length > 160) {
      return NextResponse.json({ error: "short_description must be 1-160 characters" }, { status: 400 });
    }
    update.short_description = body.short_description;
  }

  if ("description" in body) {
    if (typeof body.description !== "string" || body.description.length < 1 || body.description.length > 10000) {
      return NextResponse.json({ error: "description must be 1-10000 characters" }, { status: 400 });
    }
    update.description = body.description;
  }

  if ("icon_url" in body) {
    if (body.icon_url !== null && typeof body.icon_url !== "string") {
      return NextResponse.json({ error: "icon_url must be a string or null" }, { status: 400 });
    }
    update.icon_url = body.icon_url;
  }

  if ("banner_url" in body) {
    if (body.banner_url !== null && typeof body.banner_url !== "string") {
      return NextResponse.json({ error: "banner_url must be a string or null" }, { status: 400 });
    }
    update.banner_url = body.banner_url;
  }

  if ("screenshots" in body) {
    if (!Array.isArray(body.screenshots) || body.screenshots.length > 5) {
      return NextResponse.json({ error: "screenshots must be an array of up to 5 URLs" }, { status: 400 });
    }
    if (!body.screenshots.every((s: unknown) => typeof s === "string")) {
      return NextResponse.json({ error: "Each screenshot must be a URL string" }, { status: 400 });
    }
    update.screenshots = body.screenshots;
  }

  if ("homepage" in body) {
    if (body.homepage !== null && body.homepage !== "" && typeof body.homepage !== "string") {
      return NextResponse.json({ error: "homepage must be a URL string or null" }, { status: 400 });
    }
    update.homepage = body.homepage || null;
  }

  if ("repository" in body) {
    if (body.repository !== null && body.repository !== "" && typeof body.repository !== "string") {
      return NextResponse.json({ error: "repository must be a URL string or null" }, { status: 400 });
    }
    update.repository = body.repository || null;
  }

  if ("license" in body) {
    if (!VALID_LICENSES.includes(body.license)) {
      return NextResponse.json({ error: `license must be one of: ${VALID_LICENSES.join(", ")}` }, { status: 400 });
    }
    update.license = body.license;
  }

  if ("categories" in body) {
    if (!Array.isArray(body.categories) || body.categories.length > 5) {
      return NextResponse.json({ error: "categories must be an array of up to 5" }, { status: 400 });
    }
    if (!body.categories.every((c: unknown) => CATEGORY_IDS.includes(c as any))) {
      return NextResponse.json({ error: "Invalid category" }, { status: 400 });
    }
    update.categories = body.categories;
  }

  if ("tags" in body) {
    if (!Array.isArray(body.tags) || body.tags.length > 10) {
      return NextResponse.json({ error: "tags must be an array of up to 10" }, { status: 400 });
    }
    if (!body.tags.every((t: unknown) => typeof t === "string" && (t as string).length <= 30)) {
      return NextResponse.json({ error: "Each tag must be a string up to 30 characters" }, { status: 400 });
    }
    update.tags = body.tags;
  }

  if (Object.keys(update).length === 0) {
    return NextResponse.json({ error: "No valid fields to update" }, { status: 400 });
  }

  update.updated_at = new Date().toISOString();

  const { data, error: updateError } = await (service.from("extensions") as any)
    .update(update)
    .eq("id", id)
    .select()
    .single();

  if (updateError) {
    return NextResponse.json({ error: updateError.message }, { status: 500 });
  }

  return NextResponse.json(data);
}
