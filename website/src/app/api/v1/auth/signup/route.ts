import { NextResponse, type NextRequest } from "next/server";
import { createServiceClient } from "@/lib/supabase/server";
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
  const config = RATE_LIMITS.signup_attempt!;
  const result = await checkRateLimit(
    `ip:${ipHash}`,
    "signup_attempt",
    config.max as number,
    config.window
  );

  if (!result.allowed) {
    await incrementIPCounter(ipHash, "total_rate_limits");
    await logSecurityEvent({
      eventType: "rate_limited",
      ip,
      userAgent,
      metadata: { action: "signup_attempt" },
    });

    return NextResponse.json(
      { error: "Too many signup attempts. Please try again later." },
      { status: 429, headers: rateLimitHeaders(result) }
    );
  }

  await recordRateLimitHit(`ip:${ipHash}`, "signup_attempt");

  // ── 2. Parse Body ──────────────────────────────────────
  let body: Record<string, unknown>;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid request body" }, { status: 400 });
  }

  const { email, password, username, full_name, turnstile_token, hp_website, hp_timestamp, hp_token } = body as {
    email?: string;
    password?: string;
    username?: string;
    full_name?: string;
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
      metadata: { signals: honeypotResult.signals, action: "signup" },
    });

    return NextResponse.json(
      { error: "Request could not be processed" },
      { status: 400 }
    );
  }

  // ── 4. Turnstile Verification (managed mode) ──────────
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
      metadata: { error_codes: turnstileResult.error_codes, action: "signup" },
    });

    return NextResponse.json(
      { error: "Verification failed. Please try again." },
      { status: 422 }
    );
  }

  // ── 5. Input Validation ────────────────────────────────
  if (!email || !password || !username) {
    return NextResponse.json(
      { error: "Email, password, and username are required." },
      { status: 400 }
    );
  }

  if (!/^[a-z0-9_-]{3,39}$/.test(username)) {
    return NextResponse.json(
      { error: "Username must be 3-39 characters: lowercase letters, numbers, hyphens, underscores." },
      { status: 400 }
    );
  }

  if (password.length < 8) {
    return NextResponse.json(
      { error: "Password must be at least 8 characters." },
      { status: 400 }
    );
  }

  // ── 6. Create Account via Supabase Admin API ──────────
  const supabase = createServiceClient();

  // Check username uniqueness
  const { data: existingUser } = await supabase
    .from("profiles")
    .select("id")
    .eq("username", username)
    .single();

  if (existingUser) {
    return NextResponse.json(
      { error: "Username is already taken." },
      { status: 409 }
    );
  }

  const { data, error } = await supabase.auth.admin.createUser({
    email,
    password,
    email_confirm: false, // User must confirm via email
    user_metadata: {
      username,
      full_name: full_name || "",
    },
  });

  if (error) {
    return NextResponse.json({ error: error.message }, { status: 400 });
  }

  return NextResponse.json(
    { message: "Account created. Please check your email to confirm." },
    { status: 201 }
  );
}
