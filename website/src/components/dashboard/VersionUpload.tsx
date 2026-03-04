"use client";

import { useState, useRef, useCallback } from "react";
import { Upload, Package, Loader2, FileText, Check, X, AlertTriangle } from "lucide-react";
import { parse as parseToml } from "smol-toml";
import { createClient } from "@/lib/supabase/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

interface VersionUploadProps {
  extensionId: string;
}

interface ParsedManifest {
  extension?: {
    id?: string;
    version?: string;
    [key: string]: unknown;
  };
  permissions?: Record<string, unknown>[];
  tools?: Record<string, unknown>[];
  [key: string]: unknown;
}

export function VersionUpload({ extensionId }: VersionUploadProps) {
  const wasmInputRef = useRef<HTMLInputElement>(null);
  const tomlInputRef = useRef<HTMLInputElement>(null);
  const [wasmFile, setWasmFile] = useState<File | null>(null);
  const [tomlFile, setTomlFile] = useState<File | null>(null);
  const [version, setVersion] = useState("");
  const [changelog, setChangelog] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [parsedManifest, setParsedManifest] = useState<ParsedManifest | null>(null);
  const [versionAutoDetected, setVersionAutoDetected] = useState(false);
  const [parseError, setParseError] = useState<string | null>(null);

  const parseManifestToml = useCallback(async (file: File) => {
    setParseError(null);
    setParsedManifest(null);
    try {
      const text = await file.text();
      const parsed = parseToml(text) as unknown as ParsedManifest;
      setParsedManifest(parsed);

      if (parsed.extension?.version) {
        setVersion(String(parsed.extension.version));
        setVersionAutoDetected(true);
      }
    } catch (err) {
      setParseError(
        `Failed to parse manifest: ${err instanceof Error ? err.message : "Invalid TOML"}`,
      );
    }
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!wasmFile) return;
    setError(null);
    setLoading(true);
    setSuccess(false);

    try {
      const supabase = createClient();

      const wasmPath = `${extensionId}/${version}/${wasmFile.name}`;
      const { error: uploadError } = await supabase.storage
        .from("extension-wasm")
        .upload(wasmPath, wasmFile);
      if (uploadError) throw uploadError;

      const { data: urlData } = supabase.storage
        .from("extension-wasm")
        .getPublicUrl(wasmPath);

      const buffer = await wasmFile.arrayBuffer();
      const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      const checksum =
        "sha256:" + hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

      const manifestForDb = parsedManifest ?? {
        extension: { id: extensionId, version },
      };

      const { error: verError } = await supabase.from("extension_versions").insert({
        extension_id: extensionId,
        version,
        changelog: changelog || null,
        wasm_url: urlData.publicUrl,
        wasm_size_bytes: wasmFile.size,
        checksum,
        manifest: manifestForDb,
        permissions: parsedManifest?.permissions ?? [],
        tools: parsedManifest?.tools ?? [],
        published: true,
      } as any);
      if (verError) throw verError;

      // Update latest_version on the extension
      await (supabase.from("extensions") as any)
        .update({ latest_version: version, updated_at: new Date().toISOString() })
        .eq("id", extensionId);

      setSuccess(true);
      setWasmFile(null);
      setTomlFile(null);
      setVersion("");
      setChangelog("");
      setParsedManifest(null);
      setVersionAutoDetected(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Upload failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-5">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
        <div>
          <label className="text-[13px] font-medium mb-1.5 block">
            Version
            {versionAutoDetected && (
              <span className="inline-flex items-center gap-1 text-[10px] font-mono text-primary ml-2">
                <Check className="h-3 w-3" /> auto-detected
              </span>
            )}
          </label>
          <Input
            placeholder="1.0.0"
            value={version}
            onChange={(e) => {
              setVersion(e.target.value);
              setVersionAutoDetected(false);
            }}
            required
            pattern="^\d+\.\d+\.\d+$"
            className={versionAutoDetected ? "border-primary/30" : ""}
          />
        </div>
        <div>
          <label className="text-[13px] font-medium mb-1.5 block">Files</label>
          <div className="flex gap-2">
            <input
              ref={wasmInputRef}
              type="file"
              accept=".wasm"
              className="hidden"
              onChange={(e) => setWasmFile(e.target.files?.[0] || null)}
            />
            <input
              ref={tomlInputRef}
              type="file"
              accept=".toml"
              className="hidden"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) {
                  setTomlFile(f);
                  parseManifestToml(f);
                }
              }}
            />
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="flex-1 gap-2"
              onClick={() => wasmInputRef.current?.click()}
            >
              {wasmFile ? (
                <>
                  <Package className="h-3.5 w-3.5 text-primary" />
                  <span className="truncate text-[13px]">{wasmFile.name}</span>
                </>
              ) : (
                <>
                  <Upload className="h-3.5 w-3.5" />
                  <span className="text-[13px]">.wasm</span>
                </>
              )}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => tomlInputRef.current?.click()}
            >
              {tomlFile ? (
                <>
                  <FileText className="h-3.5 w-3.5 text-primary" />
                  <Check className="h-3 w-3 text-primary" />
                </>
              ) : (
                <>
                  <FileText className="h-3.5 w-3.5" />
                  <span className="text-[13px]">.toml</span>
                </>
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* File status */}
      {(wasmFile || tomlFile) && (
        <div className="flex flex-wrap gap-3">
          {wasmFile && (
            <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
              <Package className="h-3 w-3" />
              {wasmFile.name}
              <span className="text-muted-foreground/50">({(wasmFile.size / 1024).toFixed(1)} KB)</span>
              <button type="button" onClick={() => setWasmFile(null)} className="hover:text-foreground">
                <X className="h-3 w-3" />
              </button>
            </span>
          )}
          {tomlFile && (
            <span className="inline-flex items-center gap-1.5 text-xs text-primary">
              <FileText className="h-3 w-3" />
              {tomlFile.name}
              <span className="font-mono text-[10px]">parsed</span>
              <button
                type="button"
                onClick={() => {
                  setTomlFile(null);
                  setParsedManifest(null);
                  setVersionAutoDetected(false);
                }}
                className="hover:text-foreground"
              >
                <X className="h-3 w-3" />
              </button>
            </span>
          )}
        </div>
      )}

      {parseError && (
        <div className="flex items-start gap-2 text-sm text-destructive">
          <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
          <span>{parseError}</span>
        </div>
      )}

      <div>
        <label className="text-[13px] font-medium mb-1.5 block">Changelog (optional)</label>
        <Textarea
          value={changelog}
          onChange={(e) => setChangelog(e.target.value)}
          placeholder="What's new in this version..."
          rows={3}
        />
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}
      {success && (
        <div className="flex items-center gap-2 text-[13px] text-success">
          <Check className="h-3.5 w-3.5" />
          Version uploaded successfully. Security scan has been queued.
        </div>
      )}

      <Button type="submit" disabled={loading || !wasmFile || !version || !parsedManifest} size="sm" className="gap-2">
        {loading ? (
          <>
            <Loader2 className="h-4 w-4 animate-spin" />
            Uploading...
          </>
        ) : (
          "Upload Version"
        )}
      </Button>
    </form>
  );
}
