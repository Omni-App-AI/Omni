import { isDomainSuspicious, containsSpamKeywords } from "./suspicious-domains";
import { createServiceClient } from "@/lib/supabase/server";

// ── Types ──────────────────────────────────────────────────

export interface AuthorContext {
  userId: string;
  reputation: number;
  accountCreatedAt: string; // ISO timestamp
  postCount: number;
}

export interface SpamSignal {
  name: string;
  weight: number;
  detail?: string;
}

export interface SpamAnalysis {
  score: number; // 0-100
  signals: SpamSignal[];
  verdict: "clean" | "suspicious" | "spam";
}

// ── URL Extraction ─────────────────────────────────────────

const URL_REGEX = /https?:\/\/[^\s<>\[\]()'"]+/gi;

function extractURLs(text: string): string[] {
  return text.match(URL_REGEX) || [];
}

function extractDomain(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}

// ── Signal Detectors ───────────────────────────────────────

function checkExcessiveLinks(text: string, contentType: "post" | "reply"): SpamSignal | null {
  const urls = extractURLs(text);
  const threshold = contentType === "post" ? 3 : 1;
  if (urls.length > threshold) {
    return { name: "excessive_links", weight: 25, detail: `${urls.length} links (limit: ${threshold})` };
  }
  return null;
}

function checkLinkRatio(text: string): SpamSignal | null {
  const urls = extractURLs(text);
  if (urls.length === 0) return null;

  const totalLinkChars = urls.reduce((sum, url) => sum + url.length, 0);
  const ratio = totalLinkChars / Math.max(text.length, 1);
  if (ratio > 0.3) {
    return { name: "link_ratio", weight: 20, detail: `${Math.round(ratio * 100)}% link content` };
  }
  return null;
}

function checkSuspiciousDomains(text: string): SpamSignal | null {
  const urls = extractURLs(text);
  const suspicious = urls
    .map(extractDomain)
    .filter((d) => d && isDomainSuspicious(d));

  if (suspicious.length > 0) {
    return { name: "suspicious_domains", weight: 30, detail: suspicious.join(", ") };
  }
  return null;
}

function checkRepetitiveText(text: string): SpamSignal | null {
  const words = text.toLowerCase().split(/\s+/).filter((w) => w.length > 3);
  if (words.length < 6) return null;

  // Check for repeated phrases (3+ word ngrams appearing 3+ times)
  const ngrams = new Map<string, number>();
  for (let i = 0; i < words.length - 2; i++) {
    const ngram = words.slice(i, i + 3).join(" ");
    ngrams.set(ngram, (ngrams.get(ngram) || 0) + 1);
  }

  const repeated = [...ngrams.entries()].filter(([, count]) => count >= 3);
  if (repeated.length > 0) {
    return { name: "repetitive_text", weight: 15, detail: `${repeated.length} repeated phrases` };
  }
  return null;
}

function checkAllCapsRatio(text: string): SpamSignal | null {
  const alpha = text.replace(/[^a-zA-Z]/g, "");
  if (alpha.length < 20) return null;

  const upperCount = alpha.replace(/[^A-Z]/g, "").length;
  const ratio = upperCount / alpha.length;
  if (ratio > 0.5) {
    return { name: "all_caps_ratio", weight: 10, detail: `${Math.round(ratio * 100)}% uppercase` };
  }
  return null;
}

function checkCryptoSpam(text: string): SpamSignal | null {
  const matches = containsSpamKeywords(text);
  if (matches.length > 0) {
    return { name: "crypto_spam", weight: 25, detail: matches.slice(0, 3).join(", ") };
  }
  return null;
}

function checkContactInfoSpam(text: string): SpamSignal | null {
  const patterns = [
    /\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b/, // email
    /\b(?:\+?1[-.]?)?\(?[0-9]{3}\)?[-.]?[0-9]{3}[-.]?[0-9]{4}\b/, // phone
    /(?:t\.me|telegram\.me)\/[a-zA-Z0-9_]+/i, // Telegram
    /(?:wa\.me|api\.whatsapp\.com)\/[0-9]+/i, // WhatsApp
  ];

  const found = patterns.filter((p) => p.test(text));
  if (found.length > 0) {
    return { name: "contact_info_spam", weight: 20, detail: `${found.length} contact patterns` };
  }
  return null;
}

function checkAsciiArt(text: string): SpamSignal | null {
  const lines = text.split("\n");
  const artLines = lines.filter((line) => {
    const nonAlphaRatio = line.replace(/[a-zA-Z0-9\s]/g, "").length / Math.max(line.length, 1);
    return line.length > 20 && nonAlphaRatio > 0.5;
  });

  if (artLines.length >= 3) {
    return { name: "ascii_art", weight: 10, detail: `${artLines.length} art-like lines` };
  }
  return null;
}

function checkTooShort(text: string, title?: string): SpamSignal | null {
  if (title && title.length < 10) {
    return { name: "too_short", weight: 5, detail: `Title only ${title.length} chars` };
  }
  if (text.length < 20) {
    return { name: "too_short", weight: 5, detail: `Body only ${text.length} chars` };
  }
  return null;
}

function checkNewAccountLinks(text: string, author: AuthorContext): SpamSignal | null {
  const urls = extractURLs(text);
  if (urls.length === 0) return null;

  const accountAge = Date.now() - new Date(author.accountCreatedAt).getTime();
  const oneDayMs = 24 * 60 * 60 * 1000;

  if (accountAge < oneDayMs) {
    return { name: "new_account_links", weight: 20, detail: `Account <24h old with ${urls.length} links` };
  }
  return null;
}

// ── Main Analysis Function ─────────────────────────────────

export function analyzeContent(
  text: string,
  author: AuthorContext,
  options?: { title?: string; contentType?: "post" | "reply" }
): SpamAnalysis {
  const contentType = options?.contentType || "post";
  const signals: SpamSignal[] = [];

  const checks = [
    checkExcessiveLinks(text, contentType),
    checkLinkRatio(text),
    checkSuspiciousDomains(text),
    checkRepetitiveText(text),
    checkAllCapsRatio(text),
    checkCryptoSpam(text),
    checkContactInfoSpam(text),
    checkAsciiArt(text),
    checkTooShort(text, options?.title),
    checkNewAccountLinks(text, author),
  ];

  for (const signal of checks) {
    if (signal) signals.push(signal);
  }

  const score = Math.min(100, signals.reduce((sum, s) => sum + s.weight, 0));

  let verdict: SpamAnalysis["verdict"] = "clean";
  if (score > 60) verdict = "spam";
  else if (score > 30) verdict = "suspicious";

  return { score, signals, verdict };
}

// ── Async Behavior Signals (require DB) ─────────────────────

/**
 * Checks for duplicate content posted recently by the same user.
 * Weight: +30 if identical content found within last hour.
 */
async function checkCopyPasteFlood(
  text: string,
  userId: string,
  contentType: "post" | "reply"
): Promise<SpamSignal | null> {
  try {
    const supabase = createServiceClient();
    const oneHourAgo = new Date(Date.now() - 60 * 60 * 1000).toISOString();
    const table = contentType === "post" ? "forum_posts" : "forum_replies";
    const bodyField = "body";

    const { data } = await (supabase
      .from(table) as any)
      .select(bodyField)
      .eq("author_id", userId)
      .gte("created_at", oneHourAgo)
      .limit(20);

    if (data) {
      const normalizedText = text.trim().toLowerCase();
      const duplicates = data.filter(
        (row: any) => row[bodyField]?.trim().toLowerCase() === normalizedText
      );
      if (duplicates.length > 0) {
        return { name: "copy_paste_flood", weight: 30, detail: `${duplicates.length} identical post(s) in last hour` };
      }
    }
  } catch {
    // DB error -- skip this check
  }
  return null;
}

/**
 * Checks if the user has posted 3+ times in the last 5 minutes.
 * Weight: +15 for burst posting behavior.
 */
async function checkBurstPosting(
  userId: string,
  contentType: "post" | "reply"
): Promise<SpamSignal | null> {
  try {
    const supabase = createServiceClient();
    const fiveMinAgo = new Date(Date.now() - 5 * 60 * 1000).toISOString();
    const table = contentType === "post" ? "forum_posts" : "forum_replies";

    const { count } = await (supabase
      .from(table) as any)
      .select("*", { count: "exact", head: true })
      .eq("author_id", userId)
      .gte("created_at", fiveMinAgo);

    if (count !== null && count >= 3) {
      return { name: "burst_posting", weight: 15, detail: `${count} posts in last 5 minutes` };
    }
  } catch {
    // DB error -- skip this check
  }
  return null;
}

/**
 * Async behavior-based spam analysis (requires DB access).
 * Checks copy-paste flooding and burst posting.
 * Returns additional signals to merge with the synchronous analysis.
 */
export async function analyzeAuthorBehavior(
  text: string,
  userId: string,
  contentType: "post" | "reply"
): Promise<SpamSignal[]> {
  const results = await Promise.all([
    checkCopyPasteFlood(text, userId, contentType),
    checkBurstPosting(userId, contentType),
  ]);

  return results.filter((s): s is SpamSignal => s !== null);
}
