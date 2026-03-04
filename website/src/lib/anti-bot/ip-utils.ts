import { createServiceClient } from "@/lib/supabase/server";

// ── IP Extraction ──────────────────────────────────────────

export function extractIP(request: Request): string {
  const headers = new Headers(request.headers);
  const forwarded = headers.get("x-forwarded-for");
  if (forwarded) {
    return forwarded.split(",")[0]!.trim();
  }
  const real = headers.get("x-real-ip");
  if (real) return real.trim();
  return "unknown";
}

// ── IP Hashing ─────────────────────────────────────────────

export async function hashIP(ip: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(ip);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  return hashHex.slice(0, 16);
}

// ── IP Blocklist Cache ─────────────────────────────────────

let blockedIPs = new Set<string>();
let lastRefresh = 0;
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes

async function refreshBlocklist(): Promise<void> {
  try {
    const supabase = createServiceClient();
    const { data } = await (supabase
      .from("ip_reputation") as any)
      .select("ip_hash")
      .eq("blocked", true);

    if (data) {
      blockedIPs = new Set(data.map((row: any) => row.ip_hash));
    }
    lastRefresh = Date.now();
  } catch {
    // Keep existing cache on error
  }
}

export async function isBlockedIP(ipHash: string): Promise<boolean> {
  if (Date.now() - lastRefresh > CACHE_TTL_MS) {
    await refreshBlocklist();
  }
  return blockedIPs.has(ipHash);
}

// ── IP Reputation Tracking ─────────────────────────────────

export async function incrementIPCounter(
  ipHash: string,
  field: "total_flags" | "total_rate_limits" | "total_honeypots" | "total_turnstile_fails"
): Promise<void> {
  const supabase = createServiceClient();

  // Upsert: create if not exists, increment counter
  const { data: existing } = await (supabase
    .from("ip_reputation") as any)
    .select("ip_hash, risk_score, total_flags, total_rate_limits, total_honeypots, total_turnstile_fails")
    .eq("ip_hash", ipHash)
    .single();

  if (existing) {
    const row = existing as any;
    const newCount = ((row as Record<string, number>)[field] ?? 0) + 1;
    const riskScore = calculateRiskScore({
      ...row,
      [field]: newCount,
    });

    await (supabase
      .from("ip_reputation") as any)
      .update({
        [field]: newCount,
        risk_score: riskScore,
        blocked: riskScore >= 80,
        last_seen_at: new Date().toISOString(),
      })
      .eq("ip_hash", ipHash);
  } else {
    const initial = {
      ip_hash: ipHash,
      [field]: 1,
      risk_score: field === "total_honeypots" ? 30 : 10,
      last_seen_at: new Date().toISOString(),
    };
    await (supabase.from("ip_reputation") as any).insert(initial);
  }
}

function calculateRiskScore(counters: {
  total_flags: number;
  total_rate_limits: number;
  total_honeypots: number;
  total_turnstile_fails: number;
}): number {
  const score =
    counters.total_flags * 15 +
    counters.total_rate_limits * 5 +
    counters.total_honeypots * 30 +
    counters.total_turnstile_fails * 20;
  return Math.min(100, score);
}
