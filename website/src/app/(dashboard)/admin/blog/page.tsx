import Link from "next/link";
import { redirect } from "next/navigation";
import { Plus, Edit2, Trash2, Eye, EyeOff, Star } from "lucide-react";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { BlogPostActions } from "./actions";

export const metadata = {
  title: "Blog Posts",
  description: "Manage blog posts.",
};

function formatDate(dateStr: string | null) {
  if (!dateStr) return "—";
  return new Date(dateStr).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

export default async function AdminBlogPage() {
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) {
    redirect("/login?redirect=/admin/blog");
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

  const { data: posts } = await service
    .from("blog_posts")
    .select("id, slug, title, category, published, featured, view_count, published_at, created_at, updated_at, author:profiles(display_name)")
    .order("created_at", { ascending: false });

  const allPosts = (posts as any[]) || [];

  return (
    <div>
      {/* Header */}
      <section className="relative overflow-hidden border-b border-border/50">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative px-8 lg:px-12 pt-10 pb-8">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
                Blog
              </p>
              <h1 className="text-3xl font-bold tracking-tight">Blog Posts</h1>
              <p className="text-muted-foreground mt-1">
                Create, edit, and manage blog articles.
              </p>
            </div>
            <Link href="/admin/blog/new">
              <Button className="gap-2">
                <Plus className="h-4 w-4" />
                New Post
              </Button>
            </Link>
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="border-b border-border/50">
        <div className="px-8 lg:px-12 py-6">
          <div className="grid grid-cols-3 gap-8">
            <div>
              <p className="text-2xl font-bold">{allPosts.length}</p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Total Posts
              </p>
            </div>
            <div>
              <p className="text-2xl font-bold">{allPosts.filter((p) => p.published).length}</p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Published
              </p>
            </div>
            <div>
              <p className="text-2xl font-bold">{allPosts.reduce((sum, p) => sum + (p.view_count || 0), 0)}</p>
              <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mt-1">
                Total Views
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Posts list */}
      <section className="px-8 lg:px-12 py-8">
        {allPosts.length === 0 ? (
          <div className="border border-dashed border-border/50 rounded-lg p-12 text-center">
            <p className="text-sm text-muted-foreground mb-1">No blog posts yet.</p>
            <p className="text-xs text-muted-foreground/60 mb-6">
              Create your first blog post to get started.
            </p>
            <Link href="/admin/blog/new">
              <Button size="sm">Create Post</Button>
            </Link>
          </div>
        ) : (
          <div className="border border-border/50 rounded-lg divide-y divide-border/50">
            {allPosts.map((post) => (
              <div
                key={post.id}
                className="flex items-center gap-4 px-5 py-4 hover:bg-secondary/20 transition-colors"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <h3 className="text-sm font-medium truncate">{post.title}</h3>
                    {post.featured && (
                      <Star className="h-3.5 w-3.5 fill-warning text-warning shrink-0" />
                    )}
                  </div>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    <span
                      className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono ${
                        post.published
                          ? "bg-emerald-500/10 text-emerald-400"
                          : "bg-muted text-muted-foreground"
                      }`}
                    >
                      {post.published ? "Published" : "Draft"}
                    </span>
                    <span className="text-muted-foreground/30">|</span>
                    <span>{post.category}</span>
                    <span className="text-muted-foreground/30">|</span>
                    <span>{formatDate(post.published_at || post.created_at)}</span>
                    <span className="text-muted-foreground/30">|</span>
                    <span className="flex items-center gap-1">
                      <Eye className="h-3 w-3" /> {post.view_count || 0}
                    </span>
                  </div>
                </div>
                <BlogPostActions postId={post.id} published={post.published} featured={post.featured} />
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
