"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { Command } from "cmdk";
import {
  Search,
  FileText,
  Shield,
  Wrench,
  Zap,
  BookOpen,
  Code2,
  Layers,
  ArrowRight,
  Clock,
  X,
  GitBranch,
} from "lucide-react";
import { searchEntries, type SearchEntry } from "@/lib/search/search-data";

/* ────────────────────────── helpers ────────────────────────── */

function escapeRegex(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function scoreEntry(entry: SearchEntry, terms: string[]): number {
  let score = 0;
  const sectionLower = entry.section.toLowerCase();
  const pageTitleLower = entry.pageTitle.toLowerCase();
  const contentLower = entry.content.toLowerCase();
  const keywordsLower = entry.keywords.toLowerCase();
  const combined = `${sectionLower} ${pageTitleLower} ${contentLower} ${keywordsLower}`;

  for (const term of terms) {
    // exact section heading match
    if (sectionLower === term) score += 100;
    // word-boundary match in section title
    else if (new RegExp(`\\b${escapeRegex(term)}`, "i").test(entry.section))
      score += 50;
    // word-boundary match in page title
    if (new RegExp(`\\b${escapeRegex(term)}`, "i").test(entry.pageTitle))
      score += 30;
    // keyword contains
    if (keywordsLower.includes(term)) score += 25;
    // content contains
    if (contentLower.includes(term)) score += 10;
    // prefix match on any word
    const words = `${sectionLower} ${pageTitleLower} ${keywordsLower}`.split(
      /\s+/,
    );
    if (words.some((w) => w.startsWith(term))) score += 15;
  }

  // bonus if ALL terms present somewhere
  const allMatch = terms.every((t) => combined.includes(t));
  if (allMatch && terms.length > 1) score = Math.round(score * 1.5);

  return score;
}

function searchDocs(query: string): SearchEntry[] {
  const terms = query
    .toLowerCase()
    .split(/\s+/)
    .filter((t) => t.length > 0);
  if (!terms.length) return [];

  return searchEntries
    .map((entry) => ({ entry, score: scoreEntry(entry, terms) }))
    .filter((r) => r.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, 20)
    .map((r) => r.entry);
}

function highlightText(text: string, query: string) {
  if (!query.trim()) return text;
  const terms = query
    .trim()
    .split(/\s+/)
    .filter((t) => t.length > 0);
  const regex = new RegExp(`(${terms.map(escapeRegex).join("|")})`, "gi");
  const parts = text.split(regex);
  return parts.map((part, i) =>
    regex.test(part) ? (
      <span key={i} className="text-primary font-semibold">
        {part}
      </span>
    ) : (
      part
    ),
  );
}

const categoryIcons: Record<string, typeof FileText> = {
  "Getting Started": BookOpen,
  "Core Concepts": Shield,
  Developers: Code2,
  Resources: FileText,
};

const pageIcons: Record<string, typeof FileText> = {
  "getting-started": Zap,
  configuration: Wrench,
  providers: Layers,
  channels: ArrowRight,
  security: Shield,
  tools: Wrench,
  flowcharts: GitBranch,
  hooks: Zap,
  architecture: Layers,
  sdk: Code2,
  publishing: ArrowRight,
  building: Wrench,
  changelog: Clock,
  docs: BookOpen,
};

const RECENT_KEY = "omni-docs-recent-searches";
const MAX_RECENT = 5;

function getRecentSearches(): string[] {
  if (typeof window === "undefined") return [];
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) || "[]");
  } catch {
    return [];
  }
}

function addRecentSearch(query: string) {
  const recent = getRecentSearches().filter((s) => s !== query);
  recent.unshift(query);
  localStorage.setItem(
    RECENT_KEY,
    JSON.stringify(recent.slice(0, MAX_RECENT)),
  );
}

function clearRecentSearches() {
  localStorage.removeItem(RECENT_KEY);
}

/* ────────────────────────── component ────────────────────────── */

