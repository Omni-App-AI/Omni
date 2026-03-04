"use client";

import { useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { slugify } from "@/lib/utils";
import { Eye, EyeOff, Save, Send } from "lucide-react";
import ReactMarkdown from "react-markdown";

const CATEGORIES = [
  "Announcement",
  "Engineering",
  "Security",
  "Tutorial",
  "Release",
  "Community",
  "General",
];

interface BlogPostFormProps {
  mode: "create" | "edit";
  initialData?: {
    id: string;
    title: string;
    slug: string;
    body: string;
    excerpt: string;
    category: string;
    tags: string[];
    cover_image_url: string;
    meta_title: string;
    meta_description: string;
    og_image_url: string;
    canonical_url: string;
    published: boolean;
    featured: boolean;
  };
}

export function BlogPostForm({ mode, initialData }: BlogPostFormProps) {
  const router = useRouter();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [showPreview, setShowPreview] = useState(false);
  const [showSeo, setShowSeo] = useState(false);

  const [title, setTitle] = useState(initialData?.title || "");
  const [slug, setSlug] = useState(initialData?.slug || "");
  const [body, setBody] = useState(initialData?.body || "");
  const [excerpt, setExcerpt] = useState(initialData?.excerpt || "");
  const [category, setCategory] = useState(initialData?.category || "General");
  const [tagsStr, setTagsStr] = useState(initialData?.tags?.join(", ") || "");
  const [coverImageUrl, setCoverImageUrl] = useState(initialData?.cover_image_url || "");
  const [metaTitle, setMetaTitle] = useState(initialData?.meta_title || "");
  const [metaDescription, setMetaDescription] = useState(initialData?.meta_description || "");
  const [ogImageUrl, setOgImageUrl] = useState(initialData?.og_image_url || "");
  const [canonicalUrl, setCanonicalUrl] = useState(initialData?.canonical_url || "");
  const [featured, setFeatured] = useState(initialData?.featured || false);

  const handleTitleChange = useCallback(
    (value: string) => {
      setTitle(value);
      if (mode === "create" || slug === slugify(initialData?.title || "")) {
        setSlug(slugify(value));
      }
    },
    [mode, slug, initialData?.title],
  );

  const handleSubmit = async (publish: boolean) => {
    if (!title.trim() || !body.trim()) {
      setError("Title and body are required");
      return;
    }

    setSaving(true);
    setError("");

    const tags = tagsStr
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean);

    const payload = {
      title: title.trim(),
      slug: slug.trim() || slugify(title),
      body,
      excerpt: excerpt.trim() || undefined,
      category,
      tags,
      cover_image_url: coverImageUrl.trim() || undefined,
      meta_title: metaTitle.trim() || undefined,
      meta_description: metaDescription.trim() || undefined,
      og_image_url: ogImageUrl.trim() || undefined,
      canonical_url: canonicalUrl.trim() || undefined,
      published: publish,
      featured,
    };

    try {
      const url =
        mode === "edit"
          ? `/api/v1/blog/posts/${initialData!.id}`
          : "/api/v1/blog/posts";
      const method = mode === "edit" ? "PUT" : "POST";

      const res = await fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });

      const data = await res.json();

      if (!res.ok) {
        setError(data.error || "Failed to save post");
        setSaving(false);
        return;
      }

      router.push("/admin/blog");
      router.refresh();
    } catch {
      setError("Network error. Please try again.");
      setSaving(false);
    }
  };

  const inputClass =
    "w-full bg-secondary/50 border border-border/50 rounded-md px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-2 focus:ring-primary/30 focus:border-primary/50 transition-all";

  return (
    <div className="space-y-6">
      {error && (
        <div className="bg-destructive/10 border border-destructive/30 text-destructive text-sm rounded-md px-4 py-3">
          {error}
        </div>
      )}

      {/* Title */}
      <div>
        <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
          Title
        </label>
        <input
          type="text"
          value={title}
          onChange={(e) => handleTitleChange(e.target.value)}
          placeholder="Post title"
          className={inputClass}
        />
      </div>

      {/* Slug */}
      <div>
        <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
          Slug
        </label>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground/50">/blog/</span>
          <input
            type="text"
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
            placeholder="post-slug"
            className={inputClass}
          />
        </div>
      </div>

      {/* Excerpt */}
      <div>
        <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
          Excerpt
        </label>
        <textarea
          value={excerpt}
          onChange={(e) => setExcerpt(e.target.value)}
          placeholder="Short summary for listing cards..."
          rows={2}
          className={inputClass}
        />
      </div>

      {/* Category + Tags */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
            Category
          </label>
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            className={inputClass}
          >
            {CATEGORIES.map((cat) => (
              <option key={cat} value={cat}>
                {cat}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
            Tags
          </label>
          <input
            type="text"
            value={tagsStr}
            onChange={(e) => setTagsStr(e.target.value)}
            placeholder="ai, extensions, security"
            className={inputClass}
          />
          <p className="text-[11px] text-muted-foreground/40 mt-1">Comma-separated</p>
        </div>
      </div>

      {/* Cover image */}
      <div>
        <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
          Cover Image URL
        </label>
        <input
          type="url"
          value={coverImageUrl}
          onChange={(e) => setCoverImageUrl(e.target.value)}
          placeholder="https://..."
          className={inputClass}
        />
      </div>

      {/* Featured */}
      <label className="flex items-center gap-3 cursor-pointer">
        <input
          type="checkbox"
          checked={featured}
          onChange={(e) => setFeatured(e.target.checked)}
          className="h-4 w-4 rounded border-border bg-secondary text-primary focus:ring-primary/30"
        />
        <span className="text-sm">Featured post (pinned to top of blog)</span>
      </label>

      {/* Body with preview toggle */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-xs font-mono uppercase tracking-widest text-muted-foreground/60">
            Body (Markdown)
          </label>
          <button
            type="button"
            onClick={() => setShowPreview(!showPreview)}
            className="inline-flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            {showPreview ? (
              <>
                <EyeOff className="h-3.5 w-3.5" /> Editor
              </>
            ) : (
              <>
                <Eye className="h-3.5 w-3.5" /> Preview
              </>
            )}
          </button>
        </div>
        {showPreview ? (
          <div className="min-h-[300px] bg-secondary/30 border border-border/50 rounded-md p-6 prose prose-invert prose-headings:font-bold prose-a:text-primary prose-code:text-primary/80 prose-code:bg-secondary prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded prose-code:before:content-[''] prose-code:after:content-[''] prose-pre:bg-secondary prose-pre:border prose-pre:border-border/50 max-w-none">
            <ReactMarkdown>{body || "*Nothing to preview*"}</ReactMarkdown>
          </div>
        ) : (
          <textarea
            value={body}
            onChange={(e) => setBody(e.target.value)}
            placeholder="Write your blog post in Markdown..."
            rows={20}
            className={`${inputClass} font-mono text-[13px] leading-relaxed`}
          />
        )}
      </div>

      {/* SEO section */}
      <div className="border border-border/50 rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => setShowSeo(!showSeo)}
          className="w-full flex items-center justify-between px-4 py-3 text-sm font-medium hover:bg-secondary/30 transition-colors"
        >
          <span>SEO & Meta</span>
          <span className="text-xs text-muted-foreground">{showSeo ? "Hide" : "Show"}</span>
        </button>
        {showSeo && (
          <div className="px-4 pb-4 space-y-4 border-t border-border/50 pt-4">
            <div>
              <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
                Meta Title
              </label>
              <input
                type="text"
                value={metaTitle}
                onChange={(e) => setMetaTitle(e.target.value)}
                placeholder={title || "Falls back to post title"}
                className={inputClass}
              />
              <p className="text-[11px] text-muted-foreground/40 mt-1">
                {(metaTitle || title).length}/60 characters
              </p>
            </div>
            <div>
              <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
                Meta Description
              </label>
              <textarea
                value={metaDescription}
                onChange={(e) => setMetaDescription(e.target.value)}
                placeholder={excerpt || "Falls back to excerpt"}
                rows={2}
                className={inputClass}
              />
              <p className="text-[11px] text-muted-foreground/40 mt-1">
                {(metaDescription || excerpt).length}/160 characters
              </p>
            </div>
            <div>
              <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
                OG Image URL
              </label>
              <input
                type="url"
                value={ogImageUrl}
                onChange={(e) => setOgImageUrl(e.target.value)}
                placeholder={coverImageUrl || "Falls back to cover image"}
                className={inputClass}
              />
            </div>
            <div>
              <label className="block text-xs font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
                Canonical URL
              </label>
              <input
                type="url"
                value={canonicalUrl}
                onChange={(e) => setCanonicalUrl(e.target.value)}
                placeholder={`/blog/${slug}`}
                className={inputClass}
              />
            </div>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-3 pt-4 border-t border-border/50">
        <Button
          onClick={() => handleSubmit(false)}
          disabled={saving}
          variant="outline"
          className="gap-2"
        >
          <Save className="h-4 w-4" />
          {saving ? "Saving..." : "Save as Draft"}
        </Button>
        <Button
          onClick={() => handleSubmit(true)}
          disabled={saving}
          className="gap-2"
        >
          <Send className="h-4 w-4" />
          {saving ? "Publishing..." : "Publish"}
        </Button>
      </div>
    </div>
  );
}
