import type { Metadata } from "next";
import { createClient, createServiceClient } from "@/lib/supabase/server";
import { DashboardSidebar } from "@/components/layout/DashboardSidebar";

export const metadata: Metadata = {
  robots: { index: false, follow: false },
};

export default async function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  let isModerator = false;

  try {
    const supabase = await createClient();
    const { data: { user } } = await supabase.auth.getUser();

    if (user) {
      const service = createServiceClient();
      const { data: profile } = await service
        .from("profiles")
        .select("is_moderator")
        .eq("id", user.id)
        .single();

      isModerator = !!(profile as any)?.is_moderator;
    }
  } catch { /* fallback to false */ }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)]">
      <DashboardSidebar isModerator={isModerator} />
      <div className="flex-1 overflow-x-hidden">
        {children}
      </div>
    </div>
  );
}
