import { serve } from "https://deno.land/std@0.177.0/http/server.ts";
import { getServiceClient } from "../_shared/supabase.ts";
import type { FullScanResult, ExtensionVersionRow } from "../_shared/types.ts";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;

async function callLayer(functionName: string, payload: unknown): Promise<unknown> {
  const response = await fetch(`${SUPABASE_URL}/functions/v1/${functionName}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")}`,
    },
    body: JSON.stringify(payload),
  });
  return response.json();
}

async function extractWasmStrings(wasmUrl: string): Promise<string> {
  try {
    const response = await fetch(wasmUrl);
    if (!response.ok) return "";
    const buffer = await response.arrayBuffer();
    const bytes = new Uint8Array(buffer);

    // Extract printable ASCII strings (length >= 4)
    const strings: string[] = [];
    let current = "";
    for (const byte of bytes) {
      if (byte >= 32 && byte <= 126) {
        current += String.fromCharCode(byte);
      } else {
        if (current.length >= 4) strings.push(current);
        current = "";
      }
    }
    if (current.length >= 4) strings.push(current);

    return strings.join("\n").substring(0, 50000); // Cap at 50KB
  } catch {
    return "";
  }
}

serve(async (req) => {
  const startTime = Date.now();

  try {
    const { version_id } = await req.json();
    const supabase = getServiceClient();

    // Fetch version details
    const { data: version, error: verError } = await supabase
      .from("extension_versions")
      .select("id, extension_id, version, wasm_url, wasm_size_bytes, permissions, tools, manifest")
      .eq("id", version_id)
      .single();

    if (verError || !version) {
      throw new Error(`Version not found: ${version_id}`);
    }

    const ver = version as ExtensionVersionRow;

    // Fetch extension metadata
    const { data: extension } = await supabase
      .from("extensions")
      .select("name, description, categories")
      .eq("id", ver.extension_id)
      .single();

    // Mark as scanning
    await supabase
      .from("extension_versions")
      .update({ scan_status: "scanning" })
      .eq("id", version_id);

    // Extract strings from WASM for analysis
    const wasmStrings = await extractWasmStrings(ver.wasm_url);

    // ===== Layer 1: Signature Scanning =====
    const sigResult = await callLayer("scan-signatures", {
      wasm_content: wasmStrings,
      manifest_content: JSON.stringify(ver.manifest),
    }) as { score: number; details: unknown[] };

    // Short-circuit if critical signature match
    if (sigResult.score < 20) {
      const result = buildResult(sigResult, { score: 0, details: [] }, { score: 0, details: [] }, { score: 0, details: [] }, startTime);
      result.verdict = "malicious";
      result.overall_score = sigResult.score;
      await saveScanResult(supabase, version_id, ver, result);
      return respond(result);
    }

    // ===== Layer 2: Heuristic Analysis =====
    const heuResult = await callLayer("scan-heuristics", {
      extension_id: ver.extension_id,
      categories: extension?.categories || [],
      permissions: ver.permissions,
      tools: ver.tools,
      manifest: ver.manifest,
      wasm_size_bytes: ver.wasm_size_bytes,
    }) as { score: number; details: unknown[] };

    // ===== Layer 3: AI Code Review =====
    const aiResult = await callLayer("scan-ai", {
      extension_id: ver.extension_id,
      name: extension?.name || ver.extension_id,
      description: extension?.description || "",
      permissions: ver.permissions,
      tools: ver.tools,
      manifest_json: JSON.stringify(ver.manifest, null, 2),
      wasm_strings: wasmStrings.substring(0, 10000),
    }) as { score: number; details: unknown[]; analysis?: string; flags?: unknown[] };

    // ===== Layer 4: Sandbox Execution =====
    const sandboxResult = await callLayer("scan-sandbox", {
      wasm_url: ver.wasm_url,
      tools: ver.tools,
      max_memory_mb: 64,
      max_cpu_ms: 5000,
    }) as { score: number; details: unknown };

    // ===== Calculate Overall Score =====
    const result = buildResult(
      sigResult,
      heuResult,
      { ...aiResult, analysis: (aiResult as Record<string, unknown>).analysis as string, flags: (aiResult as Record<string, unknown>).flags as unknown[] },
      { score: (sandboxResult as Record<string, unknown>).score as number, details: sandboxResult.details },
      startTime,
    );

    // Save result
    await saveScanResult(supabase, version_id, ver, result);

    return respond(result);
  } catch (error) {
    return new Response(
      JSON.stringify({ error: String(error), verdict: "error" }),
      { status: 500, headers: { "Content-Type": "application/json" } },
    );
  }
});

function buildResult(
  sig: { score: number; details: unknown[] },
  heu: { score: number; details: unknown[] },
  ai: { score: number; details: unknown[]; analysis?: string; flags?: unknown[] },
  sandbox: { score: number; details: unknown },
  startTime: number,
): FullScanResult {
  // Weighted score: signatures 30%, heuristics 25%, AI 30%, sandbox 15%
  const overall = sig.score * 0.30 + heu.score * 0.25 + ai.score * 0.30 + (sandbox.score || 50) * 0.15;

  let verdict: "clean" | "suspicious" | "malicious" | "error";
  let autoApproved = false;

  const minLayerScore = Math.min(sig.score, heu.score, ai.score, sandbox.score || 50);

  if (overall >= 80 && minLayerScore >= 60) {
    verdict = "clean";
    autoApproved = true;
  } else if (overall >= 50) {
    verdict = "suspicious";
  } else {
    verdict = "malicious";
  }

  return {
    signature_score: sig.score,
    signature_matches: sig.details as FullScanResult["signature_matches"],
    heuristic_score: heu.score,
    heuristic_details: heu.details as FullScanResult["heuristic_details"],
    ai_score: ai.score,
    ai_analysis: ai.analysis || "",
    ai_flags: (ai.flags || []) as FullScanResult["ai_flags"],
    sandbox_score: sandbox.score || 50,
    sandbox_results: (sandbox.details || {}) as FullScanResult["sandbox_results"],
    overall_score: Math.round(overall * 100) / 100,
    verdict,
    auto_approved: autoApproved,
    scan_duration_ms: Date.now() - startTime,
  };
}

async function saveScanResult(
  supabase: ReturnType<typeof getServiceClient>,
  versionId: string,
  ver: ExtensionVersionRow,
  result: FullScanResult,
) {
  // Save scan result
  await supabase.from("scan_results").insert({
    version_id: versionId,
    extension_id: ver.extension_id,
    version: ver.version,
    ...result,
  });

  // Update version status
  const scanStatus = result.verdict === "clean" ? "passed" :
    result.verdict === "suspicious" ? "flagged" : "failed";

  await supabase
    .from("extension_versions")
    .update({
      scan_status: scanStatus,
      scan_score: result.overall_score,
      scan_completed_at: new Date().toISOString(),
      published: result.auto_approved,
    })
    .eq("id", versionId);
}

function respond(result: FullScanResult) {
  return new Response(JSON.stringify(result), {
    headers: { "Content-Type": "application/json" },
  });
}
