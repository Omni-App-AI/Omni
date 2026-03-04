export interface ScanLayerResult {
  score: number; // 0-100 (100 = clean)
  details: unknown[];
}

export interface SignatureMatch {
  pattern_id: string;
  category: string;
  description: string;
  severity: number;
}

export interface HeuristicRule {
  rule: string;
  score: number;
  description: string;
  severity: "low" | "medium" | "high";
}

export interface AiFlag {
  severity: "low" | "medium" | "high" | "critical";
  finding: string;
  recommendation: string;
}

export interface SandboxMetrics {
  memory_usage_bytes: number;
  cpu_time_ms: number;
  syscalls: string[];
  network_attempts: string[];
  exit_code: number;
}

export interface FullScanResult {
  signature_score: number;
  signature_matches: SignatureMatch[];
  heuristic_score: number;
  heuristic_details: HeuristicRule[];
  ai_score: number;
  ai_analysis: string;
  ai_flags: AiFlag[];
  sandbox_score: number;
  sandbox_results: SandboxMetrics;
  overall_score: number;
  verdict: "clean" | "suspicious" | "malicious" | "error";
  auto_approved: boolean;
  scan_duration_ms: number;
}

export interface ExtensionVersionRow {
  id: string;
  extension_id: string;
  version: string;
  wasm_url: string;
  wasm_size_bytes: number;
  permissions: unknown[];
  tools: unknown[];
  manifest: Record<string, unknown>;
}
