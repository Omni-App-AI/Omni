// ── Honeypot Validation ────────────────────────────────────

export interface HoneypotPayload {
  hp_website?: string;
  hp_timestamp?: string;
  hp_token?: string;
}

export interface HoneypotResult {
  passed: boolean;
  signals: string[];
}

const MIN_SUBMIT_TIME_MS = 3000; // 3 seconds minimum
const HONEYPOT_SECRET = "omni-hp-2026"; // Used to validate JS tokens

/**
 * Generate an obfuscated timestamp for the honeypot timing field.
 * Called client-side on form mount.
 */
export function generateTimestamp(): string {
  return btoa(Date.now().toString(36));
}

/**
 * Generate a JS-proof token (client-side).
 * Proves the form was rendered in a browser with JS execution.
 */
export function generateToken(): string {
  const payload = Date.now().toString(36) + ":" + HONEYPOT_SECRET;
  return btoa(payload);
}

/**
 * Server-side validation of honeypot fields.
 */
export function validateHoneypot(fields: HoneypotPayload): HoneypotResult {
  const signals: string[] = [];

  // 1. Hidden field must be empty -- bots auto-fill visible-looking fields
  if (fields.hp_website && fields.hp_website.trim() !== "") {
    signals.push("honeypot_filled");
  }

  // 2. Timing check -- form must have been visible for at least 3 seconds
  if (fields.hp_timestamp) {
    try {
      const decoded = atob(fields.hp_timestamp);
      const renderTime = parseInt(decoded, 36);
      const elapsed = Date.now() - renderTime;

      if (elapsed < MIN_SUBMIT_TIME_MS) {
        signals.push(`too_fast_${elapsed}ms`);
      }

      // Also reject if timestamp is in the future or absurdly old (>1 hour)
      if (elapsed < 0 || elapsed > 3600000) {
        signals.push("invalid_timestamp");
      }
    } catch {
      signals.push("malformed_timestamp");
    }
  } else {
    signals.push("missing_timestamp");
  }

  // 3. JS token must be present and valid
  if (fields.hp_token) {
    try {
      const decoded = atob(fields.hp_token);
      const parts = decoded.split(":");
      if (parts.length !== 2 || parts[1] !== HONEYPOT_SECRET) {
        signals.push("invalid_token");
      }
    } catch {
      signals.push("malformed_token");
    }
  } else {
    signals.push("missing_token");
  }

  return {
    passed: signals.length === 0,
    signals,
  };
}
