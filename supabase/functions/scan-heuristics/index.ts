import { serve } from "https://deno.land/std@0.177.0/http/server.ts";
import type { ScanLayerResult, HeuristicRule } from "../_shared/types.ts";

interface HeuristicInput {
  extension_id: string;
  categories: string[];
  permissions: Array<{ capability: string; scope?: unknown; reason?: string }>;
  tools: Array<{ name: string; description: string }>;
  manifest: Record<string, unknown>;
  wasm_size_bytes: number;
}

// Heuristic rules
const rules: Array<{
  id: string;
  name: string;
  description: string;
  severity: "low" | "medium" | "high";
  check: (input: HeuristicInput) => { triggered: boolean; detail: string };
}> = [
  {
    id: "H-001",
    name: "Excessive permissions",
    description: "Extension requests more than 5 permissions",
    severity: "medium",
    check: (input) => ({
      triggered: input.permissions.length > 5,
      detail: `Requests ${input.permissions.length} permissions`,
    }),
  },
  {
    id: "H-002",
    name: "Filesystem write without justification",
    description: "Extension requests filesystem.write without a clear reason",
    severity: "high",
    check: (input) => {
      const fsPerm = input.permissions.find((p) => p.capability === "filesystem.write");
      return {
        triggered: !!fsPerm && !fsPerm.reason,
        detail: "Requests filesystem.write without providing a reason",
      };
    },
  },
  {
    id: "H-003",
    name: "Network + filesystem combination",
    description: "Extension requests both network and filesystem access (data exfiltration risk)",
    severity: "high",
    check: (input) => {
      const hasNetwork = input.permissions.some((p) => p.capability.startsWith("network."));
      const hasFs = input.permissions.some((p) => p.capability.startsWith("filesystem."));
      return {
        triggered: hasNetwork && hasFs,
        detail: "Requests both network and filesystem access",
      };
    },
  },
  {
    id: "H-004",
    name: "Category-permission mismatch",
    description: "Extension category doesn't match permission profile",
    severity: "medium",
    check: (input) => {
      const benignCategories = ["weather", "utilities", "education"];
      const isBenign = input.categories.some((c) => benignCategories.includes(c));
      const hasSensitive = input.permissions.some((p) =>
        ["filesystem.write", "browser.scrape", "channel.send"].includes(p.capability)
      );
      return {
        triggered: isBenign && hasSensitive,
        detail: `Benign category (${input.categories.join(", ")}) requests sensitive permissions`,
      };
    },
  },
  {
    id: "H-005",
    name: "Missing manifest fields",
    description: "Extension manifest is missing important metadata",
    severity: "low",
    check: (input) => {
      const ext = input.manifest.extension as Record<string, unknown> | undefined;
      const missing = [];
      if (!ext?.author) missing.push("author");
      if (!ext?.description) missing.push("description");
      if (!ext?.repository) missing.push("repository");
      if (!ext?.license) missing.push("license");
      return {
        triggered: missing.length >= 2,
        detail: `Missing fields: ${missing.join(", ")}`,
      };
    },
  },
  {
    id: "H-006",
    name: "Oversized WASM binary",
    description: "WASM binary is unusually large (> 10MB)",
    severity: "medium",
    check: (input) => ({
      triggered: input.wasm_size_bytes > 10 * 1024 * 1024,
      detail: `WASM binary is ${(input.wasm_size_bytes / 1024 / 1024).toFixed(1)}MB`,
    }),
  },
  {
    id: "H-007",
    name: "No tools defined",
    description: "Extension declares no tools (may be a placeholder or stub)",
    severity: "low",
    check: (input) => ({
      triggered: input.tools.length === 0,
      detail: "Extension defines no tools",
    }),
  },
  {
    id: "H-008",
    name: "Broad network scope",
    description: "Extension requests network access without domain restrictions",
    severity: "high",
    check: (input) => {
      const netPerm = input.permissions.find((p) => p.capability === "network.http");
      const scope = netPerm?.scope as Record<string, unknown> | undefined;
      return {
        triggered: !!netPerm && (!scope || !scope.domains),
        detail: "Requests unrestricted network access (no domain scope)",
      };
    },
  },
];

serve(async (req) => {
  try {
    const input: HeuristicInput = await req.json();

    const triggeredRules: HeuristicRule[] = [];

    for (const rule of rules) {
      const result = rule.check(input);
      if (result.triggered) {
        triggeredRules.push({
          rule: rule.name,
          score: rule.severity === "high" ? 30 : rule.severity === "medium" ? 15 : 5,
          description: result.detail,
          severity: rule.severity,
        });
      }
    }

    const totalPenalty = triggeredRules.reduce((sum, r) => sum + r.score, 0);
    const score = Math.max(0, 100 - totalPenalty);

    const result: ScanLayerResult = {
      score,
      details: triggeredRules,
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
