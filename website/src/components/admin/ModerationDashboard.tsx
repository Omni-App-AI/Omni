"use client";

import { useState, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Shield, AlertTriangle, Ban, CheckCircle, XCircle, RefreshCw, ChevronDown,
  Package, Eye, EyeOff,
} from "lucide-react";

// ── Types ──────────────────────────────────────────────────

interface Flag {
  id: string;
  content_type: string;
  content_id: string;
  reason: string;
  details: string | null;
  spam_score: number | null;
  status: string;
  created_at: string;
  reporter: { username: string; display_name: string } | null;
  moderator: { username: string; display_name: string } | null;
}

interface ModExtension {
  id: string;
  name: string;
  short_description: string;
  icon_url: string | null;
  trust_level: string;
  published: boolean;
  total_downloads: number;
  average_rating: number;
  moderation_status: string;
  moderation_note: string | null;
  moderated_at: string | null;
  created_at: string;
  publisher: { username: string; display_name: string } | null;
}

interface Stats {
  pending_flags: number;
  security_events_24h: number;
  blocked_ips: number;
  active_bans: number;
  rate_limits_24h: number;
  turnstile_fails_24h: number;
  honeypots_caught_24h: number;
  spam_blocked_24h: number;
  extensions_under_review: number;
  extensions_taken_down: number;
}

// ── Main Component ─────────────────────────────────────────

