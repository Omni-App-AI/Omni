// ── Cloudflare Turnstile Server Verification ───────────────

const VERIFY_URL = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
const TURNSTILE_DISABLED = process.env.NEXT_PUBLIC_TURNSTILE_DISABLED === "true";

export interface TurnstileResult {
  success: boolean;
  challenge_ts?: string;
  hostname?: string;
  error_codes: string[];
  action?: string;
  cdata?: string;
}

export async function verifyTurnstile(
  token: string,
  ip?: string
): Promise<TurnstileResult> {
  if (TURNSTILE_DISABLED) {
    return { success: true, error_codes: [] };
  }

  const secretKey = process.env.TURNSTILE_SECRET_KEY;

  if (!secretKey) {
    console.error("[turnstile] TURNSTILE_SECRET_KEY not configured");
    return { success: false, error_codes: ["missing-secret-key"] };
  }

  if (!token || token.trim() === "") {
    return { success: false, error_codes: ["missing-input-response"] };
  }

  try {
    const body = new URLSearchParams({
      secret: secretKey,
      response: token,
    });

    if (ip) {
      body.set("remoteip", ip);
    }

    const response = await fetch(VERIFY_URL, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: body.toString(),
    });

    if (!response.ok) {
      return {
        success: false,
        error_codes: [`http-error-${response.status}`],
      };
    }

    const result = (await response.json()) as TurnstileResult;
    return result;
  } catch (error) {
    console.error("[turnstile] Verification failed:", error);
    return {
      success: false,
      error_codes: ["network-error"],
    };
  }
}

/**
 * Checks whether Turnstile should be enforced for a given trust tier.
 * Returns true if the user must pass Turnstile, false if they can skip it.
 */
export function isTurnstileRequired(
  mode: "managed" | "invisible" | "trust-gated" | undefined,
  trustTier: string
): boolean {
  if (TURNSTILE_DISABLED) return false;
  if (!mode) return false;
  if (mode === "managed" || mode === "invisible") return true;

  // trust-gated: required only for newcomers
  return trustTier === "newcomer";
}
