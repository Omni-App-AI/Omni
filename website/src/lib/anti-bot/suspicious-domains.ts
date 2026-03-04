// ── Known Suspicious Domains & Patterns ─────────────────────

export const SUSPICIOUS_EXACT_DOMAINS = new Set([
  // URL shorteners
  "bit.ly",
  "tinyurl.com",
  "t.co",
  "goo.gl",
  "ow.ly",
  "is.gd",
  "v.gd",
  "buff.ly",
  "rebrand.ly",
  "cutt.ly",
  "shorturl.at",
  "rb.gy",

  // Known spam/phishing hosting
  "blogspot.com",
  "weebly.com",
  "wix.com",
  "sites.google.com",
]);

export const SUSPICIOUS_TLD_PATTERNS = [
  /\.(xyz|top|club|wang|loan|click|racing|download|stream|gq|ml|cf|tk|ga)$/i,
];

export const SPAM_KEYWORD_PATTERNS = [
  // Crypto/gambling
  /\b(crypto[\s-]?airdrop|free[\s-]?bitcoin|nft[\s-]?giveaway|casino[\s-]?bonus)\b/i,
  /\b(online[\s-]?casino|poker[\s-]?room|sports[\s-]?betting|slot[\s-]?machine)\b/i,
  /\b(buy[\s-]?crypto|earn[\s-]?bitcoin|mining[\s-]?pool|token[\s-]?presale)\b/i,

  // Pharma spam
  /\b(buy[\s-]?viagra|cheap[\s-]?cialis|online[\s-]?pharmacy|weight[\s-]?loss[\s-]?pill)\b/i,

  // SEO spam
  /\b(buy[\s-]?backlinks|seo[\s-]?service|rank[\s-]?#?1|guaranteed[\s-]?traffic)\b/i,
  /\b(cheap[\s-]?followers|buy[\s-]?likes|instagram[\s-]?followers|youtube[\s-]?views)\b/i,

  // Scam patterns
  /\b(make[\s-]?money[\s-]?fast|work[\s-]?from[\s-]?home[\s-]?earn|passive[\s-]?income[\s-]?guarantee)\b/i,
  /\b(congratulations[\s-]?you[\s-]?won|claim[\s-]?your[\s-]?prize|lottery[\s-]?winner)\b/i,

  // Malware/phishing
  /\b(download[\s-]?free[\s-]?crack|keygen|serial[\s-]?key[\s-]?generator)\b/i,
  /\b(verify[\s-]?your[\s-]?account|confirm[\s-]?identity|update[\s-]?payment)\b/i,
];

export function isDomainSuspicious(domain: string): boolean {
  const lower = domain.toLowerCase();
  if (SUSPICIOUS_EXACT_DOMAINS.has(lower)) return true;
  return SUSPICIOUS_TLD_PATTERNS.some((p) => p.test(lower));
}

export function containsSpamKeywords(text: string): string[] {
  const matches: string[] = [];
  for (const pattern of SPAM_KEYWORD_PATTERNS) {
    const match = text.match(pattern);
    if (match) {
      matches.push(match[0]!);
    }
  }
  return matches;
}
