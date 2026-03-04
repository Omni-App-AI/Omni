"use client";

import { useState, useRef, useCallback } from "react";
import { useRouter } from "next/navigation";
import { Upload, Package, Loader2, FileText, Check, X, AlertTriangle } from "lucide-react";
import { parse as parseToml } from "smol-toml";
import { createClient } from "@/lib/supabase/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Select } from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { CATEGORIES } from "@/lib/constants";

/* ---------- types for parsed manifest ---------- */

interface ParsedManifest {
  extension?: {
    id?: string;
    name?: string;
    version?: string;
    author?: string;
    description?: string;
    license?: string;
    homepage?: string;
    repository?: string;
    categories?: string[];
  };
  runtime?: {
    type?: string;
    entrypoint?: string;
    max_memory_mb?: number;
    max_cpu_ms_per_call?: number;
    max_concurrent_calls?: number;
  };
  permissions?: {
    capability?: string;
    scope?: Record<string, unknown>;
    reason?: string;
    required?: boolean;
  }[];
  tools?: {
    name?: string;
    description?: string;
    parameters?: Record<string, unknown>;
  }[];
  hooks?: {
    on_install?: boolean;
    on_message?: boolean;
    on_schedule?: string;
  };
  config?: {
    fields?: Record<
      string,
      { type?: string; label?: string; help?: string; sensitive?: boolean; required?: boolean }
    >;
  };
}

/* ---------- WASM binary helpers ---------- */

function readLeb128(bytes: Uint8Array, offset: number): { value: number; bytesRead: number } {
  let result = 0;
  let shift = 0;
  let bytesRead = 0;
  let byte: number;
  do {
    if (offset + bytesRead >= bytes.length) break;
    byte = bytes[offset + bytesRead];
    result |= (byte & 0x7f) << shift;
    shift += 7;
    bytesRead++;
  } while (byte & 0x80);
  return { value: result, bytesRead };
}

function extractWasmCustomSections(buffer: ArrayBuffer): string | null {
  const bytes = new Uint8Array(buffer);
  if (bytes.length < 8) return null;
  if (bytes[0] !== 0x00 || bytes[1] !== 0x61 || bytes[2] !== 0x73 || bytes[3] !== 0x6d) {
    return null;
  }

  let offset = 8;
  while (offset < bytes.length) {
    const sectionId = bytes[offset];
    offset++;
    const { value: sectionLen, bytesRead } = readLeb128(bytes, offset);
    offset += bytesRead;
    const sectionEnd = offset + sectionLen;

    if (sectionId === 0) {
      const { value: nameLen, bytesRead: nameLenBytes } = readLeb128(bytes, offset);
      const nameStart = offset + nameLenBytes;
      const nameBytes = bytes.slice(nameStart, nameStart + nameLen);
      const name = new TextDecoder().decode(nameBytes);

      if (name === "omni-manifest" || name === "omni_manifest") {
        const dataStart = nameStart + nameLen;
        const dataBytes = bytes.slice(dataStart, sectionEnd);
        return new TextDecoder().decode(dataBytes);
      }
    }
    offset = sectionEnd;
  }
  return null;
}

function extractWasmStrings(buffer: ArrayBuffer): string[] {
  const bytes = new Uint8Array(buffer);
  const strings: string[] = [];
  let current = "";

  for (const byte of bytes) {
    if (byte >= 32 && byte <= 126) {
      current += String.fromCharCode(byte);
    } else {
      if (current.length >= 4) strings.push(current);
      current = "";
    }
  }
  if (current.length >= 4) strings.push(current);
  return strings;
}

function detectExtensionId(strings: string[]): string | null {
  const idPattern = /^[a-z][a-z0-9]*(\.[a-z0-9_-]+){2,}$/;
  for (const s of strings) {
    const trimmed = s.trim();
    if (idPattern.test(trimmed) && trimmed.length >= 5 && trimmed.length <= 100) {
      return trimmed;
    }
  }
  return null;
}

/* ---------- component ---------- */

type AutoField = "id" | "name" | "shortDescription" | "description" | "version" | "homepage" | "repository" | "license";

