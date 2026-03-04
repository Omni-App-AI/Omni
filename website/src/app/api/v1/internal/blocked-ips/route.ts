import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";

/**
 * Internal endpoint for middleware to fetch blocked IP hashes.
 * Protected by INTERNAL_API_KEY -- not for public consumption.
 */
export async function GET(request: NextRequest) {
  const key = request.headers.get("x-internal-key");
  const expected = process.env.INTERNAL_API_KEY;

  if (!expected || key !== expected) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const supabase = createServiceClient();
  const { data } = await (supabase
    .from("ip_reputation") as any)
    .select("ip_hash")
    .eq("blocked", true);

  const ips = data ? data.map((row: any) => row.ip_hash) : [];

  return NextResponse.json({ ips });
}
