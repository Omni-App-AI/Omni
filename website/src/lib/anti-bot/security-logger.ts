import { createServiceClient } from "@/lib/supabase/server";
import { hashIP } from "./ip-utils";

// ── Security Event Types ───────────────────────────────────

export type SecurityEventType =
  | "rate_limited"
  | "turnstile_fail"
  | "honeypot_caught"
  | "spam_blocked"
  | "spam_flagged"
  | "account_banned"
  | "account_unbanned"
  | "login_failed"
  | "ip_blocked"
  | "ip_unblocked"
  | "content_flagged"
  | "content_removed"
  | "extension_moderated"
  | "trust_gate_blocked";

interface LogEventOptions {
  eventType: SecurityEventType;
  actorId?: string;
  ip?: string;
  userAgent?: string;
  metadata?: Record<string, unknown>;
}

// ── Logging Function ───────────────────────────────────────

export async function logSecurityEvent(options: LogEventOptions): Promise<void> {
  try {
    const supabase = createServiceClient();
    const ipHash = options.ip ? await hashIP(options.ip) : null;

    await (supabase.from("security_events") as any).insert({
      event_type: options.eventType,
      actor_id: options.actorId || null,
      ip_address: options.ip || null,
      ip_hash: ipHash,
      user_agent: options.userAgent || null,
      metadata: options.metadata || {},
    });
  } catch (error) {
    // Fire-and-forget -- don't let logging failures block requests
    console.error("[security-logger] Failed to log event:", error);
  }
}

// ── Auto-Flag Content ──────────────────────────────────────

export async function autoFlagContent(options: {
  contentType: "post" | "reply" | "review" | "extension";
  contentId: string;
  reason: "auto_spam" | "auto_suspicious";
  spamScore: number;
  details: string;
}): Promise<void> {
  try {
    const supabase = createServiceClient();
    await (supabase.from("content_flags") as any).insert({
      content_type: options.contentType,
      content_id: options.contentId,
      reporter_id: null, // System-generated
      reason: options.reason,
      spam_score: options.spamScore,
      details: options.details,
      status: "pending",
    });
  } catch (error) {
    console.error("[security-logger] Failed to auto-flag content:", error);
  }
}
