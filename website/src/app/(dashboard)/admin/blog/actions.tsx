"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { Edit2, Trash2, Eye, EyeOff, Star, StarOff } from "lucide-react";
import { Button } from "@/components/ui/button";

interface BlogPostActionsProps {
  postId: string;
  published: boolean;
  featured: boolean;
}

export function BlogPostActions({ postId, published, featured }: BlogPostActionsProps) {
  const router = useRouter();
  const [loading, setLoading] = useState(false);

  const handleTogglePublish = async () => {
    setLoading(true);
    await fetch(`/api/v1/blog/posts/${postId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ published: !published }),
    });
    router.refresh();
    setLoading(false);
  };

  const handleToggleFeatured = async () => {
    setLoading(true);
    await fetch(`/api/v1/blog/posts/${postId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ featured: !featured }),
    });
    router.refresh();
    setLoading(false);
  };

  const handleDelete = async () => {
    if (!confirm("Are you sure you want to delete this post?")) return;
    setLoading(true);
    await fetch(`/api/v1/blog/posts/${postId}`, { method: "DELETE" });
    router.refresh();
    setLoading(false);
  };

  return (
    <div className="flex items-center gap-1.5 shrink-0">
      <Link href={`/admin/blog/${postId}/edit`}>
        <Button variant="ghost" size="icon" className="h-8 w-8" title="Edit">
          <Edit2 className="h-3.5 w-3.5" />
        </Button>
      </Link>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8"
        onClick={handleTogglePublish}
        disabled={loading}
        title={published ? "Unpublish" : "Publish"}
      >
        {published ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8"
        onClick={handleToggleFeatured}
        disabled={loading}
        title={featured ? "Unfeature" : "Feature"}
      >
        {featured ? (
          <StarOff className="h-3.5 w-3.5" />
        ) : (
          <Star className="h-3.5 w-3.5" />
        )}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8 text-destructive hover:text-destructive"
        onClick={handleDelete}
        disabled={loading}
        title="Delete"
      >
        <Trash2 className="h-3.5 w-3.5" />
      </Button>
    </div>
  );
}
