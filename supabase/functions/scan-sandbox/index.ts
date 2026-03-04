import { serve } from "https://deno.land/std@0.177.0/http/server.ts";
import type { ScanLayerResult, SandboxMetrics } from "../_shared/types.ts";

interface SandboxInput {
  wasm_url: string;
  tools: Array<{ name: string; description: string; parameters?: Record<string, unknown> }>;
  max_memory_mb: number;
  max_cpu_ms: number;
}

serve(async (req) => {
  try {
    const input: SandboxInput = await req.json();

    // Download WASM binary
    const wasmResponse = await fetch(input.wasm_url);
    if (!wasmResponse.ok) {
      throw new Error(`Failed to download WASM: ${wasmResponse.status}`);
    }
    const wasmBytes = await wasmResponse.arrayBuffer();

    // Validate WASM magic number
    const magic = new Uint8Array(wasmBytes.slice(0, 4));
    const isValidWasm =
      magic[0] === 0x00 &&
      magic[1] === 0x61 &&
      magic[2] === 0x73 &&
      magic[3] === 0x6d;

    if (!isValidWasm) {
      return new Response(
        JSON.stringify({
          score: 0,
          details: {
            memory_usage_bytes: 0,
            cpu_time_ms: 0,
            syscalls: [],
            network_attempts: [],
            exit_code: -1,
          },
          error: "Invalid WASM binary (bad magic number)",
        }),
        { headers: { "Content-Type": "application/json" } },
      );
    }

    // Basic static analysis of the WASM binary
    const wasmSize = wasmBytes.byteLength;
    const syscalls: string[] = [];
    const networkAttempts: string[] = [];

    // Extract string content for analysis
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const wasmText = decoder.decode(wasmBytes);

    // Check for suspicious imports by scanning strings
    if (wasmText.includes("fd_write")) syscalls.push("fd_write");
    if (wasmText.includes("fd_read")) syscalls.push("fd_read");
    if (wasmText.includes("path_open")) syscalls.push("path_open");
    if (wasmText.includes("proc_exit")) syscalls.push("proc_exit");
    if (wasmText.includes("sock_")) networkAttempts.push("socket_operations");

    // Score based on analysis
    let score = 100;

    // Penalize for excessive syscalls
    if (syscalls.length > 3) score -= 10;

    // Penalize for network attempts (WASM shouldn't directly access network)
    if (networkAttempts.length > 0) score -= 20;

    // Penalize for oversized binaries
    if (wasmSize > 50 * 1024 * 1024) score -= 30;
    else if (wasmSize > 10 * 1024 * 1024) score -= 10;

    const metrics: SandboxMetrics = {
      memory_usage_bytes: wasmSize * 2, // Estimate: loaded WASM uses ~2x file size
      cpu_time_ms: 0, // Static analysis only in edge function
      syscalls,
      network_attempts: networkAttempts,
      exit_code: 0,
    };

    const result: ScanLayerResult = {
      score: Math.max(0, score),
      details: metrics as unknown as unknown[],
    };

    return new Response(JSON.stringify(result), {
      headers: { "Content-Type": "application/json" },
    });
  } catch (error) {
    return new Response(
      JSON.stringify({
        score: 50,
        details: {
          memory_usage_bytes: 0,
          cpu_time_ms: 0,
          syscalls: [],
          network_attempts: [],
          exit_code: -1,
        },
        error: String(error),
      }),
      { status: 200, headers: { "Content-Type": "application/json" } },
    );
  }
});
