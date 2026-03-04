import { NextResponse, type NextRequest } from "next/server";
import { createClient } from "@/lib/supabase/server";
import { verifyTurnstile } from "@/lib/anti-bot/turnstile";
import { validateHoneypot } from "@/lib/anti-bot/honeypot";
import { extractIP, hashIP, incrementIPCounter } from "@/lib/anti-bot/ip-utils";
import {
  RATE_LIMITS,
  checkRateLimit,
  recordRateLimitHit,
  rateLimitHeaders,
} from "@/lib/anti-bot/rate-limits";
import { logSecurityEvent } from "@/lib/anti-bot/security-logger";

export async function POST(request: NextRequest) {
  const ip = extractIP(request);
  const ipHash = await hashIP(ip);
  const userAgent = request.headers.get("user-agent") || "";

  // ── 1. Rate Limit (IP-based) ───────────────────────────
  const config = RATE_LIMITS.login_attempt!;
  const result = await checkRateLimit(
    `ip:${ipHash}`,
    "login_attempt",
    config.max as number,
    config.window
  );

  if (!result.allowed) {
    await incrementIPCounter(ipHash, "total_rate_limits");
    await logSecurityEvent({
      eventType: "rate_limited",
      ip,
      userAgent,
      metadata: { action: "login_attempt" },
    });

    return NextResponse.json(
      { error: "Too many login attempts. Please try again later." },
      { status: 429, headers: rateLimitHeaders(result) }
    );
  }

  await recordRateLimitHit(`ip:${ipHash}`, "login_attempt");

  // ── 2. Parse Body ──────────────────────────────────────
  let body: Record<string, unknown>;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid request body" }, { status: 400 });
  }

  const { email, password, turnstile_token, hp_website, hp_timestamp, hp_token } = body as {
    email?: string;
    password?: string;
    turnstile_token?: string;
    hp_website?: string;
    hp_timestamp?: string;
    hp_token?: string;
  };

  // ── 3. Honeypot Check ──────────────────────────────────
  const honeypotResult = validateHoneypot({ hp_website, hp_timestamp, hp_token });
  if (!honeypotResult.passed) {
    await incrementIPCounter(ipHash, "total_honeypots");
    await logSecurityEvent({
      eventType: "honeypot_caught",
      ip,
      userAgent,
      metadata: { signals: honeypotResult.signals, action: "login" },
    });

    return NextResponse.json(
      { error: "Request could not be processed" },
      { status: 400 }
    );
  }

  // ── 4. Turnstile Verification (invisible mode) ────────
  if (!turnstile_token) {
    return NextResponse.json(
      { error: "Please complete the verification challenge." },
      { status: 422 }
    );
  }

  const turnstileResult = await verifyTurnstile(turnstile_token, ip);
  if (!turnstileResult.success) {
    await incrementIPCounter(ipHash, "total_turnstile_fails");
    await logSecurityEvent({
      eventType: "turnstile_fail",
      ip,
      userAgent,
      metadata: { error_codes: turnstileResult.error_codes, action: "login" },
    });

    return NextResponse.json(
      { error: "Verification failed. Please try again." },
      { status: 422 }
    );
  }

  // ── 5. Input Validation ────────────────────────────────
  if (!email || !password) {
    return NextResponse.json(
      { error: "Email and password are required." },
      { status: 400 }
    );
  }

  // ── 6. Authenticate via Supabase ──────────────────────
  const supabase = await createClient();
  const { error } = await supabase.auth.signInWithPassword({ email, password });

  if (error) {
    await logSecurityEvent({
      eventType: "login_failed",
      ip,
      userAgent,
      metadata: { email },
    });

    return NextResponse.json({ error: error.message }, { status: 401 });
  }

  return NextResponse.json({ message: "Login successful" }, { status: 200 });
}
