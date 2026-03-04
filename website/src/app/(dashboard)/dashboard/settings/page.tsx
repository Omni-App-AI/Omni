import type { Metadata } from "next";
import { createClient } from "@/lib/supabase/server";
import { PublisherProfile } from "@/components/dashboard/PublisherProfile";

export const metadata: Metadata = {
  title: "Settings",
  description:
    "Manage your Omni publisher profile and account preferences.",
};

export default async function SettingsPage() {
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) return null;

  const { data: profile } = await supabase
    .from("profiles")
    .select("*")
    .eq("id", user.id)
    .single();

  return (
    <div>
      {/* Header */}
      <section className="relative overflow-hidden border-b border-border/50">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative px-8 lg:px-12 pt-10 pb-8">
          <div>
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
              Account
            </p>
            <h1 className="text-3xl font-bold tracking-tight">
              Settings
            </h1>
            <p className="text-muted-foreground mt-1">
              Manage your profile and preferences.
            </p>
          </div>
        </div>
      </section>

      {/* Content */}
      <section className="px-8 lg:px-12 py-8">
        <div className="max-w-4xl">
          {profile && <PublisherProfile profile={profile} />}
        </div>
      </section>
    </div>
  );
}
