"use client";

import { usePathname } from "next/navigation";
import Link from "next/link";
import { ArrowRight, FileText, BookOpen, Package, MessageSquare, LayoutDashboard } from "lucide-react";

// All known static routes
const KNOWN_ROUTES = [
  { path: "/", label: "Home" },
  { path: "/extensions", label: "Browse Extensions" },
  { path: "/community", label: "Community Forum" },
  { path: "/community/new", label: "New Discussion" },
  { path: "/docs", label: "Documentation" },
  { path: "/docs/getting-started", label: "Getting Started Guide" },
  { path: "/docs/sdk", label: "SDK Reference" },
  { path: "/docs/publishing", label: "Publishing Guide" },
  { path: "/about", label: "About Omni" },
  { path: "/blog", label: "Blog" },
  { path: "/login", label: "Login" },
  { path: "/signup", label: "Sign Up" },
  { path: "/dashboard", label: "Dashboard" },
  { path: "/dashboard/extensions", label: "My Extensions" },
  { path: "/dashboard/extensions/new", label: "Create Extension" },
  { path: "/dashboard/api-keys", label: "API Keys" },
  { path: "/dashboard/settings", label: "Account Settings" },
  { path: "/dashboard/posts", label: "My Posts" },
  { path: "/privacy", label: "Privacy Policy" },
  { path: "/terms", label: "Terms of Service" },
  { path: "/security", label: "Security" },
];

const BLOG_POSTS = [
  { title: "Introducing the Omni Marketplace", date: "Feb 15, 2026", category: "Announcement" },
  { title: "How Our 4-Layer Security Pipeline Works", date: "Feb 10, 2026", category: "Security" },
  { title: "Building Your First Extension with the Omni SDK", date: "Feb 5, 2026", category: "Tutorial" },
  { title: "Why We Chose WASM for Extension Sandboxing", date: "Jan 28, 2026", category: "Engineering" },
  { title: "Omni v1.0 — Privacy-First AI Agents", date: "Jan 15, 2026", category: "Release" },
];

const DOC_PAGES = [
  { path: "/docs", label: "Documentation Home", desc: "Overview of the Omni platform" },
  { path: "/docs/getting-started", label: "Getting Started", desc: "Install the SDK and build your first extension" },
  { path: "/docs/sdk", label: "SDK Reference", desc: "Full API reference for the Omni Rust SDK" },
  { path: "/docs/publishing", label: "Publishing Guide", desc: "How to publish extensions to the marketplace" },
];

function levenshtein(a: string, b: string): number {
  const m = a.length;
  const n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));
  for (let i = 0; i <= m; i++) dp[i][0] = i;
  for (let j = 0; j <= n; j++) dp[0][j] = j;
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      dp[i][j] = Math.min(dp[i - 1][j] + 1, dp[i][j - 1] + 1, dp[i - 1][j - 1] + cost);
    }
  }
  return dp[m][n];
}

type Context = "blog" | "docs" | "extensions" | "community" | "dashboard" | "auth" | "general";

function detectContext(pathname: string): Context {
  const seg = pathname.split("/").filter(Boolean)[0]?.toLowerCase() ?? "";
  if (["blog", "blogs", "post", "posts", "article", "articles", "news"].includes(seg)) return "blog";
  if (["doc", "docs", "documentation", "guide", "guides", "help", "wiki", "tutorial", "tutorials", "api"].includes(seg)) return "docs";
  if (["extension", "extensions", "plugin", "plugins", "package", "packages", "marketplace", "store", "addons", "add-ons"].includes(seg)) return "extensions";
  if (["community", "forum", "forums", "discuss", "discussions", "support", "questions"].includes(seg)) return "community";
  if (["dashboard", "account", "profile", "settings", "admin", "panel", "manage"].includes(seg)) return "dashboard";
  if (["login", "signin", "sign-in", "signup", "sign-up", "register", "auth", "authenticate"].includes(seg)) return "auth";
  return "general";
}

function getFuzzySuggestions(pathname: string): { path: string; label: string }[] {
  const normalized = pathname.toLowerCase().replace(/\/+$/, "") || "/";
  return KNOWN_ROUTES
    .map((r) => ({ ...r, distance: levenshtein(normalized, r.path) }))
    .filter((r) => r.distance <= Math.max(3, Math.floor(normalized.length * 0.5)))
    .sort((a, b) => a.distance - b.distance)
    .slice(0, 4);
}

