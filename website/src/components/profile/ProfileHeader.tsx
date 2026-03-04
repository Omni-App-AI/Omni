"use client";

import Link from "next/link";
import { Calendar, Globe, ExternalLink, Users, Shield } from "lucide-react";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { ReputationBadge } from "./ReputationBadge";
import { FollowButton } from "./FollowButton";
import type { Profile, Extension, ForumPost } from "@/lib/supabase/types";

interface ProfileHeaderProps {
  profile: Profile;
  isOwnProfile: boolean;
  isFollowing: boolean;
  pinnedExtension?: Pick<Extension, "id" | "name" | "icon_url" | "short_description"> | null;
  pinnedPost?: Pick<ForumPost, "id" | "title"> | null;
  badges: { badge_id: string; earned_at: string }[];
}

export function ProfileHeader({
  profile,
  isOwnProfile,
  isFollowing,
  pinnedExtension,
  pinnedPost,
  badges,
}: ProfileHeaderProps) {
  return (
    <div className="space-y-6">
      {/* Main info */}
      <div className="flex items-start gap-6">
        <Avatar src={profile.avatar_url} fallback={profile.display_name} size="lg" />
        <div className="flex-1">
          <div className="flex items-center gap-3">
            <h1 className="text-2xl font-bold">{profile.display_name}</h1>
            {profile.is_moderator && (
              <Badge variant="default" className="gap-1">
                <Shield className="h-3 w-3" />
                Mod
              </Badge>
            )}
            {profile.verified_publisher && (
              <Badge variant="success">Verified Publisher</Badge>
            )}
            <ReputationBadge reputation={profile.reputation} />
          </div>
          <p className="text-muted-foreground">@{profile.username}</p>

          {profile.bio && (
            <p className="mt-3 text-sm max-w-xl">{profile.bio}</p>
          )}

          {/* Stats row */}
          <div className="mt-3 flex items-center gap-4 text-sm text-muted-foreground">
            <span className="flex items-center gap-1">
              <Calendar className="h-4 w-4" />
              Joined{" "}
              {new Date(profile.created_at).toLocaleDateString("en-US", {
                month: "long",
                year: "numeric",
              })}
            </span>
            {profile.website && (
              <a
                href={profile.website}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1 hover:text-foreground"
              >
                <Globe className="h-4 w-4" />
                Website
              </a>
            )}
            {profile.github_username && (
              <a
                href={`https://github.com/${profile.github_username}`}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1 hover:text-foreground"
              >
                <ExternalLink className="h-4 w-4" />
                GitHub
              </a>
            )}
          </div>

          {/* Follower / following counts */}
          <div className="mt-3 flex items-center gap-4 text-sm">
            <span className="flex items-center gap-1">
              <Users className="h-4 w-4 text-muted-foreground" />
              <span className="font-medium">{profile.follower_count}</span>
              <span className="text-muted-foreground">followers</span>
            </span>
            <span>
              <span className="font-medium">{profile.following_count}</span>
              <span className="text-muted-foreground"> following</span>
            </span>
            <span>
              <span className="font-medium">{profile.post_count}</span>
              <span className="text-muted-foreground"> posts</span>
            </span>
          </div>

          {/* Follow button */}
          {!isOwnProfile && (
            <div className="mt-4">
              <FollowButton username={profile.username} initialFollowing={isFollowing} />
            </div>
          )}
        </div>
      </div>

      {/* Badges row */}
      {badges.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {badges.slice(0, 6).map((badge) => {
            const BADGE_DEFS: Record<string, { name: string }> = {
              "first-post": { name: "First Post" },
              "first-extension": { name: "First Extension" },
              helpful: { name: "Helpful" },
              popular: { name: "Popular" },
              contributor: { name: "Contributor" },
              veteran: { name: "Veteran" },
              trusted: { name: "Trusted" },
              "top-reviewer": { name: "Top Reviewer" },
              donor: { name: "Donor" },
            };
            const def = BADGE_DEFS[badge.badge_id];
            if (!def) return null;

            return (
              <span
                key={badge.badge_id}
                className="inline-flex items-center px-2 py-0.5 rounded-full bg-primary/10 text-primary text-[10px] font-medium"
              >
                {def.name}
              </span>
            );
          })}
        </div>
      )}

      {/* Pinned items */}
      {(pinnedExtension || pinnedPost) && (
        <div className="flex flex-wrap gap-3">
          {pinnedExtension && (
            <Link
              href={`/extensions/${pinnedExtension.id}`}
              className="flex items-center gap-2 px-3 py-2 border border-primary/30 rounded-lg bg-primary/5 hover:bg-primary/10 transition-colors text-sm"
            >
              <span className="text-[10px] text-primary font-medium uppercase">Pinned</span>
              <span>{pinnedExtension.name}</span>
            </Link>
          )}
          {pinnedPost && (
            <Link
              href={`/community/post/${pinnedPost.id}`}
              className="flex items-center gap-2 px-3 py-2 border border-primary/30 rounded-lg bg-primary/5 hover:bg-primary/10 transition-colors text-sm"
            >
              <span className="text-[10px] text-primary font-medium uppercase">Pinned</span>
              <span className="truncate max-w-[200px]">{pinnedPost.title}</span>
            </Link>
          )}
        </div>
      )}
    </div>
  );
}
