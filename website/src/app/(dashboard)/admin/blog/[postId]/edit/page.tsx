import { redirect, notFound } from "next/navigation";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { BlogPostForm } from "@/components/admin/BlogPostForm";

export const metadata = {
  title: "Edit Blog Post",
  description: "Edit an existing blog post.",
};

export default async function EditBlogPostPage({
  params,
}: {
  params: Promise<{ postId: string }>;
}) {
  const { postId } = await params;

  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) {
    redirect(`/login?redirect=/admin/blog/${postId}/edit`);
  }

  const service = createServiceClient();
  const { data: profile } = await service
    .from("profiles")
    .select("is_moderator")
    .eq("id", user.id)
    .single();

  if (!profile || !(profile as any).is_moderator) {
    redirect("/dashboard");
  }

  const { data: post } = await service
    .from("blog_posts")
    .select("*")
    .eq("id", postId)
    .single();

  if (!post) {
    notFound();
  }

  const p = post as any;

  return (
    <div>
      <section className="relative overflow-hidden border-b border-border/50">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative px-8 lg:px-12 pt-10 pb-8">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
            Blog
          </p>
          <h1 className="text-3xl font-bold tracking-tight">Edit Post</h1>
          <p className="text-muted-foreground mt-1">
            Update &ldquo;{p.title}&rdquo;
          </p>
        </div>
      </section>

      <section className="px-8 lg:px-12 py-8 max-w-3xl">
        <BlogPostForm
          mode="edit"
          initialData={{
            id: p.id,
            title: p.title,
            slug: p.slug,
            body: p.body,
            excerpt: p.excerpt || "",
            category: p.category,
            tags: p.tags || [],
            cover_image_url: p.cover_image_url || "",
            meta_title: p.meta_title || "",
            meta_description: p.meta_description || "",
            og_image_url: p.og_image_url || "",
            canonical_url: p.canonical_url || "",
            published: p.published,
            featured: p.featured,
          }}
        />
      </section>
    </div>
  );
}
