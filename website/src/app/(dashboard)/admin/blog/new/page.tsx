import { redirect } from "next/navigation";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { BlogPostForm } from "@/components/admin/BlogPostForm";

export const metadata = {
  title: "New Blog Post",
  description: "Create a new blog post.",
};

export default async function NewBlogPostPage() {
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) {
    redirect("/login?redirect=/admin/blog/new");
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

  return (
    <div>
      <section className="relative overflow-hidden border-b border-border/50">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative px-8 lg:px-12 pt-10 pb-8">
          <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
            Blog
          </p>
          <h1 className="text-3xl font-bold tracking-tight">New Post</h1>
          <p className="text-muted-foreground mt-1">
            Write and publish a new blog article.
          </p>
        </div>
      </section>

      <section className="px-8 lg:px-12 py-8 max-w-3xl">
        <BlogPostForm mode="create" />
      </section>
    </div>
  );
}