export function ModerationDashboard() {
  const [tab, setTab] = useState<"flags" | "extensions">("flags");
  const [flags, setFlags] = useState<Flag[]>([]);
  const [extensions, setExtensions] = useState<ModExtension[]>([]);
  const [stats, setStats] = useState<Stats | null>(null);
  const [loading, setLoading] = useState(true);
  const [statusFilter, setStatusFilter] = useState("pending");
  const [extFilter, setExtFilter] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [actioningId, setActioningId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchFlags = useCallback(async () => {
    try {
      const res = await fetch(`/api/v1/admin/moderation/queue?status=${statusFilter}&page=${page}`);
      if (!res.ok) {
        if (res.status === 403) {
          setError("You do not have moderator access.");
          return;
        }
        throw new Error("Failed to fetch");
      }
      const data = await res.json();
      setFlags(data.flags || []);
      setTotalPages(data.pages || 1);
    } catch {
      setError("Failed to load moderation queue");
    }
  }, [statusFilter, page]);

  const fetchExtensions = useCallback(async () => {
    try {
      const params = new URLSearchParams({ page: String(page) });
      if (extFilter) params.set("moderation_status", extFilter);
      const res = await fetch(`/api/v1/admin/extensions?${params}`);
      if (!res.ok) throw new Error("Failed to fetch");
      const data = await res.json();
      setExtensions(data.extensions || []);
      setTotalPages(data.pages || 1);
    } catch {
      setError("Failed to load extensions");
    }
  }, [extFilter, page]);

  const fetchStats = useCallback(async () => {
    try {
      const res = await fetch("/api/v1/admin/stats");
      if (res.ok) {
        setStats(await res.json());
      }
    } catch {
      // Stats are non-critical
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    const fetcher = tab === "flags" ? fetchFlags : fetchExtensions;
    Promise.all([fetcher(), fetchStats()]).finally(() => setLoading(false));
  }, [tab, fetchFlags, fetchExtensions, fetchStats]);

  const resolveFlag = async (flagId: string, status: string, action?: string) => {
    setActioningId(flagId);
    try {
      const res = await fetch(`/api/v1/admin/moderation/flags/${flagId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ status, action }),
      });

      if (res.ok) {
        setFlags((prev) => prev.filter((f) => f.id !== flagId));
        fetchStats();
      }
    } catch {
      // Ignore
    }
    setActioningId(null);
  };

  const moderateExtension = async (extensionId: string, action: string, note?: string) => {
    setActioningId(extensionId);
    try {
      const res = await fetch(`/api/v1/admin/extensions/${extensionId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action, note }),
      });
      if (res.ok) {
        fetchExtensions();
        fetchStats();
      }
    } catch {
      // Ignore
    }
    setActioningId(null);
  };

  if (error) {
    return (
      <div className="max-w-4xl mx-auto p-8">
        <div className="bg-destructive/10 text-destructive rounded-lg p-4">{error}</div>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Shield className="h-6 w-6" /> Moderation Dashboard
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            Review flagged content, manage extensions, and monitor anti-bot defenses
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={() => {
          if (tab === "flags") fetchFlags();
          else fetchExtensions();
          fetchStats();
        }}>
          <RefreshCw className="h-4 w-4 mr-1" /> Refresh
        </Button>
      </div>

      {/* Stats Grid */}
      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
          <StatCard label="Pending Flags" value={stats.pending_flags} icon={<AlertTriangle className="h-4 w-4 text-warning" />} />
          <StatCard label="Under Review" value={stats.extensions_under_review} icon={<Eye className="h-4 w-4 text-yellow-400" />} />
          <StatCard label="Taken Down" value={stats.extensions_taken_down} icon={<EyeOff className="h-4 w-4 text-destructive" />} />
          <StatCard label="Blocked IPs" value={stats.blocked_ips} icon={<Ban className="h-4 w-4 text-destructive" />} />
          <StatCard label="Active Bans" value={stats.active_bans} icon={<Ban className="h-4 w-4 text-destructive" />} />
          <StatCard label="Security Events (24h)" value={stats.security_events_24h} icon={<Shield className="h-4 w-4 text-blue-400" />} />
          <StatCard label="Rate Limits (24h)" value={stats.rate_limits_24h} icon={<AlertTriangle className="h-4 w-4 text-yellow-400" />} />
          <StatCard label="Turnstile Fails (24h)" value={stats.turnstile_fails_24h} icon={<XCircle className="h-4 w-4 text-red-400" />} />
          <StatCard label="Honeypots (24h)" value={stats.honeypots_caught_24h} icon={<AlertTriangle className="h-4 w-4 text-orange-400" />} />
          <StatCard label="Spam Blocked (24h)" value={stats.spam_blocked_24h} icon={<XCircle className="h-4 w-4 text-red-400" />} />
        </div>
      )}

      {/* Tab Bar */}
      <div className="flex gap-2 border-b border-border pb-2">
        <Button
          variant={tab === "flags" ? "default" : "ghost"}
          size="sm"
          onClick={() => { setTab("flags"); setPage(1); }}
        >
          <AlertTriangle className="h-4 w-4 mr-1" />
          Content Flags
        </Button>
        <Button
          variant={tab === "extensions" ? "default" : "ghost"}
          size="sm"
          onClick={() => { setTab("extensions"); setPage(1); }}
        >
          <Package className="h-4 w-4 mr-1" />
          Extensions
        </Button>
      </div>

      {/* Tab Content */}
      {tab === "flags" ? (
        <FlagsTab
          flags={flags}
          loading={loading}
          statusFilter={statusFilter}
          onFilterChange={(s) => { setStatusFilter(s); setPage(1); }}
          actioningId={actioningId}
          onDismiss={(id) => resolveFlag(id, "dismissed", "dismiss")}
          onRemove={(id) => resolveFlag(id, "actioned", "remove_content")}
        />
      ) : (
        <ExtensionsTab
          extensions={extensions}
          loading={loading}
          extFilter={extFilter}
          onFilterChange={(f) => { setExtFilter(f); setPage(1); }}
          actioningId={actioningId}
          onModerate={moderateExtension}
        />
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex justify-center gap-2">
          <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
            Previous
          </Button>
          <span className="text-sm text-muted-foreground py-2">
            Page {page} of {totalPages}
          </span>
          <Button variant="outline" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
            Next
          </Button>
        </div>
      )}
    </div>
  );
}

// ── Flags Tab ──────────────────────────────────────────────

function FlagsTab({
  flags, loading, statusFilter, onFilterChange, actioningId, onDismiss, onRemove,
}: {
  flags: Flag[];
  loading: boolean;
  statusFilter: string;
  onFilterChange: (s: string) => void;
  actioningId: string | null;
  onDismiss: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  return (
    <>
      <div className="flex gap-2">
        {["pending", "actioned", "dismissed"].map((s) => (
          <Button
            key={s}
            variant={statusFilter === s ? "default" : "outline"}
            size="sm"
            onClick={() => onFilterChange(s)}
          >
            {s.charAt(0).toUpperCase() + s.slice(1)}
          </Button>
        ))}
      </div>

      {loading ? (
        <div className="text-center py-12 text-muted-foreground">Loading...</div>
      ) : flags.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <CheckCircle className="h-8 w-8 mx-auto mb-2 text-success" />
          No {statusFilter} flags
        </div>
      ) : (
        <div className="space-y-3">
          {flags.map((flag) => (
            <FlagCard
              key={flag.id}
              flag={flag}
              actioning={actioningId === flag.id}
              onDismiss={() => onDismiss(flag.id)}
              onRemoveContent={() => onRemove(flag.id)}
            />
          ))}
        </div>
      )}
    </>
  );
}

