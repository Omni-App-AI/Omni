"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Check, Loader2, Plus, X } from "lucide-react";
import { createClient } from "@/lib/supabase/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Select } from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { ImageUpload } from "./ImageUpload";
import { CATEGORIES } from "@/lib/constants";
import type { Extension } from "@/lib/supabase/types";

interface ExtensionSettingsProps {
  extension: Extension;
}

async function uploadImage(
  extensionId: string,
  path: string,
  file: File,
): Promise<string> {
  const supabase = createClient();
  const fullPath = `${extensionId}/${path}`;

  // Remove existing file first (ignore errors if not found)
  await supabase.storage.from("extension-images").remove([fullPath]);

  const { error } = await supabase.storage
    .from("extension-images")
    .upload(fullPath, file, { upsert: true });
  if (error) throw error;

  const { data } = supabase.storage
    .from("extension-images")
    .getPublicUrl(fullPath);
  return data.publicUrl;
}

export function ExtensionSettings({ extension }: ExtensionSettingsProps) {
  const router = useRouter();
  const [saving, setSaving] = useState(false);
  const [success, setSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tagInput, setTagInput] = useState("");

  const [form, setForm] = useState({
    name: extension.name,
    short_description: extension.short_description,
    description: extension.description,
    homepage: extension.homepage || "",
    repository: extension.repository || "",
    license: extension.license || "MIT",
    categories: [...extension.categories],
    tags: [...extension.tags],
    icon_url: extension.icon_url,
    banner_url: (extension as any).banner_url as string | null,
    screenshots: ((extension as any).screenshots as string[]) || [],
  });

  const updateForm = <K extends keyof typeof form>(key: K, value: (typeof form)[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
    setSuccess(false);
  };

  const toggleCategory = (catId: string) => {
    setForm((prev) => ({
      ...prev,
      categories: prev.categories.includes(catId)
        ? prev.categories.filter((c) => c !== catId)
        : prev.categories.length < 5
          ? [...prev.categories, catId]
          : prev.categories,
    }));
    setSuccess(false);
  };

  const addTag = () => {
    const tag = tagInput.trim().toLowerCase();
    if (tag && !form.tags.includes(tag) && form.tags.length < 10 && tag.length <= 30) {
      updateForm("tags", [...form.tags, tag]);
      setTagInput("");
    }
  };

  const removeTag = (tag: string) => {
    updateForm("tags", form.tags.filter((t) => t !== tag));
  };

  const handleScreenshotUpload = async (file: File) => {
    if (form.screenshots.length >= 5) return;
    const ext = file.name.split(".").pop() || "png";
    const url = await uploadImage(
      extension.id,
      `screenshots/${form.screenshots.length}.${ext}`,
      file,
    );
    updateForm("screenshots", [...form.screenshots, url]);
  };

  const removeScreenshot = (index: number) => {
    updateForm(
      "screenshots",
      form.screenshots.filter((_, i) => i !== index),
    );
  };

  const handleSave = async () => {
    setError(null);
    setSuccess(false);
    setSaving(true);

    try {
      const res = await fetch(`/api/v1/extensions/${extension.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: form.name,
          short_description: form.short_description,
          description: form.description,
          homepage: form.homepage || null,
          repository: form.repository || null,
          license: form.license,
          categories: form.categories,
          tags: form.tags,
          icon_url: form.icon_url,
          banner_url: form.banner_url,
          screenshots: form.screenshots,
        }),
      });

      if (!res.ok) {
        const data = await res.json();
        throw new Error(data.error || "Failed to save");
      }

      setSuccess(true);
      router.refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save");
    }
    setSaving(false);
  };

  return (
    <div className="space-y-8">
      {/* Images */}
      <div>
        <h3 className="text-sm font-semibold mb-4 text-muted-foreground uppercase tracking-widest">
          Images
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <ImageUpload
            label="Icon"
            currentUrl={form.icon_url}
            onUpload={async (file) => {
              const ext = file.name.split(".").pop() || "png";
              const url = await uploadImage(extension.id, `icon.${ext}`, file);
              updateForm("icon_url", url);
              return url;
            }}
            onRemove={() => updateForm("icon_url", null)}
            maxSizeKB={512}
            recommendedSize="256x256px"
            aspectRatio="1/1"
          />
          <ImageUpload
            label="Banner"
            currentUrl={form.banner_url}
            onUpload={async (file) => {
              const ext = file.name.split(".").pop() || "png";
              const url = await uploadImage(extension.id, `banner.${ext}`, file);
              updateForm("banner_url", url);
              return url;
            }}
            onRemove={() => updateForm("banner_url", null)}
            maxSizeKB={2048}
            recommendedSize="1280x640px (2:1)"
            aspectRatio="2/1"
          />
        </div>

        {/* Screenshots */}
        <div className="mt-6">
          <label className="text-[13px] font-medium mb-1.5 block">
            Screenshots ({form.screenshots.length}/5)
          </label>
          <p className="text-xs text-muted-foreground/60 mb-3">
            Recommended: 1280x800px (16:10). Max 2MB each. PNG, JPG, or WebP.
          </p>
          <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
            {form.screenshots.map((url, i) => (
              <div key={i} className="relative group rounded-lg overflow-hidden border border-border/50">
                <img
                  src={url}
                  alt={`Screenshot ${i + 1}`}
                  className="w-full aspect-[16/10] object-cover"
                />
                <button
                  type="button"
                  onClick={() => removeScreenshot(i)}
                  className="absolute top-1.5 right-1.5 h-6 w-6 flex items-center justify-center rounded-full bg-black/60 text-white opacity-0 group-hover:opacity-100 transition-opacity"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              </div>
            ))}
            {form.screenshots.length < 5 && (
              <label className="border-2 border-dashed border-border/50 rounded-lg aspect-[16/10] flex flex-col items-center justify-center cursor-pointer hover:border-primary/30 transition-colors">
                <Plus className="h-5 w-5 text-muted-foreground/40 mb-1" />
                <span className="text-xs text-muted-foreground">Add screenshot</span>
                <input
                  type="file"
                  accept="image/png,image/jpeg,image/webp"
                  className="hidden"
                  onChange={async (e) => {
                    const f = e.target.files?.[0];
                    if (f) {
                      if (f.size > 2 * 1024 * 1024) {
                        setError("Screenshot must be under 2MB");
                        return;
                      }
                      try {
                        await handleScreenshotUpload(f);
                      } catch (err) {
                        setError(err instanceof Error ? err.message : "Upload failed");
                      }
                    }
                  }}
                />
              </label>
            )}
          </div>
        </div>
      </div>

      {/* Basic Info */}
      <div>
        <h3 className="text-sm font-semibold mb-4 text-muted-foreground uppercase tracking-widest">
          Basic Info
        </h3>
        <div className="space-y-5">
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">Name</label>
            <Input
              value={form.name}
              onChange={(e) => updateForm("name", e.target.value)}
              maxLength={100}
            />
          </div>
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">Short Description</label>
            <Input
              value={form.short_description}
              onChange={(e) => updateForm("short_description", e.target.value)}
              maxLength={160}
            />
            <p className="text-xs text-muted-foreground/60 mt-1 font-mono">
              {form.short_description.length}/160
            </p>
          </div>
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">Description</label>
            <Textarea
              value={form.description}
              onChange={(e) => updateForm("description", e.target.value)}
              rows={6}
            />
          </div>
        </div>
      </div>

      {/* Categories */}
      <div>
        <h3 className="text-sm font-semibold mb-4 text-muted-foreground uppercase tracking-widest">
          Categories
        </h3>
        <p className="text-xs text-muted-foreground/60 mb-3">Select up to 5.</p>
        <div className="flex flex-wrap gap-2">
          {CATEGORIES.map((cat) => (
            <Badge
              key={cat.id}
              variant={form.categories.includes(cat.id) ? "default" : "outline"}
              className="cursor-pointer"
              onClick={() => toggleCategory(cat.id)}
            >
              {cat.name}
            </Badge>
          ))}
        </div>
      </div>

      {/* Tags */}
      <div>
        <h3 className="text-sm font-semibold mb-4 text-muted-foreground uppercase tracking-widest">
          Tags
        </h3>
        <div className="flex flex-wrap gap-2 mb-3">
          {form.tags.map((tag) => (
            <Badge key={tag} variant="secondary" className="gap-1">
              {tag}
              <button
                type="button"
                onClick={() => removeTag(tag)}
                className="text-muted-foreground hover:text-foreground"
              >
                <X className="h-3 w-3" />
              </button>
            </Badge>
          ))}
        </div>
        {form.tags.length < 10 && (
          <div className="flex gap-2">
            <Input
              placeholder="Add a tag..."
              value={tagInput}
              onChange={(e) => setTagInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addTag();
                }
              }}
              maxLength={30}
              className="max-w-[200px]"
            />
            <Button type="button" variant="outline" size="sm" onClick={addTag}>
              Add
            </Button>
          </div>
        )}
      </div>

      {/* Links */}
      <div>
        <h3 className="text-sm font-semibold mb-4 text-muted-foreground uppercase tracking-widest">
          Links
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">Homepage URL</label>
            <Input
              type="url"
              placeholder="https://..."
              value={form.homepage}
              onChange={(e) => updateForm("homepage", e.target.value)}
            />
          </div>
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">Repository URL</label>
            <Input
              type="url"
              placeholder="https://github.com/..."
              value={form.repository}
              onChange={(e) => updateForm("repository", e.target.value)}
            />
          </div>
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">License</label>
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
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-3 pt-2">
        <Button onClick={handleSave} disabled={saving} size="sm" className="gap-2">
          {saving ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Saving...
            </>
          ) : (
            "Save Changes"
          )}
        </Button>

        {success && (
          <span className="flex items-center gap-1 text-sm text-success">
            <Check className="h-4 w-4" />
            Saved
          </span>
        )}

        {error && (
          <span className="text-sm text-destructive">{error}</span>
        )}
      </div>
    </div>
  );
}
