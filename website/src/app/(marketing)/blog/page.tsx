import type { Metadata } from "next";
import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { createServiceClient } from "@/lib/supabase/server";

export const metadata: Metadata = {
  title: "Blog — AI Agent News & Extension Updates",
  description:
    "Read the latest news, tutorials, and updates from the Omni team covering AI agent development, WASM extension building, platform security practices, and marketplace releases.",
  openGraph: {
    title: "Omni Blog — AI Agent News, Tutorials & Extension Updates",
    description:
      "Read the latest news, tutorials, and updates from the Omni team covering AI agent development, WASM extension building, platform security, and marketplace releases.",
    url: "/blog",
  },
  alternates: { canonical: "/blog" },
};

function formatDate(dateStr: string) {
  return new Date(dateStr).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

export default async function BlogPage() {
  const supabase = createServiceClient();

  const { data: posts } = await supabase
    .from("blog_posts")
    .select("id, slug, title, excerpt, category, tags, cover_image_url, featured, read_time_minutes, published_at, author:profiles(id, username, display_name, avatar_url)")
    .eq("published", true)
    .order("published_at", { ascending: false })
    .limit(50);

  const allPosts = (posts as any[]) || [];
  const featured = allPosts.find((p) => p.featured);
  const rest = allPosts.filter((p) => !p.featured);

  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="max-w-2xl mb-12">
        <p className="text-sm font-medium text-primary mb-3">Blog</p>
        <h1 className="text-3xl md:text-4xl font-bold tracking-tight">
          News and updates
        </h1>
        <p className="mt-4 text-muted-foreground leading-relaxed">
          Engineering insights, release announcements, and security deep dives from the Omni team.
        </p>
      </div>

      {/* Featured post */}
      {featured && (
        <Link href={`/blog/${featured.slug}`} className="block mb-10 group">
          <div className="border border-border/50 rounded-lg overflow-hidden hover:border-border transition-colors">
            {featured.cover_image_url && (
              <div className="aspect-[3/1] relative overflow-hidden">
                <img
                  src={featured.cover_image_url}
                  alt={featured.title}
                  className="w-full h-full object-cover"
                />
              </div>
            )}
            <div className="p-8 md:p-10">
              <div className="flex items-center gap-3 mb-4">
                <span className="text-xs font-mono text-primary">{featured.category}</span>
                <span className="text-xs text-muted-foreground/40">|</span>
                <span className="text-xs text-muted-foreground">{featured.published_at ? formatDate(featured.published_at) : ""}</span>
                <span className="text-xs text-muted-foreground/40">|</span>
                <span className="text-xs text-muted-foreground">{featured.read_time_minutes} min read</span>
              </div>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight group-hover:text-primary transition-colors">
                {featured.title}
              </h2>
              {featured.excerpt && (
                <p className="mt-4 text-muted-foreground leading-relaxed max-w-2xl">
                  {featured.excerpt}
                </p>
              )}
              <div className="mt-6 flex items-center text-sm text-primary font-medium">
                Read more
                <ArrowRight className="h-3.5 w-3.5 ml-1 group-hover:translate-x-1 transition-transform" />
              </div>
            </div>
          </div>
        </Link>
      )}

      {/* Post list */}
      {rest.length > 0 && (
        <div className="border-t border-border/50">
          {rest.map((post, i) => (
            <Link key={post.id} href={`/blog/${post.slug}`} className="group block">
              <div className={`py-8 flex flex-col md:flex-row md:items-baseline gap-4 md:gap-8 ${i < rest.length - 1 ? "border-b border-border/50" : ""}`}>
                <div className="flex items-center gap-3 md:w-48 shrink-0">
                  <span className="text-xs font-mono text-muted-foreground">{post.published_at ? formatDate(post.published_at) : ""}</span>
                  <span className="text-xs text-muted-foreground/40 hidden md:inline">|</span>
                  <span className="text-xs font-mono text-muted-foreground/60 hidden md:inline">{post.read_time_minutes} min</span>
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-baseline gap-3 mb-1">
                    <h3 className="font-medium text-[15px] group-hover:text-primary transition-colors">
                      {post.title}
                    </h3>
                    <span className="text-[11px] font-mono text-muted-foreground/50 shrink-0">{post.category}</span>
                  </div>
                  {post.excerpt && (
                    <p className="text-sm text-muted-foreground line-clamp-2">
                      {post.excerpt}
                    </p>
                  )}
                </div>
              </div>
            </Link>
          ))}
        </div>
      )}

      {allPosts.length === 0 && (
        <div className="border border-dashed border-border/50 rounded-lg p-12 text-center">
          <p className="text-sm text-muted-foreground">No blog posts yet. Check back soon.</p>
        </div>
      )}
    </div>
  );
}
