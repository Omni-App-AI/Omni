import type { Metadata } from "next";
import { ExtensionForm } from "@/components/dashboard/ExtensionForm";

export const metadata: Metadata = {
  title: "Publish Extension",
  description:
    "Upload and publish a new WASM extension to the Omni Marketplace.",
};

export default function NewExtensionPage() {
  return (
    <div>
      {/* Header */}
      <section className="relative overflow-hidden border-b border-border/50">
        <div className="absolute inset-0 gradient-mesh" />
        <div className="absolute inset-0 bg-grid fade-bottom" />
        <div className="relative px-8 lg:px-12 pt-10 pb-8">
          <div>
            <p className="text-[11px] font-mono uppercase tracking-widest text-muted-foreground/60 mb-2">
              Publish
            </p>
            <h1 className="text-3xl font-bold tracking-tight">
              Publish New Extension
            </h1>
            <p className="text-muted-foreground mt-1">
              Upload your WASM extension and provide details for the marketplace listing.
            </p>
          </div>
        </div>
      </section>

      {/* Form */}
      <section className="px-8 lg:px-12 py-8">
        <div className="max-w-4xl">
          <ExtensionForm />
        </div>
      </section>
    </div>
  );
}
