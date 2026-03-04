"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { PostCard } from "./PostCard";
import { POST_SORT_OPTIONS } from "@/lib/constants";
import type { ForumPostFull } from "@/lib/supabase/types";

interface PostListProps {
  posts: ForumPostFull[];
  total: number;
  page: number;
  pages: number;
  basePath: string;
}

export function PostList({ posts, total, page, pages, basePath }: PostListProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const currentSort = searchParams.get("sort") || "newest";

  const handleSort = (sort: string) => {
    const params = new URLSearchParams(searchParams.toString());
    params.set("sort", sort);
    params.delete("page");
    router.push(`${basePath}?${params.toString()}`);
  };

  const handlePage = (newPage: number) => {
    const params = new URLSearchParams(searchParams.toString());
    params.set("page", String(newPage));
    router.push(`${basePath}?${params.toString()}`);
  };

  return (
    <div>
      {/* Sort bar */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border/50">
        <span className="text-xs text-muted-foreground">
          {total} {total === 1 ? "post" : "posts"}
        </span>
        <div className="flex gap-1">
          {POST_SORT_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              onClick={() => handleSort(opt.value)}
              className={`px-2.5 py-1 text-xs rounded-md transition-colors ${
                currentSort === opt.value
                  ? "bg-primary/10 text-primary font-medium"
                  : "text-muted-foreground hover:text-foreground hover:bg-secondary/50"
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      {/* Posts */}
      {posts.length === 0 ? (
        <div className="p-8 text-center text-muted-foreground text-sm">
          No posts yet. Be the first to start a discussion.
        </div>
      ) : (
        <div>
          {posts.map((post) => (
            <PostCard key={post.id} post={post} />
          ))}
        </div>
      )}

      {/* Pagination */}
      {pages > 1 && (
        <div className="flex items-center justify-center gap-2 p-4 border-t border-border/50">
          <button
            onClick={() => handlePage(page - 1)}
            disabled={page <= 1}
            className="px-3 py-1.5 text-xs rounded-md border border-border/50 text-muted-foreground hover:text-foreground disabled:opacity-30 disabled:cursor-not-allowed"
          >
            Previous
          </button>
          <span className="text-xs text-muted-foreground">
            Page {page} of {pages}
          </span>
          <button
            onClick={() => handlePage(page + 1)}
            disabled={page >= pages}
            className="px-3 py-1.5 text-xs rounded-md border border-border/50 text-muted-foreground hover:text-foreground disabled:opacity-30 disabled:cursor-not-allowed"
          >
            Next
          </button>
        </div>
      )}
    </div>
  );
}