export function DocsSearch() {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [recentSearches, setRecentSearches] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);

  // keyboard shortcut
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen((o) => !o);
      }
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);

  // load recent searches when dialog opens
  useEffect(() => {
    if (open) {
      setRecentSearches(getRecentSearches());
      setQuery("");
    }
  }, [open]);

  const results = useMemo(() => searchDocs(query), [query]);

  // group results by category
  const grouped = useMemo(() => {
    const groups: Record<string, SearchEntry[]> = {};
    for (const entry of results) {
      if (!groups[entry.category]) groups[entry.category] = [];
      groups[entry.category].push(entry);
    }
    return groups;
  }, [results]);

  const navigate = useCallback(
    (href: string) => {
      setOpen(false);
      if (query.trim()) addRecentSearch(query.trim());
      router.push(href);
    },
    [router, query],
  );

  const handleRecentClick = useCallback(
    (search: string) => {
      setQuery(search);
      inputRef.current?.focus();
    },
    [],
  );

  const handleClearRecent = useCallback(() => {
    clearRecentSearches();
    setRecentSearches([]);
  }, []);

  const hasResults = results.length > 0;
  const hasQuery = query.trim().length > 0;

  return (
    <>
      {/* Trigger button (used by Navbar) */}
      <button
        onClick={() => setOpen(true)}
        className="flex items-center gap-2 px-3 py-1.5 text-[13px] text-muted-foreground hover:text-foreground transition-colors rounded-md border border-border/50 hover:border-border bg-secondary/30 hover:bg-secondary/50"
      >
        <Search className="h-3.5 w-3.5" />
        <span className="hidden lg:inline">Search docs</span>
        <kbd className="hidden lg:inline-flex items-center gap-0.5 ml-2 px-1.5 py-0.5 text-[10px] font-mono text-muted-foreground/60 bg-background/50 rounded border border-border/50">
          <span className="text-[11px]">⌘</span>K
        </kbd>
      </button>

      {/* Command palette dialog */}
      <Command.Dialog
        open={open}
        onOpenChange={setOpen}
        label="Search documentation"
        shouldFilter={false}
        className="docs-search-dialog"
      >
        <div className="docs-search-header">
          <Search className="h-4 w-4 text-muted-foreground shrink-0" />
          <Command.Input
            ref={inputRef}
            value={query}
            onValueChange={setQuery}
            placeholder="Search documentation..."
            className="docs-search-input"
          />
          {hasQuery && (
            <button
              onClick={() => setQuery("")}
              className="p-0.5 rounded hover:bg-secondary/60 text-muted-foreground/60 hover:text-muted-foreground transition-colors"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
          <kbd className="hidden sm:inline-flex px-1.5 py-0.5 text-[10px] font-mono text-muted-foreground/40 border border-border/30 rounded shrink-0">
            ESC
          </kbd>
        </div>

        <Command.List className="docs-search-list">
          {/* Empty state */}
          {hasQuery && !hasResults && (
            <Command.Empty className="docs-search-empty">
              <Search className="h-10 w-10 text-muted-foreground/20 mb-3" />
              <p className="text-sm text-muted-foreground">
                No results for &ldquo;
                <span className="text-foreground font-medium">{query}</span>
                &rdquo;
              </p>
              <p className="text-xs text-muted-foreground/60 mt-1">
                Try different keywords or check spelling
              </p>
            </Command.Empty>
          )}

          {/* Recent searches (when no query) */}
          {!hasQuery && recentSearches.length > 0 && (
            <Command.Group
              heading={
                <span className="flex items-center justify-between w-full">
                  <span>Recent searches</span>
                  <button
                    onClick={handleClearRecent}
                    className="text-[11px] text-muted-foreground/40 hover:text-muted-foreground transition-colors font-normal"
                  >
                    Clear
                  </button>
                </span>
              }
              className="docs-search-group"
            >
              {recentSearches.map((search) => (
                <Command.Item
                  key={search}
                  value={search}
                  onSelect={() => handleRecentClick(search)}
                  className="docs-search-item"
                >
                  <Clock className="h-3.5 w-3.5 text-muted-foreground/40 shrink-0" />
                  <span className="text-sm text-muted-foreground">{search}</span>
                </Command.Item>
              ))}
            </Command.Group>
          )}

          {/* Quick navigation (when no query) */}
          {!hasQuery && (
            <Command.Group heading="Quick navigation" className="docs-search-group">
              {[
                { href: "/docs/getting-started", label: "Getting Started", slug: "getting-started" },
                { href: "/docs/configuration", label: "Configuration", slug: "configuration" },
                { href: "/docs/providers", label: "LLM Providers", slug: "providers" },
                { href: "/docs/channels", label: "Channels", slug: "channels" },
                { href: "/docs/security", label: "Security & Permissions", slug: "security" },
                { href: "/docs/tools", label: "Native Tools", slug: "tools" },
                { href: "/docs/flowcharts", label: "Flowchart Builder", slug: "flowcharts" },
                { href: "/docs/sdk", label: "SDK Reference", slug: "sdk" },
                { href: "/docs/architecture", label: "Architecture", slug: "architecture" },
              ].map((item) => {
                const Icon = pageIcons[item.slug] || FileText;
                return (
                  <Command.Item
                    key={item.href}
                    value={item.label}
                    onSelect={() => navigate(item.href)}
                    className="docs-search-item"
                  >
                    <Icon className="h-3.5 w-3.5 text-muted-foreground/40 shrink-0" />
                    <span className="text-sm">{item.label}</span>
                    <ArrowRight className="h-3 w-3 text-muted-foreground/30 ml-auto shrink-0 opacity-0 group-data-[selected=true]:opacity-100 transition-opacity" />
                  </Command.Item>
                );
              })}
            </Command.Group>
          )}

          {/* Search results grouped by category */}
          {hasQuery &&
            Object.entries(grouped).map(([category, entries]) => {
              const CategoryIcon = categoryIcons[category] || FileText;
              return (
                <Command.Group
                  key={category}
                  heading={
                    <span className="flex items-center gap-1.5">
                      <CategoryIcon className="h-3 w-3" />
                      {category}
                    </span>
                  }
                  className="docs-search-group"
                >
                  {entries.map((entry) => {
                    const PageIcon = pageIcons[entry.pageSlug] || FileText;
                    return (
                      <Command.Item
                        key={entry.id}
                        value={`${entry.pageTitle} ${entry.section} ${entry.keywords}`}
                        onSelect={() => navigate(entry.href)}
                        className="docs-search-item"
                      >
                        <PageIcon className="h-3.5 w-3.5 text-muted-foreground/40 shrink-0 mt-0.5" />
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="text-sm font-medium truncate">
                              {highlightText(entry.section, query)}
                            </span>
                            {entry.section !== entry.pageTitle && (
                              <span className="text-[11px] text-muted-foreground/40 truncate shrink-0">
                                {entry.pageTitle}
                              </span>
                            )}
                          </div>
                          <p className="text-xs text-muted-foreground/60 truncate mt-0.5">
                            {highlightText(entry.content, query)}
                          </p>
                        </div>
                        <ArrowRight className="h-3 w-3 text-muted-foreground/30 ml-2 shrink-0 opacity-0 group-data-[selected=true]:opacity-100 transition-opacity" />
                      </Command.Item>
                    );
                  })}
                </Command.Group>
              );
            })}
        </Command.List>

        {/* Footer */}
        <div className="docs-search-footer">
          <div className="flex items-center gap-3">
            <span className="flex items-center gap-1 text-[11px] text-muted-foreground/40">
              <kbd className="px-1 py-0.5 bg-secondary/50 rounded border border-border/30 text-[10px]">
                ↑↓
              </kbd>
              Navigate
            </span>
            <span className="flex items-center gap-1 text-[11px] text-muted-foreground/40">
              <kbd className="px-1 py-0.5 bg-secondary/50 rounded border border-border/30 text-[10px]">
                ↵
              </kbd>
              Open
            </span>
            <span className="flex items-center gap-1 text-[11px] text-muted-foreground/40">
              <kbd className="px-1 py-0.5 bg-secondary/50 rounded border border-border/30 text-[10px]">
                esc
              </kbd>
              Close
            </span>
          </div>
          <span className="text-[11px] text-muted-foreground/30">
            {hasQuery
              ? `${results.length} result${results.length !== 1 ? "s" : ""}`
              : `${searchEntries.length} sections indexed`}
          </span>
        </div>
      </Command.Dialog>
    </>
  );
}
