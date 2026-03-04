import { NextResponse } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const supabase = createServiceClient();

  const { data: versions, error } = await supabase
    .from("extension_versions")
    .select("id, version, changelog, min_omni_version, wasm_size_bytes, permissions, tools, scan_status, scan_score, created_at")
    .eq("extension_id", id)
    .eq("published", true)
    .order("created_at", { ascending: false });

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 500 });
  }

  return NextResponse.json({ versions });
}
