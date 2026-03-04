import { serve } from "https://deno.land/std@0.177.0/http/server.ts";
import { scanContent } from "../_shared/signatures.ts";
import type { ScanLayerResult, SignatureMatch } from "../_shared/types.ts";

serve(async (req) => {
  try {
    const { wasm_content, manifest_content } = await req.json();

    // Scan WASM binary strings (extract readable strings from hex-encoded content)
    const wasmMatches = wasm_content ? scanContent(wasm_content) : { matches: [], score: 100 };

    // Scan manifest content
    const manifestMatches = manifest_content ? scanContent(manifest_content) : { matches: [], score: 100 };

    // Combine results
    const allMatches: SignatureMatch[] = [
      ...wasmMatches.matches.map((m) => ({ pattern_id: m.id, ...m })),
      ...manifestMatches.matches.map((m) => ({ pattern_id: m.id, ...m })),
    ];

    // Use the worst score
    const score = Math.min(wasmMatches.score, manifestMatches.score);

    const result: ScanLayerResult = {
      score,
      details: allMatches,
    };

    return new Response(JSON.stringify(result), {
      headers: { "Content-Type": "application/json" },
    });
  } catch (error) {
    return new Response(
      JSON.stringify({ score: 0, details: [], error: String(error) }),
      { status: 500, headers: { "Content-Type": "application/json" } },
    );
  }
});
