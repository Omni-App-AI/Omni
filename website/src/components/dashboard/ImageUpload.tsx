"use client";

import { useState, useRef, useCallback } from "react";
import { Upload, X, Image as ImageIcon, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";

interface ImageUploadProps {
  label: string;
  currentUrl: string | null;
  onUpload: (file: File) => Promise<string>;
  onRemove?: () => void;
  accept?: string;
  maxSizeKB?: number;
  recommendedSize?: string;
  aspectRatio?: string;
  className?: string;
}

function validateImage(
  file: File,
  maxSizeKB: number,
): Promise<string | null> {
  return new Promise((resolve) => {
    if (!file.type.startsWith("image/")) {
      resolve("File must be an image (PNG, JPG, or WebP)");
      return;
    }
    const allowed = ["image/png", "image/jpeg", "image/webp"];
    if (!allowed.includes(file.type)) {
      resolve("Only PNG, JPG, and WebP images are supported");
      return;
    }
    if (file.size > maxSizeKB * 1024) {
      resolve(`Image must be under ${maxSizeKB >= 1024 ? `${(maxSizeKB / 1024).toFixed(0)}MB` : `${maxSizeKB}KB`}`);
      return;
    }
    resolve(null);
  });
}

export function ImageUpload({
  label,
  currentUrl,
  onUpload,
  onRemove,
  accept = "image/png,image/jpeg,image/webp",
  maxSizeKB = 512,
  recommendedSize = "256x256px",
  aspectRatio,
  className = "",
}: ImageUploadProps) {
  const [preview, setPreview] = useState<string | null>(currentUrl);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleFile = useCallback(
    async (file: File) => {
      setError(null);
      const validationError = await validateImage(file, maxSizeKB);
      if (validationError) {
        setError(validationError);
        return;
      }

      setLoading(true);
      try {
        const url = await onUpload(file);
        setPreview(url);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Upload failed");
      }
      setLoading(false);
    },
    [maxSizeKB, onUpload],
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      const file = e.dataTransfer.files[0];
      if (file) handleFile(file);
    },
    [handleFile],
  );

  const handleRemove = () => {
    setPreview(null);
    setError(null);
    if (inputRef.current) inputRef.current.value = "";
    onRemove?.();
  };

  return (
    <div className={className}>
      <label className="text-[13px] font-medium mb-1.5 block">{label}</label>
      <p className="text-xs text-muted-foreground/60 mb-3">
        Recommended: {recommendedSize}. Max {maxSizeKB >= 1024 ? `${(maxSizeKB / 1024).toFixed(0)}MB` : `${maxSizeKB}KB`}. PNG, JPG, or WebP.
      </p>

      {preview ? (
        <div className="relative group inline-block">
          <img
            src={preview}
            alt={label}
            className="rounded-lg border border-border/50 object-cover"
            style={{
              maxWidth: aspectRatio === "1/1" ? "128px" : "100%",
              maxHeight: aspectRatio === "1/1" ? "128px" : "200px",
              aspectRatio: aspectRatio || "auto",
            }}
          />
          <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity rounded-lg flex items-center justify-center gap-2">
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => inputRef.current?.click()}
            >
              Replace
            </Button>
            {onRemove && (
              <Button
                type="button"
                variant="destructive"
                size="sm"
                onClick={handleRemove}
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        </div>
      ) : (
        <div
          onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
          onDragLeave={() => setDragOver(false)}
          onDrop={handleDrop}
          onClick={() => !loading && inputRef.current?.click()}
          className={`border-2 border-dashed rounded-lg p-6 text-center cursor-pointer transition-colors ${
            dragOver
              ? "border-primary bg-primary/5"
              : "border-border/50 hover:border-primary/30"
          }`}
          style={{
            maxWidth: aspectRatio === "1/1" ? "160px" : "100%",
          }}
        >
          {loading ? (
            <Loader2 className="h-6 w-6 text-muted-foreground/40 mx-auto animate-spin" />
          ) : (
            <>
              <ImageIcon className="h-6 w-6 text-muted-foreground/40 mx-auto mb-2" />
              <p className="text-xs text-muted-foreground">
                Drop image or click to browse
              </p>
            </>
          )}
        </div>
      )}

      <input
        ref={inputRef}
        type="file"
        accept={accept}
        className="hidden"
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) handleFile(f);
        }}
      />

      {error && (
        <p className="text-xs text-destructive mt-2">{error}</p>
      )}
    </div>
  );
}
