import { serve } from "https://deno.land/std@0.177.0/http/server.ts";
import { getServiceClient } from "../_shared/supabase.ts";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;

serve(async (_req) => {
  try {
    const supabase = getServiceClient();

    // Find all published versions that haven't been scanned in the last 7 days
    const sevenDaysAgo = new Date();
    sevenDaysAgo.setDate(sevenDaysAgo.getDate() - 7);

    const { data: versions, error } = await supabase
      .from("extension_versions")
      .select("id, extension_id, version")
      .eq("published", true)
      .or(`scan_completed_at.is.null,scan_completed_at.lt.${sevenDaysAgo.toISOString()}`);

    if (error) {
      throw new Error(`Query error: ${error.message}`);
    }

    const results = [];

    for (const version of versions || []) {
      try {
        // Trigger scan for each version
        const response = await fetch(`${SUPABASE_URL}/functions/v1/scan-extension`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")}`,
          },
          body: JSON.stringify({ version_id: version.id }),
        });

        results.push({
          version_id: version.id,
          extension_id: version.extension_id,
          version: version.version,
          status: response.ok ? "queued" : "failed",
        });
      } catch (err) {
        results.push({
          version_id: version.id,
          extension_id: version.extension_id,
          version: version.version,
          status: "error",
          error: String(err),
        });
      }
    }

    return new Response(
      JSON.stringify({
        message: `Rescan queued for ${results.length} versions`,
        results,
      }),
      { headers: { "Content-Type": "application/json" } },
    );
  } catch (error) {
    return new Response(
      JSON.stringify({ error: String(error) }),
      { status: 500, headers: { "Content-Type": "application/json" } },
    );
  }
});
