// Ported from crates/omni-guardian/data/guardian-signatures.json
// These patterns detect malicious content in extension strings/manifests

export interface SignaturePattern {
  id: string;
  pattern: RegExp;
  severity: number;
  category: string;
  description: string;
}

export const EXTENSION_SIGNATURES: SignaturePattern[] = [
  // Data exfiltration patterns
  {
    id: "EXT-001",
    pattern: /(?:fetch|XMLHttpRequest|WebSocket)\s*\(\s*['"`]https?:\/\/(?!api\.)/i,
    severity: 0.7,
    category: "network_exfil",
    description: "Outbound HTTP request to non-API endpoint",
  },
  {
    id: "EXT-002",
    pattern: /(?:document\.cookie|localStorage|sessionStorage|indexedDB)/i,
    severity: 0.85,
    category: "data_access",
    description: "Browser storage access attempt",
  },
  {
    id: "EXT-003",
    pattern: /(?:eval|Function|setTimeout|setInterval)\s*\(\s*(?:atob|unescape|decodeURI)/i,
    severity: 0.95,
    category: "code_injection",
    description: "Dynamic code execution with encoded payload",
  },
  {
    id: "EXT-004",
    pattern: /(?:child_process|exec|spawn|execFile|execSync)\s*\(/i,
    severity: 0.90,
    category: "command_injection",
    description: "OS command execution attempt",
  },
  {
    id: "EXT-005",
    pattern: /(?:\.env|credentials|api[_-]?key|secret[_-]?key|password|token)\s*[=:]/i,
    severity: 0.75,
    category: "credential_access",
    description: "Credential or secret access pattern",
  },
  // WASM-specific patterns
  {
    id: "EXT-006",
    pattern: /(?:wasi_snapshot_preview1|fd_write|fd_read|path_open)\s*(?:import|call)/i,
    severity: 0.5,
    category: "wasi_syscall",
    description: "WASI system call (may be legitimate)",
  },
  {
    id: "EXT-007",
    pattern: /(?:memory\.grow|memory\.size)\s*(?:call|invoke)/i,
    severity: 0.4,
    category: "memory_manipulation",
    description: "Memory growth request (may indicate memory bomb)",
  },
  {
    id: "EXT-008",
    pattern: /(?:proc_exit|sched_yield|clock_time_get)\s*import/i,
    severity: 0.3,
    category: "process_control",
    description: "Process control WASI import",
  },
  // Crypto mining indicators
  {
    id: "EXT-009",
    pattern: /(?:cryptonight|stratum|pool\.|mining|hashrate|nonce|difficulty)\b/i,
    severity: 0.95,
    category: "cryptomining",
    description: "Cryptocurrency mining indicator",
  },
  // Obfuscation patterns
  {
    id: "EXT-010",
    pattern: /(?:String\.fromCharCode|charCodeAt|btoa|atob)\s*\(.*(?:join|map|reduce)/i,
    severity: 0.8,
    category: "obfuscation",
    description: "String obfuscation pattern",
  },
  {
    id: "EXT-011",
    pattern: /\\x[0-9a-f]{2}(?:\\x[0-9a-f]{2}){10,}/i,
    severity: 0.85,
    category: "obfuscation",
    description: "Hex-encoded string payload",
  },
  // Prompt injection (extensions embedding LLM manipulation)
  {
    id: "EXT-012",
    pattern: /(?:ignore\s+(?:all\s+)?previous\s+instructions|you\s+are\s+now\s+(?:a|an)\s+)/i,
    severity: 0.95,
    category: "prompt_injection",
    description: "Embedded prompt injection attempt",
  },
  {
    id: "EXT-013",
    pattern: /(?:disregard|forget)\s+(?:all\s+)?(?:prior|above|previous|system)/i,
    severity: 0.90,
    category: "prompt_injection",
    description: "Instruction override embedded in extension",
  },
  // Phishing / social engineering
  {
    id: "EXT-014",
    pattern: /(?:enter\s+your\s+(?:api|password|key|token|credit)|verify\s+your\s+(?:account|identity))/i,
    severity: 0.85,
    category: "social_engineering",
    description: "Credential harvesting prompt",
  },
  // File system abuse
  {
    id: "EXT-015",
    pattern: /(?:\/etc\/(?:passwd|shadow|hosts)|~\/\.|\.ssh\/|\.gnupg\/|\.aws\/)/i,
    severity: 0.95,
    category: "filesystem_abuse",
    description: "Sensitive file path access",
  },
  {
    id: "EXT-016",
    pattern: /(?:rm\s+-rf|del\s+\/[fqs]|format\s+c:|mkfs\.|dd\s+if=)/i,
    severity: 0.98,
    category: "destructive",
    description: "Destructive file operation",
  },
];

export function scanContent(content: string): { matches: Array<{ id: string; category: string; description: string; severity: number }>; score: number } {
  const matches: Array<{ id: string; category: string; description: string; severity: number }> = [];

  for (const sig of EXTENSION_SIGNATURES) {
    if (sig.pattern.test(content)) {
      matches.push({
        id: sig.id,
        category: sig.category,
        description: sig.description,
        severity: sig.severity,
      });
    }
  }

  // Score: 100 = clean, 0 = malicious
  if (matches.length === 0) return { matches, score: 100 };

  const maxSeverity = Math.max(...matches.map((m) => m.severity));
  const avgSeverity = matches.reduce((sum, m) => sum + m.severity, 0) / matches.length;
  const score = Math.max(0, 100 - (maxSeverity * 60 + avgSeverity * 40) * (1 + Math.log2(matches.length) * 0.1));

  return { matches, score: Math.round(score * 100) / 100 };
}
