import { NextResponse, type NextRequest } from "next/server";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { verifyTurnstile, isTurnstileRequired } from "./turnstile";
import {
  RATE_LIMITS,
  getRateLimit,
  checkRateLimit,
  recordRateLimitHit,
  rateLimitHeaders,
  type TrustTier,
} from "./rate-limits";
import { validateHoneypot, type HoneypotPayload } from "./honeypot";
import { analyzeContent, analyzeAuthorBehavior, type AuthorContext, type SpamAnalysis } from "./spam-detector";
import { getTrustTier, getCapabilities, checkCapability, checkLinkLimit, type TrustCapabilities } from "./trust";
import { extractIP, hashIP, incrementIPCounter } from "./ip-utils";
import { logSecurityEvent, autoFlagContent } from "./security-logger";

// ── Types ──────────────────────────────────────────────────

export interface ProtectionConfig {
  /** Turnstile challenge mode. "trust-gated" = required for newcomers only */
  turnstile?: "managed" | "invisible" | "trust-gated";
  /** Key into RATE_LIMITS config */
  rateLimit: string;
  /** Check honeypot fields in request body */
  honeypot?: boolean;
  /** Run content body through spam detector */
  spamCheck?: boolean;
  /** Content type for spam/link checking */
  contentType?: "post" | "reply" | "review";
  /** Required capability name from TrustCapabilities (e.g. "can_post") */
  trustGate?: keyof TrustCapabilities;
  /** Require authenticated user (default: true) */
  requireAuth?: boolean;
}

export interface ProtectedContext {
  user: { id: string; email?: string };
  profile: { reputation: number; is_moderator: boolean; is_banned: boolean };
  trustTier: TrustTier;
  capabilities: TrustCapabilities;
  body: Record<string, unknown>;
  ip: string;
  ipHash: string;
  spamAnalysis?: SpamAnalysis;
  /** True if user is shadow-banned -- content should be saved with shadow_hidden=true */
  shadowBanned: boolean;
}

export type ProtectedHandler = (
  request: NextRequest,
  context: ProtectedContext,
  routeContext?: { params: Promise<Record<string, string>> }
) => Promise<NextResponse>;

// ── Main Wrapper ───────────────────────────────────────────

