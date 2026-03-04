import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { createServiceClient } from "@/lib/supabase/server";
import { BlogViewCounter } from "./view-counter";
import { BlogContent } from "./blog-content";

type Props = {
  params: Promise<{ slug: string }>;
};

async function getPost(slug: string) {
  const supabase = createServiceClient();
  const { data: post } = await supabase
    .from("blog_posts")
    .select("*, author:profiles(id, username, display_name, avatar_url)")
    .eq("slug", slug)
    .eq("published", true)
    .single();

  return post as any;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  const post = await getPost(slug);

  if (!post) {
    return { title: "Post Not Found" };
  }

  const title = post.meta_title || post.title;
  const description = post.meta_description || post.excerpt || "";
  const ogImage = post.og_image_url || post.cover_image_url;
  const canonical = post.canonical_url || `/blog/${slug}`;

  return {
    title: `${title} — Omni Blog`,
    description,
    openGraph: {
      title,
      description,
      url: `/blog/${slug}`,
      type: "article",
      publishedTime: post.published_at || undefined,
      authors: post.author?.display_name ? [post.author.display_name] : undefined,
      tags: post.tags || undefined,
      ...(ogImage ? { images: [{ url: ogImage }] } : {}),
    },
    twitter: {
      card: ogImage ? "summary_large_image" : "summary",
      title,
      description,
      ...(ogImage ? { images: [ogImage] } : {}),
    },
    alternates: { canonical },
  };
}

function formatDate(dateStr: string) {
  return new Date(dateStr).toLocaleDateString("en-US", {
    month: "long",
    day: "numeric",
    year: "numeric",
  });
}

export default async function BlogPostPage({ params }: Props) {
  const { slug } = await params;
  const post = await getPost(slug);

  if (!post) {
    notFound();
  }

  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "Article",
    headline: post.title,
    description: post.meta_description || post.excerpt || "",
    image: post.og_image_url || post.cover_image_url || undefined,
    datePublished: post.published_at,
    dateModified: post.updated_at,
    author: {
      "@type": "Person",
      name: post.author?.display_name || "Omni Team",
    },
    publisher: {
      "@type": "Organization",
      name: "Omni",
      url: "https://www.omniapp.org",
    },
    mainEntityOfPage: {
      "@type": "WebPage",
      "@id": `https://www.omniapp.org/blog/${slug}`,
    },
  };

  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <BlogViewCounter slug={slug} />

      <article className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
        {/* Back link */}
        <Link
          href="/blog"
          className="inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors mb-10"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
          Back to Blog
        </Link>

        {/* Header */}
        <header className="mb-10">
          <div className="flex items-center gap-3 mb-4">
            <span className="text-xs font-mono text-primary">{post.category}</span>
            <span className="text-xs text-muted-foreground/40">|</span>
            <span className="text-xs text-muted-foreground">
              {post.published_at ? formatDate(post.published_at) : ""}
            </span>
            <span className="text-xs text-muted-foreground/40">|</span>
            <span className="text-xs text-muted-foreground">{post.read_time_minutes} min read</span>
          </div>
          <h1 className="text-3xl md:text-4xl font-bold tracking-tight leading-tight">
            {post.title}
          </h1>
          {post.excerpt && (
            <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
              {post.excerpt}
            </p>
          )}

          {/* Author */}
          {post.author && (
            <div className="flex items-center gap-3 mt-6 pt-6 border-t border-border/50">
              {post.author.avatar_url && (
                <img
                  src={post.author.avatar_url}
                  alt={post.author.display_name}
                  className="h-8 w-8 rounded-full"
                />
              )}
              <div>
                <p className="text-sm font-medium">{post.author.display_name}</p>
                <p className="text-xs text-muted-foreground">@{post.author.username}</p>
              </div>
            </div>
          )}
        </header>

        {/* Cover image */}
        {post.cover_image_url && (
          <div className="mb-10 rounded-lg overflow-hidden border border-border/50">
            <img
              src={post.cover_image_url}
              alt={post.title}
              className="w-full"
            />
          </div>
        )}

        {/* Body */}
        <BlogContent body={post.body} />

        {/* Tags */}
        {post.tags && post.tags.length > 0 && (
          <div className="mt-12 pt-6 border-t border-border/50">
            <div className="flex flex-wrap gap-2">
              {post.tags.map((tag: string) => (
                <span
                  key={tag}
                  className="inline-flex items-center px-2.5 py-0.5 rounded-md text-xs font-mono bg-secondary text-muted-foreground"
                >
                  {tag}
                </span>
              ))}
            </div>
          </div>
        )}
      </article>
    </>
  );
}
