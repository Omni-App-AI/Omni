import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import { ArrowLeft } from "lucide-react";
import { createClient } from "@/lib/supabase/server";
import { Button } from "@/components/ui/button";
import { AnalyticsChart } from "@/components/dashboard/AnalyticsChart";

export const metadata: Metadata = {
  title: "Extension Analytics",
  description:
    "Download analytics and trends for your extension.",
};

export default async function AnalyticsPage({
  params,
}: {
  params: Promise<{ extensionId: string }>;
}) {
  const { extensionId } = await params;
  const supabase = await createClient();
  const { data: { user } } = await supabase.auth.getUser();

  if (!user) return null;

  const { data: extensionData } = await supabase
    .from("extensions")
    .select("id, name")
    .eq("id", extensionId)
    .eq("publisher_id", user.id)
    .single();

  const extension = extensionData as { id: string; name: string } | null;
  if (!extension) notFound();

  // Get download stats for the last 30 days
  const thirtyDaysAgo = new Date();
  thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

  const { data: statsData } = await supabase
    .from("download_stats")
    .select("date, count")
    .eq("extension_id", extensionId)
    .gte("date", thirtyDaysAgo.toISOString().split("T")[0])
    .order("date", { ascending: true });

  const stats = statsData as { date: string; count: number }[] | null;

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
                Analytics
              </p>
              <h1 className="text-3xl font-bold tracking-tight">
                {extension.name}
              </h1>
              <p className="text-muted-foreground mt-1">
                Download statistics for the last 30 days.
              </p>
            </div>
            <Link href={`/dashboard/extensions/${extensionId}`}>
              <Button variant="outline" size="sm" className="gap-2">
                <ArrowLeft className="h-4 w-4" />
                Back
              </Button>
            </Link>
          </div>
        </div>
      </section>

      {/* Chart */}
      <section className="px-8 lg:px-12 py-8">
        <h2 className="text-lg font-semibold mb-4">Downloads Over Time</h2>
        <div className="border border-border/50 rounded-lg p-6">
          <AnalyticsChart data={stats || []} />
        </div>
      </section>
    </div>
  );
}
