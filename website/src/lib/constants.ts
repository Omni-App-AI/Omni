export const CATEGORIES = [
  { id: "ai", name: "AI & Machine Learning", icon: "Brain" },
  { id: "automation", name: "Automation", icon: "Zap" },
  { id: "communication", name: "Communication", icon: "MessageSquare" },
  { id: "data", name: "Data & Analytics", icon: "BarChart3" },
  { id: "developer-tools", name: "Developer Tools", icon: "Code2" },
  { id: "education", name: "Education", icon: "GraduationCap" },
  { id: "finance", name: "Finance", icon: "DollarSign" },
  { id: "media", name: "Media & Content", icon: "Image" },
  { id: "productivity", name: "Productivity", icon: "CheckSquare" },
  { id: "security", name: "Security", icon: "Shield" },
  { id: "social", name: "Social", icon: "Users" },
  { id: "utilities", name: "Utilities", icon: "Wrench" },
  { id: "weather", name: "Weather", icon: "Cloud" },
  { id: "web", name: "Web & APIs", icon: "Globe" },
] as const;

export type CategoryId = (typeof CATEGORIES)[number]["id"];

export const TRUST_LEVELS = {
  verified: { label: "Verified", color: "text-success", bgColor: "bg-success/10" },
  community: { label: "Community", color: "text-blue-400", bgColor: "bg-blue-400/10" },
  unverified: { label: "Unverified", color: "text-muted-foreground", bgColor: "bg-muted" },
} as const;

export type TrustLevel = keyof typeof TRUST_LEVELS;

export const SCAN_VERDICTS = {
  clean: { label: "Clean", color: "text-success", icon: "ShieldCheck" },
  suspicious: { label: "Suspicious", color: "text-warning", icon: "AlertTriangle" },
  malicious: { label: "Malicious", color: "text-destructive", icon: "ShieldX" },
  error: { label: "Error", color: "text-muted-foreground", icon: "AlertCircle" },
} as const;

export const SORT_OPTIONS = [
  { value: "downloads", label: "Most Downloads" },
  { value: "rating", label: "Highest Rated" },
  { value: "newest", label: "Newest" },
  { value: "updated", label: "Recently Updated" },
  { value: "name", label: "Name (A-Z)" },
] as const;

export const PERMISSIONS_DISPLAY: Record<string, { label: string; severity: "low" | "medium" | "high" }> = {
  "network.http": { label: "HTTP Network Access", severity: "medium" },
  "network.websocket": { label: "WebSocket Access", severity: "medium" },
  "filesystem.read": { label: "File System Read", severity: "high" },
  "filesystem.write": { label: "File System Write", severity: "high" },
  "clipboard.read": { label: "Clipboard Read", severity: "medium" },
  "clipboard.write": { label: "Clipboard Write", severity: "low" },
  "system.notifications": { label: "Notifications", severity: "low" },
  "system.scheduling": { label: "Task Scheduling", severity: "low" },
  "ai.inference": { label: "AI Inference", severity: "medium" },
  "channel.send": { label: "Channel Messaging", severity: "high" },
  "browser.scrape": { label: "Web Scraping", severity: "high" },
  "storage.persistent": { label: "Persistent Storage", severity: "low" },
};

// Forum categories (general community discussions)
export const FORUM_CATEGORIES = [
  { id: "announcements", name: "Announcements", description: "Official updates from the Omni team", icon: "Megaphone" },
  { id: "help", name: "Help & Support", description: "Get help with Omni, extensions, and the SDK", icon: "HelpCircle" },
  { id: "showcase", name: "Showcase", description: "Share what you've built with Omni", icon: "Sparkles" },
  { id: "feature-requests", name: "Feature Requests", description: "Suggest and vote on new features", icon: "Lightbulb" },
  { id: "extensions", name: "Extension Development", description: "Discuss building and debugging extensions", icon: "Code2" },
  { id: "general", name: "General Discussion", description: "Chat about anything Omni-related", icon: "MessageCircle" },
] as const;

export type ForumCategoryId = (typeof FORUM_CATEGORIES)[number]["id"];

// Badge definitions
export const BADGE_DEFINITIONS: Record<string, { name: string; description: string; icon: string }> = {
  "first-post":       { name: "First Post",       description: "Created your first forum post",          icon: "MessageSquare" },
  "first-extension":  { name: "First Extension",   description: "Published your first extension",         icon: "Package" },
  "helpful":          { name: "Helpful",            description: "Had 5 replies accepted as answers",      icon: "CheckCircle" },
  "popular":          { name: "Popular",            description: "Received 25 upvotes on a single post",   icon: "TrendingUp" },
  "contributor":      { name: "Contributor",        description: "Published 5 extensions",                 icon: "Star" },
  "veteran":          { name: "Veteran",            description: "Member for over 1 year",                 icon: "Award" },
  "trusted":          { name: "Trusted",            description: "Reached 500 reputation points",          icon: "Shield" },
  "top-reviewer":     { name: "Top Reviewer",       description: "Written 10 extension reviews",           icon: "Eye" },
  "donor":            { name: "Donor",              description: "Supported Omni with a donation",         icon: "Heart" },
};

// Reputation points per action
export const REPUTATION_ACTIONS = {
  post_upvoted: 5,
  reply_upvoted: 10,
  answer_accepted: 15,
  extension_published: 20,
  review_written: 2,
  post_downvoted: -2,
  reply_downvoted: -2,
} as const;

// Reputation display tiers
export const REPUTATION_TIERS = [
  { min: 1000, label: "Expert",      color: "text-yellow-400", bgColor: "bg-yellow-400/10" },
  { min: 500,  label: "Trusted",     color: "text-purple-400", bgColor: "bg-purple-400/10" },
  { min: 200,  label: "Contributor", color: "text-green-400",  bgColor: "bg-green-400/10" },
  { min: 50,   label: "Member",      color: "text-blue-400",   bgColor: "bg-blue-400/10" },
  { min: 0,    label: "Newcomer",    color: "text-muted-foreground", bgColor: "bg-muted" },
] as const;

// Forum post sort options
export const POST_SORT_OPTIONS = [
  { value: "newest", label: "Newest" },
  { value: "votes", label: "Most Votes" },
  { value: "activity", label: "Recent Activity" },
  { value: "unanswered", label: "Unanswered" },
] as const;

// Anti-bot: Trust tier capabilities
export const TRUST_CAPABILITIES = {
  newcomer: {
    max_links_per_post: 0,
    max_links_per_reply: 0,
    turnstile_required: true,
    max_posts_per_day: 3,
    max_replies_per_day: 10,
  },
  member: {
    max_links_per_post: 2,
    max_links_per_reply: 1,
    turnstile_required: false,
    max_posts_per_day: 10,
    max_replies_per_day: 30,
  },
  contributor: {
    max_links_per_post: 5,
    max_links_per_reply: 3,
    turnstile_required: false,
    max_posts_per_day: 30,
    max_replies_per_day: 100,
  },
  trusted: {
    max_links_per_post: 10,
    max_links_per_reply: 5,
    turnstile_required: false,
    max_posts_per_day: 100,
    max_replies_per_day: 500,
  },
  expert: {
    max_links_per_post: -1,
    max_links_per_reply: -1,
    turnstile_required: false,
    max_posts_per_day: -1,
    max_replies_per_day: -1,
  },
} as const;

export type TrustTierName = keyof typeof TRUST_CAPABILITIES;
