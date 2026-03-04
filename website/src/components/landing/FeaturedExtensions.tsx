import { createClient } from "@/lib/supabase/server";
import { ExtensionCard } from "@/components/marketplace/ExtensionCard";
import type { ExtensionWithPublisher } from "@/lib/supabase/types";

export async function FeaturedExtensions() {
  const supabase = await createClient();

  const { data: extensions } = await supabase
    .from("extensions")
    .select("*, publisher:profiles(*)")
    .eq("featured", true)
    .eq("published", true)
    .order("total_downloads", { ascending: false })
    .limit(6);
  const exts = extensions as ExtensionWithPublisher[] | null;

  if (!exts || exts.length === 0) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {Array.from({ length: 6 }).map((_, i) => (
          <div
            key={i}
            className="rounded-lg border border-border bg-card p-5 animate-pulse"
          >
            <div className="space-y-2 mb-4">
              <div className="h-4 w-28 rounded bg-secondary" />
              <div className="h-3 w-20 rounded bg-secondary" />
            </div>
            <div className="space-y-2">
              <div className="h-3 w-full rounded bg-secondary" />
              <div className="h-3 w-3/4 rounded bg-secondary" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {exts.map((ext) => (
        <ExtensionCard key={ext.id} extension={ext} />
      ))}
    </div>
  );
}
