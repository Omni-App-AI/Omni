import { ExtensionCard } from "./ExtensionCard";
import type { ExtensionWithPublisher } from "@/lib/supabase/types";

interface ExtensionGridProps {
  extensions: ExtensionWithPublisher[];
  emptyMessage?: string;
}

export function ExtensionGrid({ extensions, emptyMessage = "No extensions found." }: ExtensionGridProps) {
  if (extensions.length === 0) {
    return (
      <div className="text-center py-16">
        <p className="text-muted-foreground">{emptyMessage}</p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {extensions.map((ext) => (
        <ExtensionCard key={ext.id} extension={ext} />
      ))}
    </div>
  );
}
