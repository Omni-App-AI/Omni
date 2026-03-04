"use client";

import { useEffect } from "react";

export function BlogViewCounter({ slug }: { slug: string }) {
  useEffect(() => {
    fetch(`/api/v1/blog/posts/${slug}/views`, { method: "POST" }).catch(() => {});
  }, [slug]);

  return null;
}
