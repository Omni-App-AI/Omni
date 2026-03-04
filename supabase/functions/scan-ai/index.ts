import { serve } from "https://deno.land/std@0.177.0/http/server.ts";
import type { ScanLayerResult, AiFlag } from "../_shared/types.ts";

const ANTHROPIC_API_KEY = Deno.env.get("ANTHROPIC_API_KEY");

interface AiScanInput {
  extension_id: string;
  name: string;
  description: string;
  permissions: Array<{ capability: string; scope?: unknown; reason?: string }>;
  tools: Array<{ name: string; description: string }>;
  manifest_json: string;
  wasm_strings: string; // Extracted readable strings from WASM binary
}

async function analyzeWithClaude(input: AiScanInput): Promise<{ score: number; analysis: string; flags: AiFlag[] }> {
  const prompt = `You are a security auditor reviewing a WebAssembly extension for the Omni AI agent platform.

EXTENSION INFO:
- ID: ${input.extension_id}
- Name: ${input.name}
- Description: ${input.description}

PERMISSIONS REQUESTED:
${input.permissions.map((p) => `- ${p.capability}${p.reason ? ` (reason: ${p.reason})` : ""}`).join("\n")}

TOOLS PROVIDED:
${input.tools.map((t) => `- ${t.name}: ${t.description}`).join("\n")}

MANIFEST:
${input.manifest_json}

EXTRACTED STRINGS FROM WASM BINARY:
${input.wasm_strings.substring(0, 8000)}

---

Analyze this extension for security concerns. Evaluate:
1. Do the requested permissions match the stated purpose?
2. Are there signs of data exfiltration (collecting user data and sending it out)?
3. Are there hidden capabilities not mentioned in the description?
4. Do the extracted strings reveal malicious intent (URLs, commands, encoded payloads)?
5. Is the extension description honest about what it does?

Respond in this exact JSON format:
{
  "score": <number 0-100, where 100 is completely safe>,
  "analysis": "<2-3 sentence summary>",
  "flags": [
    {
      "severity": "low|medium|high|critical",
      "finding": "<what was found>",
      "recommendation": "<what to do about it>"
    }
  ]
}

Only output valid JSON, no markdown or explanation.`;

  const response = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": ANTHROPIC_API_KEY!,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify({
      model: "claude-sonnet-4-5-20250929",
      max_tokens: 1024,
      messages: [{ role: "user", content: prompt }],
    }),
  });

  if (!response.ok) {
    const errText = await response.text();
    throw new Error(`Anthropic API error: ${response.status} ${errText}`);
  }

  const data = await response.json();
  const text = data.content[0].text;

  // Parse JSON response
  const parsed = JSON.parse(text);

  return {
    score: Math.min(100, Math.max(0, parsed.score)),
    analysis: parsed.analysis,
    flags: parsed.flags || [],
  };
}

serve(async (req) => {
  try {
    if (!ANTHROPIC_API_KEY) {
      return new Response(
        JSON.stringify({ score: 50, details: [], error: "ANTHROPIC_API_KEY not configured" }),
        { headers: { "Content-Type": "application/json" } },
      );
    }

    const input: AiScanInput = await req.json();
    const result = await analyzeWithClaude(input);

    const layerResult: ScanLayerResult & { analysis: string; flags: AiFlag[] } = {
      score: result.score,
      details: result.flags,
      analysis: result.analysis,
      flags: result.flags,
    };

    return new Response(JSON.stringify(layerResult), {
      headers: { "Content-Type": "application/json" },
    });
  } catch (error) {
    return new Response(
      JSON.stringify({
        score: 50,
        details: [],
        analysis: `AI scan error: ${String(error)}`,
        flags: [{ severity: "medium", finding: `Scan error: ${String(error)}`, recommendation: "Manual review required" }],
        error: String(error),
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  }
});