// ── Extensions Tab ─────────────────────────────────────────

function ExtensionsTab({
  extensions, loading, extFilter, onFilterChange, actioningId, onModerate,
}: {
  extensions: ModExtension[];
  loading: boolean;
  extFilter: string | null;
  onFilterChange: (f: string | null) => void;
  actioningId: string | null;
  onModerate: (id: string, action: string, note?: string) => void;
}) {
  return (
    <>
      <div className="flex gap-2">
        {[
          { key: null, label: "All" },
          { key: "under_review", label: "Under Review" },
          { key: "taken_down", label: "Taken Down" },
          { key: "active", label: "Active" },
        ].map((f) => (
          <Button
            key={f.key ?? "all"}
            variant={extFilter === f.key ? "default" : "outline"}
            size="sm"
            onClick={() => onFilterChange(f.key)}
          >
            {f.label}
          </Button>
        ))}
      </div>

      {loading ? (
        <div className="text-center py-12 text-muted-foreground">Loading...</div>
      ) : extensions.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <Package className="h-8 w-8 mx-auto mb-2 text-muted-foreground" />
          No extensions found
        </div>
      ) : (
        <div className="space-y-3">
          {extensions.map((ext) => (
            <ExtensionModCard
              key={ext.id}
              ext={ext}
              actioning={actioningId === ext.id}
              onModerate={onModerate}
            />
          ))}
        </div>
      )}
    </>
  );
}

// ── Shared Components ──────────────────────────────────────

function StatCard({ label, value, icon }: { label: string; value: number; icon: React.ReactNode }) {
  return (
    <div className="bg-card border border-border rounded-lg p-4">
      <div className="flex items-center justify-between">
        <span className="text-2xl font-bold">{value}</span>
        {icon}
      </div>
      <p className="text-xs text-muted-foreground mt-1">{label}</p>
    </div>
  );
}

// ── Flag Card ──────────────────────────────────────────────

