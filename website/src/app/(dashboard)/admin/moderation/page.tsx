import { createClient, createServiceClient } from "@/lib/supabase/server";
import { redirect } from "next/navigation";
import { ModerationDashboard } from "@/components/admin/ModerationDashboard";

export const metadata = {
  title: "Moderation Queue",
  description: "Extension moderation queue.",
};

export default async function ModerationPage() {
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) {
    redirect("/login?redirect=/admin/moderation");
  }

  // Verify moderator role server-side before rendering
  const service = createServiceClient();
  const { data: profile } = await service
    .from("profiles")
    .select("is_moderator")
    .eq("id", user.id)
    .single();

  if (!profile || !(profile as any).is_moderator) {
    redirect("/dashboard");
  }

  return <ModerationDashboard />;
}
