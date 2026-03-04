import type { Metadata } from "next";
import { Heart, Server, Code2, Shield, Users } from "lucide-react";
import { createServiceClient } from "@/lib/supabase/server";
import { DonateForm } from "@/components/donate/DonateForm";
import { ThankYouBanner } from "@/components/donate/ThankYouBanner";

export const metadata: Metadata = {
  title: "Donate — Support Open-Source AI Development",
  description:
    "Support the Omni AI agent platform with a one-time or monthly donation. Fund open-source development, infrastructure, and community growth. Donors earn a profile badge and can appear on our public supporters list.",
  openGraph: {
    title: "Donate to Omni — Support Open-Source AI Development",
    description:
      "Support the Omni AI agent platform with a one-time or monthly donation. Fund open-source development, infrastructure, and community growth.",
    url: "/donate",
  },
  alternates: { canonical: "/donate" },
};

function formatCents(cents: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(cents / 100);
}

const SUPPORT_ITEMS = [
  {
    icon: Server,
    title: "Infrastructure",
    desc: "Servers, CDN, storage, and the CI/CD pipelines that build and test every release.",
  },
  {
    icon: Code2,
    title: "Development",
    desc: "Full-time work on the Omni runtime, SDK, security pipeline, and 21 channel integrations.",
  },
  {
    icon: Shield,
    title: "Security Pipeline",
    desc: "The 4-layer scanning system that keeps every marketplace extension safe for users.",
  },
  {
    icon: Users,
    title: "Community",
    desc: "Forum hosting, documentation, and resources that help developers build great extensions.",
  },
];

export default async function DonatePage({
  searchParams,
}: {
  searchParams: Promise<{ thanks?: string }>;
}) {
  const params = await searchParams;
  const supabase = createServiceClient();

  // Fetch donation stats (SECURITY DEFINER function -- intentional RLS bypass for aggregates)
  const { data: statsRows } = await supabase.rpc("get_donation_stats" as any);

  const stats = (Array.isArray(statsRows) ? statsRows[0] : statsRows) as {
    total_cents: number;
    today_cents: number;
    month_cents: number;
    year_cents: number;
    total_count: number;
  } | null;

  // Fetch recent public donors
  const { data: recentDonorsRaw } = await supabase
    .from("donations")
    .select("donor_name, amount_cents, created_at")
    .eq("show_on_list", true)
    .order("created_at", { ascending: false })
    .limit(20);

  const recentDonors = recentDonorsRaw as {
    donor_name: string | null;
    amount_cents: number;
    created_at: string;
  }[] | null;

  return (
    <div>
      {/* Thank you banner */}
      {params.thanks === "1" && <ThankYouBanner />}

      {/* Hero + Form — two-column layout */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 pt-20 pb-20 md:pt-28 md:pb-28">
          <div className="grid lg:grid-cols-2 gap-12 lg:gap-16 items-start">
            {/* Left — copy + stats */}
            <div className="lg:sticky lg:top-28">
              <p className="text-sm font-medium text-primary mb-4">
                Open Source
              </p>
              <h1 className="text-4xl md:text-5xl font-bold tracking-tight leading-[1.1]">
                Support Omni
              </h1>
              <p className="mt-6 text-base md:text-lg text-muted-foreground max-w-md leading-relaxed">
                Omni is free and open source. Your donations fund infrastructure,
                development, and the security pipeline that keeps every extension safe.
              </p>

              {/* Inline stats */}
              <div className="mt-10 grid grid-cols-2 gap-4">
                {[
                  { label: "Today", value: formatCents(stats?.today_cents ?? 0) },
                  { label: "This Month", value: formatCents(stats?.month_cents ?? 0) },
                  { label: "This Year", value: formatCents(stats?.year_cents ?? 0) },
                  { label: "All Time", value: formatCents(stats?.total_cents ?? 0) },
                ].map((stat) => (
                  <div
                    key={stat.label}
                    className="rounded-lg border border-border/50 bg-card/50 px-4 py-3"
                  >
                    <p className="text-xl md:text-2xl font-bold tracking-tight">
                      {stat.value}
                    </p>
                    <p className="text-[10px] font-mono uppercase tracking-widest text-muted-foreground/50 mt-0.5">
                      {stat.label}
                    </p>
                  </div>
                ))}
              </div>
            </div>

            {/* Right — form */}
            <div>
              <DonateForm />
            </div>
          </div>
        </div>
      </section>

      {/* Where it goes */}
      <section className="border-t border-border/50">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
          <div className="max-w-2xl mb-10">
            <p className="text-sm font-medium text-primary mb-3">Where it goes</p>
            <h2 className="text-2xl md:text-3xl font-bold tracking-tight">
              What your donation supports
            </h2>
          </div>
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-px bg-border/50 border border-border/50 rounded-lg overflow-hidden">
            {SUPPORT_ITEMS.map((item) => {
              const Icon = item.icon;
              return (
                <div key={item.title} className="bg-card p-6 md:p-8">
                  <div className="h-9 w-9 rounded-lg bg-primary/10 flex items-center justify-center mb-4">
                    <Icon className="h-4 w-4 text-primary" />
                  </div>
                  <h3 className="font-medium text-[15px] mb-2">{item.title}</h3>
                  <p className="text-sm text-muted-foreground leading-relaxed">
                    {item.desc}
                  </p>
                </div>
              );
            })}
          </div>
        </div>
      </section>

      {/* Recent donors */}
      {recentDonors && recentDonors.length > 0 && (
        <section className="border-t border-border/50">
          <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-20 md:py-28">
            <div className="max-w-2xl mx-auto">
              <p className="text-sm font-medium text-primary mb-3">Community</p>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight mb-8">
                Recent Supporters
              </h2>
              <div className="border border-border/50 rounded-lg overflow-hidden">
                {recentDonors.map((donor, i) => (
                  <div
                    key={`${donor.created_at}-${i}`}
                    className={`bg-card px-5 py-4 flex items-center justify-between ${i > 0 ? "border-t border-border/50" : ""}`}
                  >
                    <div className="flex items-center gap-3">
                      <div className="h-7 w-7 rounded-full bg-primary/10 flex items-center justify-center shrink-0">
                        <Heart className="h-3 w-3 text-primary" />
                      </div>
                      <span className="text-sm font-medium">
                        {donor.donor_name || "Anonymous"}
                      </span>
                    </div>
                    <div className="flex items-center gap-4">
                      <span className="text-sm font-medium text-primary">
                        {formatCents(donor.amount_cents)}
                      </span>
                      <span className="text-xs text-muted-foreground/50 font-mono">
                        {new Date(donor.created_at).toLocaleDateString("en-US", {
                          month: "short",
                          day: "numeric",
                        })}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>
      )}
    </div>
  );
}
