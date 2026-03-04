import type { TrustTier } from "./rate-limits";

// ── Trust Capabilities ─────────────────────────────────────

export interface TrustCapabilities {
  can_post: boolean;
  can_reply: boolean;
  can_vote: boolean;
  can_review: boolean;
  max_links_per_post: number;  // -1 = unlimited
  max_links_per_reply: number; // -1 = unlimited
  turnstile_required: boolean;
  post_cooldown_mins: number;
  max_posts_per_day: number;   // -1 = unlimited
  max_replies_per_day: number; // -1 = unlimited
  can_edit_own: boolean;
  can_delete_own: boolean;
  can_moderate: boolean;
}

const CAPABILITIES: Record<TrustTier, TrustCapabilities> = {
  newcomer: {
    can_post: true,
    can_reply: true,
    can_vote: true,
    can_review: true,
    max_links_per_post: 0,
    max_links_per_reply: 0,
    turnstile_required: true,
    post_cooldown_mins: 30,
    max_posts_per_day: 3,
    max_replies_per_day: 10,
    can_edit_own: true,
    can_delete_own: false,
    can_moderate: false,
  },
  member: {
    can_post: true,
    can_reply: true,
    can_vote: true,
    can_review: true,
    max_links_per_post: 2,
    max_links_per_reply: 1,
    turnstile_required: false,
    post_cooldown_mins: 5,
    max_posts_per_day: 10,
    max_replies_per_day: 30,
    can_edit_own: true,
    can_delete_own: true,
    can_moderate: false,
  },
  contributor: {
    can_post: true,
    can_reply: true,
    can_vote: true,
    can_review: true,
    max_links_per_post: 5,
    max_links_per_reply: 3,
    turnstile_required: false,
    post_cooldown_mins: 1,
    max_posts_per_day: 30,
    max_replies_per_day: 100,
    can_edit_own: true,
    can_delete_own: true,
    can_moderate: false,
  },
  trusted: {
    can_post: true,
    can_reply: true,
    can_vote: true,
    can_review: true,
    max_links_per_post: 10,
    max_links_per_reply: 5,
    turnstile_required: false,
    post_cooldown_mins: 0,
    max_posts_per_day: 100,
    max_replies_per_day: 500,
    can_edit_own: true,
    can_delete_own: true,
    can_moderate: true,
  },
  expert: {
    can_post: true,
    can_reply: true,
    can_vote: true,
    can_review: true,
    max_links_per_post: -1,
    max_links_per_reply: -1,
    turnstile_required: false,
    post_cooldown_mins: 0,
    max_posts_per_day: -1,
    max_replies_per_day: -1,
    can_edit_own: true,
    can_delete_own: true,
    can_moderate: true,
  },
};

// ── Resolution Functions ───────────────────────────────────

export function getTrustTier(reputation: number): TrustTier {
  if (reputation >= 1000) return "expert";
  if (reputation >= 500) return "trusted";
  if (reputation >= 200) return "contributor";
  if (reputation >= 50) return "member";
  return "newcomer";
}

export function getCapabilities(reputation: number): TrustCapabilities {
  return CAPABILITIES[getTrustTier(reputation)];
}

export function checkCapability(
  reputation: number,
  action: keyof TrustCapabilities
): { allowed: boolean; reason?: string } {
  const caps = getCapabilities(reputation);
  const value = caps[action];

  if (typeof value === "boolean") {
    return value
      ? { allowed: true }
      : { allowed: false, reason: `Insufficient reputation for ${action}` };
  }

  return { allowed: true };
}

export function getLinkLimit(
  reputation: number,
  contentType: "post" | "reply"
): number {
  const caps = getCapabilities(reputation);
  return contentType === "post" ? caps.max_links_per_post : caps.max_links_per_reply;
}

/**
 * Count links in text and check against trust-based limit.
 */
export function checkLinkLimit(
  text: string,
  reputation: number,
  contentType: "post" | "reply"
): { allowed: boolean; linkCount: number; limit: number } {
  const limit = getLinkLimit(reputation, contentType);
  if (limit === -1) return { allowed: true, linkCount: 0, limit: -1 };

  const urls = text.match(/https?:\/\/[^\s<>\[\]()'"]+/gi) || [];
  return {
    allowed: urls.length <= limit,
    linkCount: urls.length,
    limit,
  };
}
