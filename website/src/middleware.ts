import { NextResponse, type NextRequest } from "next/server";
import { updateSession } from "@/lib/supabase/middleware";

// ── In-Memory IP Blocklist Cache ───────────────────────────
// Refreshed every 5 minutes. Populated by an internal API call
// to avoid importing the full Supabase client in Edge middleware.

let blockedIPSet = new Set<string>();
let lastBlocklistRefresh = 0;
const BLOCKLIST_TTL_MS = 5 * 60 * 1000;

async function refreshBlocklist(origin: string): Promise<void> {
  try {
    // Fetch blocked IPs from internal API route (service-level)
    const res = await fetch(`${origin}/api/v1/internal/blocked-ips`, {
      headers: { "x-internal-key": process.env.INTERNAL_API_KEY || "" },
    });
    if (res.ok) {
      const data = await res.json();
      if (Array.isArray(data.ips)) {
        blockedIPSet = new Set(data.ips as string[]);
      }
    }
    lastBlocklistRefresh = Date.now();
  } catch {
    // Keep existing cache on error
    lastBlocklistRefresh = Date.now(); // Still update to avoid repeated failures
  }
}

// ── Simple IP-based global rate limit (in-memory) ──────────
// This is a lightweight first line of defense. The real rate
// limit with DB persistence happens in withProtection().
const ipHitCounts = new Map<string, { count: number; resetAt: number }>();
const GLOBAL_WINDOW_MS = 60_000; // 1 minute
const GLOBAL_MAX = 100; // 100 requests per minute per IP

function checkGlobalRateLimit(ipHash: string): { allowed: boolean; remaining: number } {
  const now = Date.now();
  const entry = ipHitCounts.get(ipHash);

  if (!entry || now >= entry.resetAt) {
    ipHitCounts.set(ipHash, { count: 1, resetAt: now + GLOBAL_WINDOW_MS });
    return { allowed: true, remaining: GLOBAL_MAX - 1 };
  }

  entry.count++;
  if (entry.count > GLOBAL_MAX) {
    return { allowed: false, remaining: 0 };
  }

  return { allowed: true, remaining: GLOBAL_MAX - entry.count };
}

// Periodic cleanup of stale entries (every 5 min)
let lastCleanup = 0;
function cleanupHitCounts() {
  const now = Date.now();
  if (now - lastCleanup < 300_000) return;
  lastCleanup = now;
  for (const [key, entry] of ipHitCounts) {
    if (now >= entry.resetAt) {
      ipHitCounts.delete(key);
    }
  }
}

// ── SHA-256 IP hash (matches ip-utils.ts hashIP) ────────────
// Edge runtime supports crypto.subtle (Web Crypto API)
async function hashIP(ip: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(ip);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  return hashHex.slice(0, 16);
}

export async function middleware(request: NextRequest) {
  const forwarded = request.headers.get("x-forwarded-for");
  const ip = forwarded?.split(",")[0]?.trim() || "unknown";
  const ipHash = await hashIP(ip);

  // ── 1. Refresh blocklist if stale ───────────────────────
  if (Date.now() - lastBlocklistRefresh > BLOCKLIST_TTL_MS) {
    const origin = request.nextUrl.origin;
    // Don't await -- fire in background to avoid blocking the request
    refreshBlocklist(origin).catch(() => {});
  }

  // ── 2. IP Blocklist Check ──────────────────────────────
  if (blockedIPSet.has(ipHash)) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  // ── 3. Global Rate Limit (API routes only) ────────────
  if (request.nextUrl.pathname.startsWith("/api/")) {
    cleanupHitCounts();
    const rl = checkGlobalRateLimit(ipHash);

    if (!rl.allowed) {
      return NextResponse.json(
        { error: "Too many requests" },
        {
          status: 429,
          headers: {
            "Retry-After": "60",
            "X-RateLimit-Limit": GLOBAL_MAX.toString(),
            "X-RateLimit-Remaining": "0",
          },
        }
      );
    }
  }

  // ── 4. Continue to Supabase session refresh ───────────
  return await updateSession(request);
}

export const config = {
  matcher: [
    "/((?!_next/static|_next/image|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)",
  ],
};