export function RouteSuggestions() {
  const pathname = usePathname();
  const context = detectContext(pathname);
  const fuzzy = getFuzzySuggestions(pathname);

  if (context === "blog") {
    return (
      <div className="relative z-10 mt-10 text-center px-4 max-w-lg mx-auto">
        <div className="border border-border/50 rounded-lg bg-card/50 backdrop-blur-sm p-6 text-left">
          <div className="flex items-center gap-2 mb-4">
            <FileText className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-medium">Looking for a blog post?</h2>
          </div>
          <div className="space-y-3">
            {BLOG_POSTS.map((post) => (
              <Link
                key={post.title}
                href="/blog"
                className="block group"
              >
                <div className="flex items-baseline justify-between gap-3">
                  <span className="text-sm text-muted-foreground group-hover:text-foreground transition-colors line-clamp-1">
                    {post.title}
                  </span>
                  <span className="text-[11px] font-mono text-muted-foreground/50 shrink-0">
                    {post.category}
                  </span>
                </div>
              </Link>
            ))}
          </div>
          <Link
            href="/blog"
            className="mt-4 flex items-center gap-1 text-xs text-primary hover:text-primary/80 transition-colors"
          >
            View all posts <ArrowRight className="h-3 w-3" />
          </Link>
        </div>
      </div>
    );
  }

  if (context === "docs") {
    return (
      <div className="relative z-10 mt-10 text-center px-4 max-w-lg mx-auto">
        <div className="border border-border/50 rounded-lg bg-card/50 backdrop-blur-sm p-6 text-left">
          <div className="flex items-center gap-2 mb-4">
            <BookOpen className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-medium">Looking for documentation?</h2>
          </div>
          <div className="space-y-3">
            {DOC_PAGES.map((doc) => (
              <Link
                key={doc.path}
                href={doc.path}
                className="block group"
              >
                <span className="text-sm text-muted-foreground group-hover:text-foreground transition-colors">
                  {doc.label}
                </span>
                <p className="text-xs text-muted-foreground/50 mt-0.5">{doc.desc}</p>
              </Link>
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (context === "extensions") {
    return (
      <div className="relative z-10 mt-10 text-center px-4 max-w-lg mx-auto">
        <div className="border border-border/50 rounded-lg bg-card/50 backdrop-blur-sm p-6 text-left">
          <div className="flex items-center gap-2 mb-4">
            <Package className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-medium">Looking for an extension?</h2>
          </div>
          <p className="text-sm text-muted-foreground mb-4">
            The extension you're looking for may have been renamed, removed, or doesn't exist yet.
          </p>
          <Link
            href="/extensions"
            className="text-xs text-primary hover:text-primary/80 transition-colors flex items-center gap-1"
          >
            Browse marketplace <ArrowRight className="h-3 w-3" />
          </Link>
        </div>
      </div>
    );
  }

  if (context === "community") {
    return (
      <div className="relative z-10 mt-10 text-center px-4 max-w-lg mx-auto">
        <div className="border border-border/50 rounded-lg bg-card/50 backdrop-blur-sm p-6 text-left">
          <div className="flex items-center gap-2 mb-4">
            <MessageSquare className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-medium">Looking for a discussion?</h2>
          </div>
          <p className="text-sm text-muted-foreground mb-4">
            This thread may have been deleted or moved. Try browsing the community forum.
          </p>
          <div className="flex items-center gap-3">
            <Link
              href="/community"
              className="text-xs text-primary hover:text-primary/80 transition-colors flex items-center gap-1"
            >
              Community forum <ArrowRight className="h-3 w-3" />
            </Link>
            <Link
              href="/community/new"
              className="text-xs text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
            >
              Start a discussion <ArrowRight className="h-3 w-3" />
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (context === "dashboard") {
    return (
      <div className="relative z-10 mt-10 text-center px-4 max-w-lg mx-auto">
        <div className="border border-border/50 rounded-lg bg-card/50 backdrop-blur-sm p-6 text-left">
          <div className="flex items-center gap-2 mb-4">
            <LayoutDashboard className="h-4 w-4 text-primary" />
            <h2 className="text-sm font-medium">Dashboard pages</h2>
          </div>
          <div className="space-y-2">
            {KNOWN_ROUTES.filter((r) => r.path.startsWith("/dashboard")).map((route) => (
              <Link
                key={route.path}
                href={route.path}
                className="block text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                {route.label}
                <span className="text-[11px] text-muted-foreground/40 ml-2 font-mono">{route.path}</span>
              </Link>
            ))}
          </div>
        </div>
      </div>
    );
  }

  // General / auth / unknown context -- show fuzzy matches
  if (fuzzy.length === 0) return null;

  return (
    <div className="relative z-10 mt-10 text-center px-4 max-w-md mx-auto">
      <div className="border border-border/50 rounded-lg bg-card/50 backdrop-blur-sm p-6 text-left">
        <h2 className="text-sm font-medium mb-4">Maybe you were looking for</h2>
        <div className="space-y-2">
          {fuzzy.map((route) => (
            <Link
              key={route.path}
              href={route.path}
              className="flex items-center justify-between group"
            >
              <span className="text-sm text-muted-foreground group-hover:text-foreground transition-colors">
                {route.label}
              </span>
              <span className="text-[11px] font-mono text-muted-foreground/40">{route.path}</span>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}
