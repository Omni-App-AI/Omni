// Anti-Bot Defense System -- Re-exports

export { verifyTurnstile, isTurnstileRequired, type TurnstileResult } from "./turnstile";
export {
  RATE_LIMITS,
  getRateLimit,
  checkRateLimit,
  recordRateLimitHit,
  rateLimitHeaders,
  type RateLimitConfig,
  type RateLimitResult,
  type TrustTier,
} from "./rate-limits";
export { analyzeContent, analyzeAuthorBehavior, type SpamAnalysis, type SpamSignal, type AuthorContext } from "./spam-detector";
export { validateHoneypot, generateTimestamp, generateToken, type HoneypotPayload, type HoneypotResult } from "./honeypot";
export { getTrustTier, getCapabilities, checkCapability, getLinkLimit, checkLinkLimit, type TrustCapabilities } from "./trust";
export { extractIP, hashIP, isBlockedIP, incrementIPCounter } from "./ip-utils";
export { logSecurityEvent, autoFlagContent, type SecurityEventType } from "./security-logger";
export { isDomainSuspicious, containsSpamKeywords } from "./suspicious-domains";
export { withProtection, type ProtectionConfig, type ProtectedContext, type ProtectedHandler } from "./with-protection";
