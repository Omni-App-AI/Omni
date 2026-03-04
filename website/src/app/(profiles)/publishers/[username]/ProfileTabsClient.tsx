"use client";

import { useState } from "react";
import Link from "next/link";
import { Star, CheckCircle2, MessageSquare } from "lucide-react";
import { ProfileTabs } from "@/components/profile/ProfileTabs";
import { ActivityFeed } from "@/components/profile/ActivityFeed";
import { BadgeGrid } from "@/components/profile/BadgeGrid";
import { ExtensionGrid } from "@/components/marketplace/ExtensionGrid";
import { Badge } from "@/components/ui/badge";
import { Avatar } from "@/components/ui/avatar";
import { timeAgo } from "@/lib/utils";
import type { ExtensionWithPublisher } from "@/lib/supabase/types";

interface ProfileTabsClientProps {
  extensions: ExtensionWithPublisher[];
  posts: any[];
  reviews: any[];
  badges: { badge_id: string; earned_at: string }[];
  activityItems: {
    type: "post" | "review" | "extension";
    id: string;
    title: string;
    href: string;
    date: string;
    meta?: string;
  }[];
  extensionCount: number;
  postCount: number;
  reviewCount: number;
}

export function ProfileTabsClient({
  extensions,
  posts,
  reviews,
  badges,
  activityItems,
  extensionCount,
  postCount,
  reviewCount,
}: ProfileTabsClientProps) {
  const [activeTab, setActiveTab] = useState("overview");

  return (
    <div>
      <ProfileTabs
        activeTab={activeTab}
        onTabChange={setActiveTab}
        extensionCount={extensionCount}
        postCount={postCount}
        reviewCount={reviewCount}
      />

      <div className="mt-6">
        {/* Overview Tab */}
        {activeTab === "overview" && (
          <div className="grid lg:grid-cols-3 gap-8">
            <div className="lg:col-span-2">
              <h3 className="text-sm font-medium mb-4">Recent Activity</h3>
              <ActivityFeed items={activityItems.slice(0, 15)} />
            </div>
            <div>
              <h3 className="text-sm font-medium mb-4">Badges</h3>
              <BadgeGrid badges={badges} />
            </div>
          </div>
        )}

        {/* Extensions Tab */}
        {activeTab === "extensions" && (
          <ExtensionGrid
            extensions={extensions}
            emptyMessage="No extensions published yet."
          />
        )}

        {/* Posts Tab */}
        {activeTab === "posts" && (
          <div className="space-y-0">
            {posts.length === 0 ? (
              <p className="text-sm text-muted-foreground py-4">No forum posts yet.</p>
            ) : (
              posts.map((post) => (
                <div
                  key={post.id}
                  className="flex items-start gap-4 py-3 border-b border-border/30 last:border-0"
                >
                  <div className="flex flex-col items-center gap-0.5 w-10 shrink-0 pt-0.5">
                    <span className={`text-sm font-medium tabular-nums ${post.vote_score > 0 ? "text-primary" : "text-muted-foreground"}`}>
                      {post.vote_score}
                    </span>
                    <span className="text-[9px] text-muted-foreground/60">votes</span>
                  </div>
                  <div className="flex-1 min-w-0">
                    <Link
                      href={`/community/post/${post.id}`}
                      className="text-sm font-medium hover:text-primary transition-colors line-clamp-1"
                    >
                      {post.title}
                    </Link>
                    <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
                      {post.category && (
                        <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                          {post.category.name}
                        </Badge>
                      )}
                      {post.extension && (
                        <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                          {post.extension.name}
                        </Badge>
                      )}
                      {post.solved && (
                        <span className="flex items-center gap-0.5 text-success">
                          <CheckCircle2 className="h-3 w-3" />
                          Solved
                        </span>
                      )}
                      <span className="flex items-center gap-0.5">
                        <MessageSquare className="h-3 w-3" />
                        {post.reply_count}
                      </span>
                      <span>{timeAgo(post.created_at)}</span>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {/* Reviews Tab */}
        {activeTab === "reviews" && (
          <div className="space-y-0">
            {reviews.length === 0 ? (
              <p className="text-sm text-muted-foreground py-4">No reviews written yet.</p>
            ) : (
              reviews.map((review) => (
                <div
                  key={review.id}
                  className="flex items-start gap-4 py-4 border-b border-border/30 last:border-0"
                >
                  {review.extension?.icon_url && (
                    <img
                      src={review.extension.icon_url}
                      alt=""
                      className="h-8 w-8 rounded-lg shrink-0"
                    />
                  )}
                  <div className="flex-1 min-w-0">
                    <Link
                      href={`/extensions/${review.extension?.id}`}
                      className="text-sm font-medium hover:text-primary transition-colors"
                    >
                      {review.extension?.name}
                    </Link>
                    <div className="flex items-center gap-2 mt-0.5">
                      <div className="flex gap-0.5">
                        {[1, 2, 3, 4, 5].map((s) => (
                          <Star
                            key={s}
                            className={`h-3.5 w-3.5 ${s <= review.rating ? "fill-warning text-warning" : "text-muted-foreground/30"}`}
                          />
                        ))}
                      </div>
                      <span className="text-xs text-muted-foreground">
                        {timeAgo(review.created_at)}
                      </span>
                    </div>
                    {review.title && (
                      <p className="text-sm font-medium mt-1">{review.title}</p>
                    )}
                    {review.body && (
                      <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
                        {review.body}
                      </p>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  );
}
