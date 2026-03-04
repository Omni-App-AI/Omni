import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import { ChevronRight } from "lucide-react";
import { createClient } from "@/lib/supabase/server";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { formatBytes } from "@/lib/utils";
import type { ExtensionVersion } from "@/lib/supabase/types";

export const metadata: Metadata = {
  title: "Version History — Changelogs & Scan Results",
  description:
    "View all published versions, changelogs, and 4-layer security scan results for this Omni WASM extension. Track updates, bug fixes, and compatibility changes across every release.",
};

export default async function VersionHistoryPage({
  params,
}: {
  params: Promise<{ extensionId: string }>;
}) {
  const { extensionId } = await params;
  const supabase = await createClient();

  const { data: extData } = await supabase
    .from("extensions")
    .select("name")
    .eq("id", extensionId)
    .single();

  const extension = extData as { name: string } | null;
  if (!extension) notFound();

  const { data: versionsData } = await supabase
    .from("extension_versions")
    .select("*")
    .eq("extension_id", extensionId)
    .eq("published", true)
    .order("created_at", { ascending: false });

  const versions = versionsData as ExtensionVersion[] | null;

  return (
    <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-8">
      <nav className="flex items-center gap-1 text-sm text-muted-foreground mb-6">
        <Link href="/extensions" className="hover:text-foreground">Extensions</Link>
        <ChevronRight className="h-3 w-3" />
        <Link href={`/extensions/${extensionId}`} className="hover:text-foreground">
          {extension.name}
        </Link>
        <ChevronRight className="h-3 w-3" />
        <span className="text-foreground">Versions</span>
      </nav>

      <h1 className="text-2xl font-bold mb-6">{extension.name} — Version History</h1>

      <Card>
        <CardContent className="p-0">
          {!versions || versions.length === 0 ? (
            <p className="p-6 text-muted-foreground">No published versions.</p>
          ) : (
            <div className="divide-y divide-border">
              {versions.map((v) => (
                <div key={v.id} className="p-6">
                  <div className="flex items-start justify-between">
                    <div>
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-mono text-lg font-semibold">v{v.version}</span>
                        {v.scan_status === "passed" && (
                          <Badge variant="success">Verified</Badge>
                        )}
                        {v.scan_score && (
                          <Badge variant="secondary">Score: {v.scan_score}/100</Badge>
                        )}
                      </div>
                      <p className="text-sm text-muted-foreground">
                        {new Date(v.created_at).toLocaleDateString("en-US", {
                          year: "numeric",
                          month: "long",
                          day: "numeric",
                        })}
                        {" · "}
                        {formatBytes(v.wasm_size_bytes)}
                        {v.min_omni_version && ` · Requires Omni ${v.min_omni_version}+`}
                      </p>
                    </div>
                  </div>
                  {v.changelog && (
                    <p className="mt-3 text-sm whitespace-pre-wrap">{v.changelog}</p>
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