export function withProtection(config: ProtectionConfig, handler: ProtectedHandler) {
  return async function protectedRoute(
    request: NextRequest,
    routeContext: { params: Promise<Record<string, string>> }
  ): Promise<NextResponse> {
    const ip = extractIP(request);
    const ipHash = await hashIP(ip);
    const userAgent = request.headers.get("user-agent") || "";
    const requireAuth = config.requireAuth !== false;

    // ── 1. Authentication Check ────────────────────────────
    const supabase = await createClient();
    const { data: { user } } = await supabase.auth.getUser();

    if (requireAuth && !user) {
      return NextResponse.json(
        { error: "Authentication required" },
        { status: 401 }
      );
    }

    // ── 2. Ban Check ───────────────────────────────────────
    let profile = { reputation: 0, is_moderator: false, is_banned: false };
    let shadowBanned = false;

    if (user) {
      const serviceClient = createServiceClient();
      const { data: profileData } = await serviceClient
        .from("profiles")
        .select("reputation, is_moderator, is_banned")
        .eq("id", user.id)
        .single();

      if (profileData) {
        profile = profileData;
      }

      if (profile.is_banned) {
        // Check for active ban
        const { data: activeBanData } = await (serviceClient
          .from("user_bans") as any)
          .select("ban_type, reason, expires_at")
          .eq("user_id", user.id)
          .is("revoked_at", null)
          .order("created_at", { ascending: false })
          .limit(1)
          .single();
        const activeBan = activeBanData as any;

        if (activeBan) {
          // Check if temporary ban has expired
          if (activeBan.ban_type === "temporary" && activeBan.expires_at) {
            if (new Date(activeBan.expires_at) < new Date()) {
              // Ban expired -- clear profile flag and revoke ban record
              await (serviceClient
                .from("profiles") as any)
                .update({ is_banned: false, ban_reason: null })
                .eq("id", user.id);
              await (serviceClient
                .from("user_bans") as any)
                .update({ revoked_at: new Date().toISOString() })
                .eq("user_id", user.id)
                .is("revoked_at", null)
                .eq("ban_type", "temporary");
            } else {
              return NextResponse.json(
                { error: "Account suspended", reason: activeBan.reason, expires_at: activeBan.expires_at },
                { status: 403 }
              );
            }
          } else if (activeBan.ban_type === "permanent") {
            return NextResponse.json(
              { error: "Account permanently banned", reason: activeBan.reason },
              { status: 403 }
            );
          }
          // Shadow bans fall through -- content will be marked shadow_hidden
          if (activeBan.ban_type === "shadow") {
            shadowBanned = true;
          }
        }
      }
    }

    // ── 3. Trust Tier Resolution ───────────────────────────
    const trustTier = getTrustTier(profile.reputation);
    const capabilities = getCapabilities(profile.reputation);

    // ── 3b. Trust Gate Check ────────────────────────────────
    if (config.trustGate) {
      const capCheck = checkCapability(profile.reputation, config.trustGate);
      if (!capCheck.allowed) {
        await logSecurityEvent({
          eventType: "trust_gate_blocked",
          actorId: user?.id,
          ip,
          userAgent,
          metadata: { gate: config.trustGate, trustTier, reason: capCheck.reason },
        });

        return NextResponse.json(
          { error: capCheck.reason || `Insufficient reputation for this action`, code: "trust_gate" },
          { status: 403 }
        );
      }
    }

    // ── 4. Turnstile Verification ──────────────────────────
    if (isTurnstileRequired(config.turnstile, trustTier)) {
      let turnstileToken: string | undefined;

      // Try to get token from header first, then body
      turnstileToken = request.headers.get("x-turnstile-token") || undefined;

      if (!turnstileToken) {
        // We'll parse body later if needed, but try the header first
        try {
          const clonedReq = request.clone();
          const bodyData = await clonedReq.json();
          turnstileToken = bodyData?.turnstile_token;
        } catch {
          // Body might not be JSON
        }
      }

      if (!turnstileToken) {
        return NextResponse.json(
          { error: "Turnstile verification required", code: "turnstile_required" },
          { status: 422 }
        );
      }

      const result = await verifyTurnstile(turnstileToken, ip);
      if (!result.success) {
        await incrementIPCounter(ipHash, "total_turnstile_fails");
        await logSecurityEvent({
          eventType: "turnstile_fail",
          actorId: user?.id,
          ip,
          userAgent,
          metadata: { error_codes: result.error_codes },
        });

        return NextResponse.json(
          { error: "Turnstile verification failed", codes: result.error_codes },
          { status: 422 }
        );
      }
    }

    // ── 5. Parse Request Body ──────────────────────────────
    let body: Record<string, unknown> = {};
    try {
      const clonedReq = request.clone();
      body = await clonedReq.json();
    } catch {
      // Body might be empty for some requests, which is fine
    }

    // ── 6. Honeypot Validation ─────────────────────────────
    if (config.honeypot) {
      const honeypotFields: HoneypotPayload = {
        hp_website: body.hp_website as string | undefined,
        hp_timestamp: body.hp_timestamp as string | undefined,
        hp_token: body.hp_token as string | undefined,
      };

      const honeypotResult = validateHoneypot(honeypotFields);
      if (!honeypotResult.passed) {
        await incrementIPCounter(ipHash, "total_honeypots");
        await logSecurityEvent({
          eventType: "honeypot_caught",
          actorId: user?.id,
          ip,
          userAgent,
          metadata: { signals: honeypotResult.signals },
        });

        // Return a generic error -- don't reveal we caught a bot
        return NextResponse.json(
          { error: "Request could not be processed" },
          { status: 400 }
        );
      }

      // Strip honeypot fields from body before passing to handler
      delete body.hp_website;
      delete body.hp_timestamp;
      delete body.hp_token;
      delete body.turnstile_token;
    }

    // ── 7. Endpoint Rate Limit ─────────────────────────────
    const rateLimitConfig = RATE_LIMITS[config.rateLimit];
    if (rateLimitConfig) {
      const key = rateLimitConfig.keyType === "ip"
        ? `ip:${ipHash}`
        : `user:${user?.id || ipHash}`;

      const limit = getRateLimit(config.rateLimit, trustTier);
      const result = await checkRateLimit(
        key,
        config.rateLimit,
        limit,
        rateLimitConfig.window
      );

      if (!result.allowed) {
        await incrementIPCounter(ipHash, "total_rate_limits");
        await logSecurityEvent({
          eventType: "rate_limited",
          actorId: user?.id,
          ip,
          userAgent,
          metadata: { action: config.rateLimit, limit, trustTier },
        });

        return NextResponse.json(
          { error: "Rate limit exceeded. Please try again later." },
          { status: 429, headers: rateLimitHeaders(result) }
        );
      }

      // Record this hit for the sliding window
      await recordRateLimitHit(key, config.rateLimit);
    }

    // ── 8. Content Spam Detection ──────────────────────────
    let spamAnalysis: SpamAnalysis | undefined;

    if (config.spamCheck && user) {
      const textContent = [
        body.title as string || "",
        body.body as string || "",
        body.content as string || "",
      ].filter(Boolean).join("\n\n");

      if (textContent.length > 0) {
        const authorContext: AuthorContext = {
          userId: user.id,
          reputation: profile.reputation,
          accountCreatedAt: user.created_at || new Date().toISOString(),
          postCount: 0, // Could be fetched from profile if needed
        };

        spamAnalysis = analyzeContent(textContent, authorContext, {
          title: body.title as string,
          contentType: (config.contentType === "review" ? "post" : config.contentType) || "post",
        });

        // Run async behavior checks (copy-paste flood, burst posting)
        const effectiveContentType = config.contentType === "review" ? "post" : (config.contentType || "post");
        if (effectiveContentType === "post" || effectiveContentType === "reply") {
          const behaviorSignals = await analyzeAuthorBehavior(textContent, user.id, effectiveContentType);
          if (behaviorSignals.length > 0) {
            spamAnalysis.signals.push(...behaviorSignals);
            spamAnalysis.score = Math.min(100, spamAnalysis.signals.reduce((sum, s) => sum + s.weight, 0));
            if (spamAnalysis.score > 60) spamAnalysis.verdict = "spam";
            else if (spamAnalysis.score > 30) spamAnalysis.verdict = "suspicious";
          }
        }

        if (spamAnalysis.verdict === "spam") {
          await logSecurityEvent({
            eventType: "spam_blocked",
            actorId: user.id,
            ip,
            userAgent,
            metadata: {
              score: spamAnalysis.score,
              signals: spamAnalysis.signals.map((s) => s.name),
            },
          });

          return NextResponse.json(
            { error: "Your content was flagged as spam. Please review and try again." },
            { status: 422 }
          );
        }

        // Check link limits based on trust tier
        const contentType = config.contentType || "post";
        if (contentType === "post" || contentType === "reply") {
          const linkCheck = checkLinkLimit(textContent, profile.reputation, contentType);
          if (!linkCheck.allowed) {
            await logSecurityEvent({
              eventType: "trust_gate_blocked",
              actorId: user.id,
              ip,
              userAgent,
              metadata: {
                reason: "link_limit",
                linkCount: linkCheck.linkCount,
                limit: linkCheck.limit,
                trustTier,
              },
            });

            return NextResponse.json(
              {
                error: `Your trust level allows up to ${linkCheck.limit} links per ${contentType}. You included ${linkCheck.linkCount}.`,
                code: "link_limit_exceeded",
              },
              { status: 422 }
            );
          }
        }
      }
    }

    // ── 9. Call Actual Handler ──────────────────────────────
    const ctx: ProtectedContext = {
      user: user ? { id: user.id, email: user.email } : { id: "anonymous" },
      profile,
      trustTier,
      capabilities,
      body,
      ip,
      ipHash,
      spamAnalysis,
      shadowBanned,
    };

    const response = await handler(request, ctx, routeContext);

    // ── 10. Post-Response: Auto-flag suspicious content ────
    if (spamAnalysis && spamAnalysis.verdict === "suspicious") {
      // The response is already sent; flag in the background
      // We need the content ID from the response if possible
      try {
        const responseData = await response.clone().json();
        const contentId = responseData?.id || responseData?.post_id || responseData?.reply_id;
        if (contentId) {
          autoFlagContent({
            contentType: config.contentType || "post",
            contentId,
            reason: "auto_suspicious",
            spamScore: spamAnalysis.score,
            details: spamAnalysis.signals.map((s) => `${s.name}: ${s.detail || ""}`).join("; "),
          });
        }
      } catch {
        // Response might not be JSON; skip auto-flagging
      }
    }

    return response;
  };
}