export function ExtensionForm() {
  const router = useRouter();
  const wasmInputRef = useRef<HTMLInputElement>(null);
  const tomlInputRef = useRef<HTMLInputElement>(null);
  const dropRef = useRef<HTMLDivElement>(null);

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [wasmFile, setWasmFile] = useState<File | null>(null);
  const [tomlFile, setTomlFile] = useState<File | null>(null);
  const [selectedCategories, setSelectedCategories] = useState<string[]>([]);
  const [parsedManifest, setParsedManifest] = useState<ParsedManifest | null>(null);
  const [autoFields, setAutoFields] = useState<Set<AutoField>>(new Set());
  const [parseError, setParseError] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [iconFile, setIconFile] = useState<File | null>(null);
  const [iconPreview, setIconPreview] = useState<string | null>(null);
  const iconInputRef = useRef<HTMLInputElement>(null);

  const [form, setForm] = useState({
    id: "",
    name: "",
    shortDescription: "",
    description: "",
    version: "0.1.0",
    homepage: "",
    repository: "",
    license: "MIT",
  });

  const updateForm = (key: string, value: string) => {
    setForm((prev) => ({ ...prev, [key]: value }));
    setAutoFields((prev) => {
      const next = new Set(prev);
      next.delete(key as AutoField);
      return next;
    });
  };

  const toggleCategory = (catId: string) => {
    setSelectedCategories((prev) =>
      prev.includes(catId)
        ? prev.filter((c) => c !== catId)
        : prev.length < 5
          ? [...prev, catId]
          : prev,
    );
  };

  /* ---------- manifest parsing ---------- */

  const applyManifest = useCallback((manifest: ParsedManifest) => {
    setParsedManifest(manifest);
    const ext = manifest.extension;
    if (!ext) return;

    const detected = new Set<AutoField>();
    const updates: Partial<typeof form> = {};

    if (ext.id) { updates.id = ext.id; detected.add("id"); }
    if (ext.name) { updates.name = ext.name; detected.add("name"); }
    if (ext.description) {
      updates.description = ext.description;
      detected.add("description");
      updates.shortDescription = ext.description.substring(0, 160);
      detected.add("shortDescription");
    }
    if (ext.version) { updates.version = ext.version; detected.add("version"); }
    if (ext.homepage) { updates.homepage = ext.homepage; detected.add("homepage"); }
    if (ext.repository) { updates.repository = ext.repository; detected.add("repository"); }
    if (ext.license) { updates.license = ext.license; detected.add("license"); }

    setForm((prev) => ({ ...prev, ...updates }));
    setAutoFields(detected);

    if (ext.categories?.length) {
      const catIds = CATEGORIES.map((c) => c.id);
      const matched = ext.categories.filter((c) => catIds.includes(c as typeof catIds[number]));
      if (matched.length > 0) setSelectedCategories(matched.slice(0, 5));
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const parseManifestToml = useCallback(
    async (file: File) => {
      setParseError(null);
      try {
        const text = await file.text();
        const parsed = parseToml(text) as unknown as ParsedManifest;
        applyManifest(parsed);
      } catch (err) {
        setParseError(
          `Failed to parse manifest: ${err instanceof Error ? err.message : "Invalid TOML"}`,
        );
      }
    },
    [applyManifest],
  );

  const processWasmFile = useCallback(
    async (file: File) => {
      setWasmFile(file);
      try {
        const buffer = await file.arrayBuffer();

        const customToml = extractWasmCustomSections(buffer);
        if (customToml) {
          try {
            const parsed = parseToml(customToml) as unknown as ParsedManifest;
            applyManifest(parsed);
            return;
          } catch {
            // Fall through to string extraction
          }
        }

        const strings = extractWasmStrings(buffer);
        const detectedId = detectExtensionId(strings);
        if (detectedId && !form.id) {
          setForm((prev) => ({ ...prev, id: detectedId }));
          setAutoFields((prev) => new Set(prev).add("id"));
        }
      } catch {
        // WASM parsing failed silently
      }
    },
    [applyManifest, form.id],
  );

  /* ---------- file handling ---------- */

  const handleFiles = useCallback(
    (files: FileList | File[]) => {
      for (const file of Array.from(files)) {
        if (file.name.endsWith(".wasm")) {
          processWasmFile(file);
        } else if (
          file.name.endsWith(".toml") ||
          file.name === "omni-extension.toml" ||
          file.name === "manifest.toml"
        ) {
          setTomlFile(file);
          parseManifestToml(file);
        }
      }
    },
    [processWasmFile, parseManifestToml],
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      if (e.dataTransfer.files.length) handleFiles(e.dataTransfer.files);
    },
    [handleFiles],
  );

  /* ---------- submit ---------- */

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!wasmFile) {
      setError("Please upload a WASM file.");
      return;
    }
    setError(null);
    setLoading(true);

    try {
      const supabase = createClient();
      const {
        data: { user },
      } = await supabase.auth.getUser();
      if (!user) throw new Error("Not authenticated");

      // Compute checksum before any DB/storage operations
      const buffer = await wasmFile.arrayBuffer();
      const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      const checksum =
        "sha256:" + hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

      const manifestForDb = parsedManifest ?? {
        extension: {
          id: form.id,
          name: form.name,
          version: form.version,
          description: form.description,
        },
      };

      // Create extension record FIRST so the storage policy can verify ownership
      // (storage INSERT policy requires extension to exist with publisher_id = auth.uid())
      const { error: extError } = await supabase.from("extensions").insert({
        id: form.id,
        publisher_id: user.id,
        name: form.name,
        description: form.description,
        short_description: form.shortDescription,
        categories: selectedCategories,
        homepage: form.homepage || null,
        repository: form.repository || null,
        license: form.license || null,
        icon_url: null,
        published: true,
        latest_version: form.version,
      } as any);
      if (extError) throw extError;

      // Now upload files (extension exists, storage ownership check passes)
      try {
        const wasmPath = `${form.id}/${form.version}/${wasmFile.name}`;
        const { error: uploadError } = await supabase.storage
          .from("extension-wasm")
          .upload(wasmPath, wasmFile);
        if (uploadError) throw uploadError;

        const { data: urlData } = supabase.storage
          .from("extension-wasm")
          .getPublicUrl(wasmPath);

        // Upload icon if provided
        let iconUrl: string | null = null;
        if (iconFile) {
          const iconExt = iconFile.name.split(".").pop() || "png";
          const iconPath = `${form.id}/icon.${iconExt}`;
          const { error: iconUploadError } = await supabase.storage
            .from("extension-images")
            .upload(iconPath, iconFile, { upsert: true });
          if (iconUploadError) throw iconUploadError;
          const { data: iconUrlData } = supabase.storage
            .from("extension-images")
            .getPublicUrl(iconPath);
          iconUrl = iconUrlData.publicUrl;
        }

        // Update extension with icon URL if uploaded
        if (iconUrl) {
          await (supabase.from("extensions") as any)
            .update({ icon_url: iconUrl })
            .eq("id", form.id);
        }

        const { error: verError } = await supabase.from("extension_versions").insert({
          extension_id: form.id,
          version: form.version,
          wasm_url: urlData.publicUrl,
          wasm_size_bytes: wasmFile.size,
          checksum,
          manifest: manifestForDb,
          permissions: parsedManifest?.permissions ?? [],
          tools: parsedManifest?.tools ?? [],
          published: true,
        } as any);
        if (verError) throw verError;
      } catch (uploadErr) {
        // Clean up the extension record if file upload or version creation fails
        await supabase.from("extensions").delete().eq("id", form.id);
        throw uploadErr;
      }

      router.push(`/dashboard/extensions/${form.id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Something went wrong");
      setLoading(false);
    }
  };

  /* ---------- helpers ---------- */

  const autoTag = (field: AutoField) =>
    autoFields.has(field) ? (
      <span className="inline-flex items-center gap-1 text-[10px] font-mono text-primary ml-2">
        <Check className="h-3 w-3" /> auto-detected
      </span>
    ) : null;

  const detectedPermissions = parsedManifest?.permissions ?? [];
  const detectedTools = parsedManifest?.tools ?? [];
  const detectedRuntime = parsedManifest?.runtime;

  /* ---------- render ---------- */

  return (
    <form onSubmit={handleSubmit} className="space-y-8">
      {/* Upload Zone */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Upload Files</h2>
        <div
          ref={dropRef}
          onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
          onDragLeave={() => setDragOver(false)}
          onDrop={handleDrop}
          className={`border-2 border-dashed rounded-lg p-10 text-center transition-colors ${
            dragOver
              ? "border-primary bg-primary/5"
              : "border-border/50 hover:border-primary/30"
          }`}
        >
          <Upload className="h-8 w-8 text-muted-foreground/40 mx-auto mb-3" />
          <p className="text-sm text-muted-foreground mb-1">
            Drag & drop your <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">.wasm</code> and{" "}
            <code className="px-1.5 py-0.5 bg-secondary rounded text-xs font-mono">manifest.toml</code> files here
          </p>
          <p className="text-xs text-muted-foreground/50 mb-5">
            The manifest will auto-fill all form fields below
          </p>
          <div className="flex justify-center gap-3">
            <input
              ref={wasmInputRef}
              type="file"
              accept=".wasm"
              className="hidden"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) processWasmFile(f);
              }}
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
              className="gap-2"
              onClick={() => wasmInputRef.current?.click()}
            >
              <Package className="h-4 w-4" />
              Choose .wasm
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => tomlInputRef.current?.click()}
            >
              <FileText className="h-4 w-4" />
              Choose manifest.toml
            </Button>
          </div>
        </div>

        {/* File status */}
        {(wasmFile || tomlFile) && (
          <div className="flex flex-wrap gap-3 mt-4">
            {wasmFile && (
              <div className="flex items-center gap-2 px-3 py-1.5 border border-border/50 rounded-md text-[13px]">
                <Package className="h-3.5 w-3.5 text-primary" />
                <span className="font-medium">{wasmFile.name}</span>
                <span className="text-muted-foreground text-xs">
                  ({(wasmFile.size / 1024).toFixed(1)} KB)
                </span>
                <button
                  type="button"
                  onClick={() => setWasmFile(null)}
                  className="text-muted-foreground hover:text-foreground ml-1"
                >
                  <X className="h-3 w-3" />
                </button>
              </div>
            )}
            {tomlFile && (
              <div className="flex items-center gap-2 px-3 py-1.5 border border-primary/20 rounded-md text-[13px]">
                <FileText className="h-3.5 w-3.5 text-primary" />
                <span className="font-medium">{tomlFile.name}</span>
                <span className="text-[10px] text-primary font-mono ml-1">parsed</span>
                <button
                  type="button"
                  onClick={() => {
                    setTomlFile(null);
                    setParsedManifest(null);
                    setAutoFields(new Set());
                  }}
                  className="text-muted-foreground hover:text-foreground ml-1"
                >
                  <X className="h-3 w-3" />
                </button>
              </div>
            )}
          </div>
        )}

        {parseError && (
          <div className="flex items-start gap-2 text-sm text-destructive mt-3">
            <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
            <span>{parseError}</span>
          </div>
        )}
      </div>

      {/* Extension Details */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Extension Details</h2>
        <div className="border border-border/50 rounded-lg p-6 space-y-5">
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">
              Extension ID {autoTag("id")}
            </label>
            <Input
              placeholder="com.yourname.extension-name"
              value={form.id}
              onChange={(e) => updateForm("id", e.target.value)}
              required
              pattern="^[a-z0-9]+(\.[a-z0-9_-]+){2,}$"
              className={autoFields.has("id") ? "border-primary/30" : ""}
            />
            <p className="text-xs text-muted-foreground/60 mt-1.5">
              Reverse-domain format (e.g., com.example.weather). Cannot be changed later.
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
            <div>
              <label className="text-[13px] font-medium mb-1.5 block">
                Name {autoTag("name")}
              </label>
              <Input
                placeholder="My Extension"
                value={form.name}
                onChange={(e) => updateForm("name", e.target.value)}
                required
                className={autoFields.has("name") ? "border-primary/30" : ""}
              />
            </div>
            <div>
              <label className="text-[13px] font-medium mb-1.5 block">
                Version {autoTag("version")}
              </label>
              <Input
                placeholder="0.1.0"
                value={form.version}
                onChange={(e) => updateForm("version", e.target.value)}
                required
                pattern="^\d+\.\d+\.\d+$"
                className={autoFields.has("version") ? "border-primary/30" : ""}
              />
            </div>
          </div>

          <div>
            <label className="text-[13px] font-medium mb-1.5 block">
              Short Description {autoTag("shortDescription")}
            </label>
            <Input
              placeholder="A brief one-liner (max 160 chars)"
              value={form.shortDescription}
              onChange={(e) => updateForm("shortDescription", e.target.value)}
              required
              maxLength={160}
              className={autoFields.has("shortDescription") ? "border-primary/30" : ""}
            />
            <p className="text-xs text-muted-foreground/60 mt-1.5 font-mono">
              {form.shortDescription.length}/160
            </p>
          </div>

          <div>
            <label className="text-[13px] font-medium mb-1.5 block">
              Description {autoTag("description")}
            </label>
            <Textarea
              placeholder="Full description of your extension..."
              value={form.description}
              onChange={(e) => updateForm("description", e.target.value)}
              required
              rows={5}
              className={autoFields.has("description") ? "border-primary/30" : ""}
            />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
            <div>
              <label className="text-[13px] font-medium mb-1.5 block">
                License {autoTag("license")}
              </label>
              <Select
                value={form.license}
                onChange={(e) => updateForm("license", e.target.value)}
              >
                <option value="MIT">MIT</option>
                <option value="Apache-2.0">Apache 2.0</option>
                <option value="GPL-3.0">GPL 3.0</option>
                <option value="BSD-3-Clause">BSD 3-Clause</option>
                <option value="proprietary">Proprietary</option>
              </Select>
            </div>
            <div>
              <label className="text-[13px] font-medium mb-1.5 block">
                Homepage URL {autoTag("homepage")}
              </label>
              <Input
                type="url"
                placeholder="https://..."
                value={form.homepage}
                onChange={(e) => updateForm("homepage", e.target.value)}
                className={autoFields.has("homepage") ? "border-primary/30" : ""}
              />
            </div>
          </div>

          <div>
            <label className="text-[13px] font-medium mb-1.5 block">
              Repository URL {autoTag("repository")}
            </label>
            <Input
              type="url"
              placeholder="https://github.com/..."
              value={form.repository}
              onChange={(e) => updateForm("repository", e.target.value)}
              className={autoFields.has("repository") ? "border-primary/30" : ""}
            />
          </div>
        </div>
      </div>

      {/* Extension Icon */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Extension Icon</h2>
        <div className="border border-border/50 rounded-lg p-6">
          <label className="text-[13px] font-medium mb-1.5 block">Icon</label>
          <p className="text-xs text-muted-foreground/60 mb-3">
            Recommended: 256x256px. Max 512KB. PNG, JPG, or WebP.
          </p>
          {iconPreview ? (
            <div className="relative group inline-block">
              <img
                src={iconPreview}
                alt="Icon preview"
                className="h-[128px] w-[128px] rounded-lg border border-border/50 object-cover"
              />
              <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex items-center justify-center gap-2">
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  onClick={() => iconInputRef.current?.click()}
                >
                  Replace
                </Button>
                <Button
                  type="button"
                  variant="destructive"
                  size="sm"
                  onClick={() => { setIconFile(null); setIconPreview(null); }}
                >
                  <X className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
          ) : (
            <div
              onClick={() => iconInputRef.current?.click()}
              className="border-2 border-dashed border-border/50 rounded-lg p-6 text-center cursor-pointer hover:border-primary/30 transition-colors max-w-[160px]"
            >
              <Package className="h-6 w-6 text-muted-foreground/40 mx-auto mb-2" />
              <p className="text-xs text-muted-foreground">Click to upload icon</p>
            </div>
          )}
          <input
            ref={iconInputRef}
            type="file"
            accept="image/png,image/jpeg,image/webp"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) {
                if (f.size > 512 * 1024) {
                  setError("Icon must be under 512KB");
                  return;
                }
                setIconFile(f);
                setIconPreview(URL.createObjectURL(f));
              }
            }}
          />
        </div>
      </div>

      {/* Categories */}
      <div>
        <h2 className="text-lg font-semibold mb-1">
          Categories
          {parsedManifest?.extension?.categories?.length ? (
            <span className="inline-flex items-center gap-1 text-[10px] font-mono text-primary ml-2 font-normal">
              <Check className="h-3 w-3" /> auto-detected
            </span>
          ) : null}
        </h2>
        <p className="text-[13px] text-muted-foreground mb-4">Select up to 5 categories.</p>
        <div className="flex flex-wrap gap-2">
          {CATEGORIES.map((cat) => (
            <Badge
              key={cat.id}
              variant={selectedCategories.includes(cat.id) ? "default" : "outline"}
              className="cursor-pointer"
              onClick={() => toggleCategory(cat.id)}
            >
              {cat.name}
            </Badge>
          ))}
        </div>
      </div>

      {/* Detected Permissions */}
      {detectedPermissions.length > 0 && (
        <div>
          <h2 className="text-lg font-semibold mb-4">
            Detected Permissions
            <span className="inline-flex items-center gap-1 text-[10px] font-mono text-primary ml-2 font-normal">
              <Check className="h-3 w-3" /> from manifest
            </span>
          </h2>
          <div className="border border-border/50 rounded-lg divide-y divide-border/50">
            {detectedPermissions.map((perm, i) => (
              <div
                key={i}
                className="flex items-start justify-between gap-3 px-5 py-3"
              >
                <div className="min-w-0">
                  <code className="text-xs font-mono text-foreground">
                    {perm.capability}
                  </code>
                  {perm.reason && (
                    <p className="text-xs text-muted-foreground mt-0.5">{perm.reason}</p>
                  )}
                </div>
                <Badge variant={perm.required !== false ? "default" : "outline"} className="shrink-0 text-[10px]">
                  {perm.required !== false ? "required" : "optional"}
                </Badge>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Detected Tools */}
      {detectedTools.length > 0 && (
        <div>
          <h2 className="text-lg font-semibold mb-4">
            Detected Tools
            <span className="inline-flex items-center gap-1 text-[10px] font-mono text-primary ml-2 font-normal">
              <Check className="h-3 w-3" /> from manifest
            </span>
          </h2>
          <div className="border border-border/50 rounded-lg divide-y divide-border/50">
            {detectedTools.map((tool, i) => (
              <div key={i} className="px-5 py-3">
                <code className="text-xs font-mono text-foreground">{tool.name}</code>
                {tool.description && (
                  <p className="text-xs text-muted-foreground mt-0.5">{tool.description}</p>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Runtime Config */}
      {detectedRuntime && (
        <div>
          <h2 className="text-lg font-semibold mb-4">
            Runtime Configuration
            <span className="inline-flex items-center gap-1 text-[10px] font-mono text-primary ml-2 font-normal">
              <Check className="h-3 w-3" /> from manifest
            </span>
          </h2>
          <div className="border border-border/50 rounded-lg p-5">
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-6">
              {[
                { label: "Type", value: detectedRuntime.type ?? "wasm" },
                { label: "Entrypoint", value: detectedRuntime.entrypoint ?? "—" },
                { label: "Max Memory", value: `${detectedRuntime.max_memory_mb ?? 64} MB` },
                { label: "CPU Timeout", value: `${detectedRuntime.max_cpu_ms_per_call ?? 5000} ms` },
              ].map((item) => (
                <div key={item.label}>
                  <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60">{item.label}</p>
                  <p className="text-sm font-mono mt-1">{item.value}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {error && (
        <div className="flex items-start gap-2 text-sm text-destructive">
          <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
          <span>{error}</span>
        </div>
      )}

      <div className="flex gap-3 pt-2">
        <Button type="submit" disabled={loading} size="sm" className="gap-2">
          {loading ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Publishing...
            </>
          ) : (
            "Publish Extension"
          )}
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => router.back()}>
          Cancel
        </Button>
      </div>
    </form>
  );
}
