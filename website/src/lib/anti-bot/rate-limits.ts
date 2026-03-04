import { createServiceClient } from "@/lib/supabase/server";

// ── Types ──────────────────────────────────────────────────

export type TrustTier = "newcomer" | "member" | "contributor" | "trusted" | "expert";

interface GraduatedMax {
  newcomer: number;
  member: number;
  contributor: number;
  trusted: number;
  expert: number;
}

export interface RateLimitConfig {
  window: number; // seconds
  max: number | GraduatedMax;
  keyType: "ip" | "user";
}

export interface RateLimitResult {
  allowed: boolean;
  limit: number;
  remaining: number;
  resetAt: Date;
  retryAfter?: number; // seconds
}

// ── Rate Limit Definitions ─────────────────────────────────

export const RATE_LIMITS: Record<string, RateLimitConfig> = {
  // Global (middleware-level, IP-based)
  global: { window: 60, max: 100, keyType: "ip" },

  // Auth
  login_attempt: { window: 900, max: 10, keyType: "ip" },
  signup_attempt: { window: 3600, max: 5, keyType: "ip" },
  password_reset: { window: 3600, max: 3, keyType: "ip" },

  // Content creation (per user, graduated by trust)
  post_create: {
    window: 3600,
    max: { newcomer: 2, member: 5, contributor: 15, trusted: 30, expert: 60 },
    keyType: "user",
  },
  reply_create: {
    window: 3600,
    max: { newcomer: 5, member: 15, contributor: 30, trusted: 60, expert: 120 },
    keyType: "user",
  },
  review_create: {
    window: 86400,
    max: { newcomer: 3, member: 10, contributor: 20, trusted: 30, expert: 50 },
    keyType: "user",
  },

  // Engagement
  vote_cast: {
    window: 3600,
    max: { newcomer: 10, member: 30, contributor: 60, trusted: 100, expert: 200 },
    keyType: "user",
  },
  follow_action: { window: 3600, max: 30, keyType: "user" },

  // Publishing
  extension_publish: { window: 3600, max: 5, keyType: "user" },
  api_key_create: { window: 86400, max: 5, keyType: "user" },

  // Downloads
  download: { window: 3600, max: 60, keyType: "ip" },
  app_download: { window: 3600, max: 60, keyType: "ip" },

  // Release publishing (CI only)
  release_publish: { window: 3600, max: 10, keyType: "ip" },

  // Flagging
  flag_create: { window: 3600, max: 10, keyType: "user" },
};

// ── Core Functions ─────────────────────────────────────────

export function getRateLimit(action: string, trustTier?: TrustTier): number {
  const config = RATE_LIMITS[action];
  if (!config) return 100; // default

  if (typeof config.max === "number") {
    return config.max;
  }

  const tier = trustTier || "newcomer";
  return config.max[tier] ?? config.max.newcomer;
}

export async function checkRateLimit(
  key: string,
  action: string,
  limit: number,
  windowSecs: number
): Promise<RateLimitResult> {
  const supabase = createServiceClient();
  const windowStart = new Date(Date.now() - windowSecs * 1000).toISOString();

  const { count } = await (supabase
    .from("rate_limits") as any)
    .select("*", { count: "exact", head: true })
    .eq("key", key)
    .eq("action", action)
    .gte("created_at", windowStart);

  const currentCount = count ?? 0;
  const allowed = currentCount < limit;
  const remaining = Math.max(0, limit - currentCount);
  const resetAt = new Date(Date.now() + windowSecs * 1000);

  return {
    allowed,
    limit,
    remaining,
    resetAt,
    retryAfter: allowed ? undefined : windowSecs,
  };
}

export async function recordRateLimitHit(
  key: string,
  action: string
): Promise<void> {
  const supabase = createServiceClient();
  await (supabase.from("rate_limits") as any).insert({
    key,
    action,
    created_at: new Date().toISOString(),
  });
}

export function rateLimitHeaders(result: RateLimitResult): Record<string, string> {
  const headers: Record<string, string> = {
    "X-RateLimit-Limit": result.limit.toString(),
    "X-RateLimit-Remaining": result.remaining.toString(),
    "X-RateLimit-Reset": Math.floor(result.resetAt.getTime() / 1000).toString(),
  };

  if (result.retryAfter !== undefined) {
    headers["Retry-After"] = result.retryAfter.toString();
  }

  return headers;
}
