import type { Metadata } from "next";
import { redirect } from "next/navigation";
import Link from "next/link";
import { ChevronRight } from "lucide-react";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { NewPostForm } from "@/components/community/NewPostForm";

export const metadata: Metadata = {
  title: "New Post — Community",
  description:
    "Create a new discussion post in the Omni AI agent community. Ask questions, share projects, request features, or discuss WASM extension development with other builders.",
  robots: { index: false, follow: false },
};

interface Props {
  searchParams: Promise<{ category?: string; extension?: string }>;
}

export default async function NewPostPage({ searchParams }: Props) {
  const supabase = await createClient();

  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) {
    redirect("/login?redirect=/community/new");
  }

  const { category, extension } = await searchParams;

  // Check if user is a moderator
  const service = createServiceClient();
  const { data: profileData } = await service
    .from("profiles")
    .select("is_moderator")
    .eq("id", user.id)
    .single();
  const isModerator = !!(profileData as any)?.is_moderator;

  // Fetch published extensions for the selector
  const { data: extensionsData } = await service
    .from("extensions")
    .select("id, name")
    .eq("published", true)
    .order("name", { ascending: true })
    .limit(200);

  const extensions = (extensionsData as { id: string; name: string }[] | null) || [];

  return (
    <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-10">
      {/* Breadcrumb */}
      <nav className="flex items-center gap-1 text-sm text-muted-foreground mb-6">
        <Link href="/community" className="hover:text-foreground">
          Community
        </Link>
        <ChevronRight className="h-3 w-3" />
        <span className="text-foreground">New Post</span>
      </nav>

      <h1 className="text-2xl font-bold mb-6">Create a New Post</h1>

      <div className="border border-border/50 rounded-lg bg-card/30 p-6">
        <NewPostForm
          defaultCategoryId={category}
          defaultExtensionId={extension}
          extensions={extensions}
          isModerator={isModerator}
        />
      </div>
    </div>
  );
}