function FlagCard({
  flag, actioning, onDismiss, onRemoveContent,
}: {
  flag: Flag;
  actioning: boolean;
  onDismiss: () => void;
  onRemoveContent: () => void;
}) {
  const [expanded, setExpanded] = useState(false);

  const reasonColors: Record<string, string> = {
    spam: "bg-red-500/10 text-red-400",
    auto_spam: "bg-red-500/10 text-red-400",
    auto_suspicious: "bg-yellow-500/10 text-yellow-400",
    harassment: "bg-orange-500/10 text-orange-400",
    malicious: "bg-red-500/10 text-red-400",
    misinformation: "bg-yellow-500/10 text-yellow-400",
    off_topic: "bg-blue-500/10 text-blue-400",
    other: "bg-gray-500/10 text-gray-400",
  };

  return (
    <div className="bg-card border border-border rounded-lg p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${reasonColors[flag.reason] || reasonColors.other}`}>
            {flag.reason.replace("_", " ")}
          </span>
          <span className="text-sm font-medium">{flag.content_type}</span>
          <span className="text-xs text-muted-foreground font-mono">{flag.content_id.slice(0, 8)}...</span>
          {flag.spam_score !== null && (
            <span className="text-xs text-muted-foreground">Score: {flag.spam_score}</span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {flag.status === "pending" && (
            <>
              <Button variant="outline" size="sm" disabled={actioning} onClick={onDismiss}>
                Dismiss
              </Button>
              <Button variant="destructive" size="sm" disabled={actioning} onClick={onRemoveContent}>
                Remove
              </Button>
            </>
          )}
          <button onClick={() => setExpanded(!expanded)} className="text-muted-foreground hover:text-foreground">
            <ChevronDown className={`h-4 w-4 transition-transform ${expanded ? "rotate-180" : ""}`} />
          </button>
        </div>
      </div>

      {expanded && (
        <div className="mt-3 pt-3 border-t border-border text-sm space-y-1">
          <p><span className="text-muted-foreground">Reported by:</span> {flag.reporter?.username || "System"}</p>
          <p><span className="text-muted-foreground">Date:</span> {new Date(flag.created_at).toLocaleString()}</p>
          {flag.details && (
            <p><span className="text-muted-foreground">Details:</span> {flag.details}</p>
          )}
          {flag.moderator && (
            <p><span className="text-muted-foreground">Reviewed by:</span> {flag.moderator.username}</p>
          )}
        </div>
      )}
    </div>
  );
}

// ── Extension Moderation Card ──────────────────────────────

function ExtensionModCard({
  ext, actioning, onModerate,
}: {
  ext: ModExtension;
  actioning: boolean;
  onModerate: (id: string, action: string, note?: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);

  const statusBadge: Record<string, { variant: "default" | "warning" | "destructive" | "success"; label: string }> = {
    active: { variant: "success", label: "Active" },
    under_review: { variant: "warning", label: "Under Review" },
    taken_down: { variant: "destructive", label: "Taken Down" },
  };

  const badge = statusBadge[ext.moderation_status] || statusBadge.active;

  return (
    <div className="bg-card border border-border rounded-lg p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3 min-w-0">
          <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10">
            {ext.icon_url ? (
              <img src={ext.icon_url} alt={ext.name} className="h-6 w-6 rounded" />
            ) : (
              <Package className="h-5 w-5 text-primary" />
            )}
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium truncate">{ext.name}</span>
              <Badge variant={badge.variant as any} className="text-[10px] shrink-0">
                {badge.label}
              </Badge>
              {!ext.published && ext.moderation_status !== "taken_down" && (
                <Badge variant="secondary" className="text-[10px] shrink-0">Unpublished</Badge>
              )}
            </div>
            <p className="text-xs text-muted-foreground truncate">
              by {ext.publisher?.display_name || "Unknown"} &middot; {ext.total_downloads} downloads
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {ext.moderation_status === "active" && (
            <>
              <Button variant="outline" size="sm" disabled={actioning} onClick={() => onModerate(ext.id, "request_review")}>
                <Eye className="h-3.5 w-3.5 mr-1" />
                Review
              </Button>
              <Button variant="destructive" size="sm" disabled={actioning} onClick={() => onModerate(ext.id, "take_down")}>
                <EyeOff className="h-3.5 w-3.5 mr-1" />
                Take Down
              </Button>
            </>
          )}
          {ext.moderation_status === "under_review" && (
            <>
              <Button variant="default" size="sm" disabled={actioning} onClick={() => onModerate(ext.id, "approve")}>
                <CheckCircle className="h-3.5 w-3.5 mr-1" />
                Approve
              </Button>
              <Button variant="destructive" size="sm" disabled={actioning} onClick={() => onModerate(ext.id, "take_down")}>
                <EyeOff className="h-3.5 w-3.5 mr-1" />
                Take Down
              </Button>
            </>
          )}
          {ext.moderation_status === "taken_down" && (
            <Button variant="default" size="sm" disabled={actioning} onClick={() => onModerate(ext.id, "approve")}>
              <CheckCircle className="h-3.5 w-3.5 mr-1" />
              Restore
            </Button>
          )}
          <button onClick={() => setExpanded(!expanded)} className="text-muted-foreground hover:text-foreground">
            <ChevronDown className={`h-4 w-4 transition-transform ${expanded ? "rotate-180" : ""}`} />
          </button>
        </div>
      </div>

      {expanded && (
        <div className="mt-3 pt-3 border-t border-border text-sm space-y-1">
          <p><span className="text-muted-foreground">ID:</span> <code className="text-xs bg-secondary px-1 rounded">{ext.id}</code></p>
          <p><span className="text-muted-foreground">Publisher:</span> {ext.publisher?.username || "Unknown"}</p>
          <p><span className="text-muted-foreground">Trust Level:</span> {ext.trust_level}</p>
          <p><span className="text-muted-foreground">Rating:</span> {ext.average_rating > 0 ? ext.average_rating.toFixed(1) : "No ratings"}</p>
          <p><span className="text-muted-foreground">Description:</span> {ext.short_description}</p>
          {ext.moderation_note && (
            <p><span className="text-muted-foreground">Mod Note:</span> {ext.moderation_note}</p>
          )}
          {ext.moderated_at && (
            <p><span className="text-muted-foreground">Last Action:</span> {new Date(ext.moderated_at).toLocaleString()}</p>
          )}
        </div>
      )}
    </div>
  );
}
