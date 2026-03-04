import type { Metadata } from "next";
import { ApiKeyManager } from "@/components/dashboard/ApiKeyManager";

export const metadata: Metadata = {
  title: "API Keys",
  description:
    "Manage your Omni Marketplace API keys for CLI publishing.",
};

export default function ApiKeysPage() {
  return (
    <div>
      {/* Header */}
      <section className="relative overflow-hidden border-b border-border/50">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative px-8 lg:px-12 pt-10 pb-8">
          <div>
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
              Developer
            </p>
            <h1 className="text-3xl font-bold tracking-tight">
              API Keys
            </h1>
            <p className="text-muted-foreground mt-1">
              Manage API keys for publishing extensions via the CLI.
            </p>
          </div>
        </div>
      </section>

      {/* Content */}
      <section className="px-8 lg:px-12 py-8">
        <div className="max-w-4xl">
          <ApiKeyManager />
        </div>
      </section>
    </div>
  );
}
