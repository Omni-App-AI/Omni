import type { Metadata } from "next";
import { DocsSidebar } from "@/components/layout/DocsSidebar";

export const metadata: Metadata = {
  title: "Changelog — Platform & Marketplace Updates",
  description:
    "Track every Omni AI agent release, WASM runtime update, marketplace feature, and security patch. Full version history with detailed changelogs for the platform and SDK.",
  openGraph: {
    title: "Omni Changelog — Platform, SDK & Marketplace Updates",
    description:
      "Track every Omni AI agent release, WASM runtime update, marketplace feature, and security patch. Full version history with detailed changelogs.",
    url: "/docs/changelog",
  },
  alternates: { canonical: "/docs/changelog" },
};

interface ReleaseSection {
  heading: string;
  items: string[];
}

interface Release {
  version: string;
  date: string;
  tag: string | null;
  sections: ReleaseSection[];
}

const releases: Release[] = [
  {
    version: "1.0.0",
    date: "2026-03-06",
    tag: "Upcoming",
    sections: [],
  },
];

const tagColor: Record<string, string> = {
  Added: "text-success",
  Changed: "text-blue-400",
  Fixed: "text-warning",
  Removed: "text-destructive",
  Security: "text-purple-400",
};

export default function ChangelogPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 py-16 md:py-24">
      <div className="flex gap-12">
        <DocsSidebar />

        <div className="flex-1 min-w-0 max-w-3xl">
          <p className="text-sm font-medium text-primary mb-3">Docs</p>
          <h1 className="text-3xl font-bold tracking-tight mb-2">Changelog</h1>
          <p className="text-muted-foreground mb-12">
            Omni hasn&apos;t released yet — v1.0.0 is planned for March 6, 2026.
            This page will track every platform release, SDK update, and security patch
            following{" "}
            <a
              href="https://keepachangelog.com"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary hover:underline"
            >
              Keep a Changelog
            </a>{" "}
            conventions.
          </p>

          <div className="space-y-16">
            {releases.map((release) => (
              <section key={release.version} id={`v${release.version}`}>
                <div className="flex items-baseline gap-3 mb-4 pb-2 border-b border-border/50">
                  <h2 className="text-xl font-medium">v{release.version}</h2>
                  <span className="text-sm text-muted-foreground">{release.date}</span>
                  {release.tag && (
                    <span className="text-[11px] font-mono uppercase px-2 py-0.5 rounded-full bg-primary/10 text-primary">
                      {release.tag}
                    </span>
                  )}
                </div>

                <div className="space-y-6">
                  {release.sections.length === 0 && (
                    <p className="text-sm text-muted-foreground">
                      Release notes will be published on launch day. Stay tuned.
                    </p>
                  )}
                  {release.sections.map((section) => (
                    <div key={section.heading}>
                      <h3
                        className={`text-sm font-medium mb-3 ${tagColor[section.heading] || "text-foreground"}`}
                      >
                        {section.heading}
                      </h3>
                      <ul className="space-y-2">
                        {section.items.map((item, i) => (
                          <li key={i} className="flex gap-2 text-sm text-muted-foreground">
                            <span className="text-muted-foreground/40 shrink-0">-</span>
                            {item}
                          </li>
                        ))}
                      </ul>
                    </div>
                  ))}
                </div>
              </section>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
