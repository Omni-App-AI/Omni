"use client";

import { useState } from "react";
import { UserPlus, UserMinus, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";

interface FollowButtonProps {
  username: string;
  initialFollowing: boolean;
}

export function FollowButton({ username, initialFollowing }: FollowButtonProps) {
  const [following, setFollowing] = useState(initialFollowing);
  const [loading, setLoading] = useState(false);

  const handleToggle = async () => {
    setLoading(true);
    const prev = following;
    setFollowing(!following); // Optimistic

    try {
      const res = await fetch(`/api/v1/users/${username}/follow`, {
        method: following ? "DELETE" : "POST",
      });

      if (!res.ok) {
        setFollowing(prev); // Revert
      }
    } catch {
      setFollowing(prev); // Revert
    } finally {
      setLoading(false);
    }
  };

  return (
    <Button
      size="sm"
      variant={following ? "outline" : "default"}
      onClick={handleToggle}
      disabled={loading}
      className="gap-1.5"
    >
      {loading ? (
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
      ) : following ? (
        <UserMinus className="h-3.5 w-3.5" />
      ) : (
        <UserPlus className="h-3.5 w-3.5" />
      )}
      {following ? "Unfollow" : "Follow"}
    </Button>
  );
}
