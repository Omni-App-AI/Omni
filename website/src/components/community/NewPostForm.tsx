"use client";

import { useState, useRef } from "react";
import { useRouter } from "next/navigation";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Select } from "@/components/ui/select";
import { FORUM_CATEGORIES } from "@/lib/constants";
import { HoneypotFields } from "@/components/ui/HoneypotFields";
import { Turnstile } from "@/components/ui/Turnstile";

interface NewPostFormProps {
  defaultCategoryId?: string;
  defaultExtensionId?: string;
  extensions?: { id: string; name: string }[];
  isModerator?: boolean;
}

export function NewPostForm({ defaultCategoryId, defaultExtensionId, extensions, isModerator }: NewPostFormProps) {
  const router = useRouter();
  const formRef = useRef<HTMLFormElement>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [categoryId, setCategoryId] = useState(defaultCategoryId || "");
  const [extensionId, setExtensionId] = useState(defaultExtensionId || "");
  const [postType, setPostType] = useState<"category" | "extension">(defaultExtensionId ? "extension" : "category");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [turnstileToken, setTurnstileToken] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !body.trim()) return;

    setSubmitting(true);
    setError(null);

    // Collect honeypot fields
    const formData = new FormData(formRef.current!);

    try {
      const res = await fetch("/api/v1/community/posts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title: title.trim(),
          body: body.trim(),
          category_id: postType === "category" ? categoryId : undefined,
          extension_id: postType === "extension" ? extensionId : undefined,
          turnstile_token: turnstileToken || undefined,
          hp_website: formData.get("hp_website") || "",
          hp_timestamp: formData.get("hp_timestamp") || "",
          hp_token: formData.get("hp_token") || "",
        }),
      });

      if (!res.ok) {
        const data = await res.json();
        setError(data.error || "Failed to create post");
        return;
      }

      const data = await res.json();
      router.push(`/community/post/${data.post.id}`);
    } catch {
      setError("Failed to create post");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form ref={formRef} onSubmit={handleSubmit} className="space-y-5">
      <HoneypotFields />

      {/* Post type selection */}
      {!defaultExtensionId && (
        <div>
          <label className="text-[13px] font-medium mb-1.5 block">Post in</label>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setPostType("category")}
              className={`px-3 py-1.5 text-sm rounded-md border transition-colors ${
                postType === "category"
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border/50 text-muted-foreground hover:text-foreground"
              }`}
            >
              Community Forum
            </button>
            <button
              type="button"
              onClick={() => setPostType("extension")}
              className={`px-3 py-1.5 text-sm rounded-md border transition-colors ${
                postType === "extension"
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border/50 text-muted-foreground hover:text-foreground"
              }`}
            >
              Extension Discussion
            </button>
          </div>
        </div>
      )}

      {/* Category selector */}
      {postType === "category" && !defaultExtensionId && (
        <div>
          <label className="text-[13px] font-medium mb-1.5 block">Category</label>
          <Select value={categoryId} onChange={(e) => setCategoryId(e.target.value)}>
            <option value="">Select a category</option>
            {FORUM_CATEGORIES.filter((cat) => cat.id !== "announcements" || isModerator).map((cat) => (
              <option key={cat.id} value={cat.id}>
                {cat.name}
              </option>
            ))}
          </Select>
        </div>
      )}

      {/* Extension selector */}
      {postType === "extension" && !defaultExtensionId && extensions && (
        <div>
          <label className="text-[13px] font-medium mb-1.5 block">Extension</label>
          <Select value={extensionId} onChange={(e) => setExtensionId(e.target.value)}>
            <option value="">Select an extension</option>
            {extensions.map((ext) => (
              <option key={ext.id} value={ext.id}>
                {ext.name}
              </option>
            ))}
          </Select>
        </div>
      )}

      <div>
        <label className="text-[13px] font-medium mb-1.5 block">Title</label>
        <Input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="What's your question or topic?"
          required
          maxLength={200}
        />
      </div>

      <div>
        <label className="text-[13px] font-medium mb-1.5 block">Body</label>
        <Textarea
          value={body}
          onChange={(e) => setBody(e.target.value)}
          placeholder="Provide details, context, and any relevant information..."
          rows={10}
          required
          className="font-mono text-sm"
        />
        <p className="text-[10px] text-muted-foreground/60 mt-1">Markdown is supported.</p>
      </div>

      {/* Invisible Turnstile for newcomers — trust-gated server-side */}
      <Turnstile
        mode="invisible"
        onVerify={setTurnstileToken}
        onExpire={() => setTurnstileToken(null)}
      />

      {error && <p className="text-sm text-destructive">{error}</p>}

      <Button
        type="submit"
        disabled={submitting || !title.trim() || !body.trim() || (postType === "category" && !categoryId) || (postType === "extension" && !extensionId)}
        className="gap-2"
      >
        {submitting ? (
          <>
            <Loader2 className="h-4 w-4 animate-spin" />
            Creating...
          </>
        ) : (
          "Create Post"
        )}
      </Button>
    </form>
  );
}
