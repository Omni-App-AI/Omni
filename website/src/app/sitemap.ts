import type { MetadataRoute } from "next";
import { createServiceClient } from "@/lib/supabase/server";
import { FORUM_CATEGORIES } from "@/lib/constants";

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const baseUrl =
    process.env.NEXT_PUBLIC_APP_URL || "https://www.omniapp.org";
  const supabase = createServiceClient();

  /* ── Static pages ─────────────────────────────────────────── */
  const staticPages: MetadataRoute.Sitemap = [
    { url: baseUrl, lastModified: new Date(), changeFrequency: "daily", priority: 1.0 },
    { url: `${baseUrl}/extensions`, lastModified: new Date(), changeFrequency: "daily", priority: 0.9 },
    { url: `${baseUrl}/about`, changeFrequency: "monthly", priority: 0.7 },
    { url: `${baseUrl}/blog`, changeFrequency: "weekly", priority: 0.7 },
    { url: `${baseUrl}/download`, changeFrequency: "monthly", priority: 0.8 },
    { url: `${baseUrl}/privacy`, changeFrequency: "yearly", priority: 0.3 },
    { url: `${baseUrl}/security`, changeFrequency: "monthly", priority: 0.7 },
    { url: `${baseUrl}/terms`, changeFrequency: "yearly", priority: 0.3 },
    { url: `${baseUrl}/community`, changeFrequency: "daily", priority: 0.7 },
    { url: `${baseUrl}/donate`, changeFrequency: "monthly", priority: 0.6 },
  ];

  /* ── Docs pages ───────────────────────────────────────────── */
  const docsSlugs = [
    "", "getting-started", "configuration", "providers", "channels",
    "security", "tools", "hooks", "architecture", "sdk", "publishing",
    "building", "changelog",
  ];
  const docsPages: MetadataRoute.Sitemap = docsSlugs.map((slug) => ({
    url: `${baseUrl}/docs${slug ? `/${slug}` : ""}`,
    changeFrequency: "monthly" as const,
    priority: slug === "" || slug === "getting-started" || slug === "sdk" ? 0.8 : 0.7,
  }));

  /* ── Community categories ─────────────────────────────────── */
  const categoryPages: MetadataRoute.Sitemap = FORUM_CATEGORIES.map((cat) => ({
    url: `${baseUrl}/community/${cat.id}`,
    changeFrequency: "daily" as const,
    priority: 0.5,
  }));

  /* ── Published extensions ─────────────────────────────────── */
  const { data: extensionsRaw } = await supabase
    .from("extensions")
    .select("id, publisher_id, updated_at")
    .eq("published", true);

  const extensions = (extensionsRaw ?? []) as {
    id: string;
    publisher_id: string;
    updated_at: string;
  }[];

  const extensionPages: MetadataRoute.Sitemap = extensions.map((ext) => ({
    url: `${baseUrl}/extensions/${ext.id}`,
    lastModified: new Date(ext.updated_at),
    changeFrequency: "weekly" as const,
    priority: 0.8,
  }));

  /* ── Publisher profiles ───────────────────────────────────── */
  const publisherIds = [...new Set(extensions.map((e) => e.publisher_id))];

  let publisherPages: MetadataRoute.Sitemap = [];
  if (publisherIds.length > 0) {
    const { data: profilesRaw } = await supabase
      .from("profiles")
      .select("username, updated_at")
      .in("id", publisherIds);

    const profiles = (profilesRaw ?? []) as {
      username: string;
      updated_at: string;
    }[];

    publisherPages = profiles.map((p) => ({
      url: `${baseUrl}/publishers/${p.username}`,
      lastModified: new Date(p.updated_at),
      changeFrequency: "weekly" as const,
      priority: 0.6,
    }));
  }

  /* ── Community posts ──────────────────────────────────────── */
  const { data: postsRaw } = await supabase
    .from("forum_posts")
    .select("id, updated_at")
    .order("created_at", { ascending: false })
    .limit(1000);

  const posts = (postsRaw ?? []) as { id: string; updated_at: string }[];

  const postPages: MetadataRoute.Sitemap = posts.map((p) => ({
    url: `${baseUrl}/community/post/${p.id}`,
    lastModified: new Date(p.updated_at),
    changeFrequency: "weekly" as const,
    priority: 0.5,
  }));

  return [
    ...staticPages,
    ...docsPages,
    ...categoryPages,
    ...extensionPages,
    ...publisherPages,
    ...postPages,
  ];
}
