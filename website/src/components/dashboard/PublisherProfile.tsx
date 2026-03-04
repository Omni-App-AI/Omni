"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Loader2, Check } from "lucide-react";
import { createClient } from "@/lib/supabase/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Avatar } from "@/components/ui/avatar";
import type { Profile } from "@/lib/supabase/types";

interface PublisherProfileProps {
  profile: Profile;
}

export function PublisherProfile({ profile }: PublisherProfileProps) {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);
  const [form, setForm] = useState({
    display_name: profile.display_name,
    bio: profile.bio || "",
    website: profile.website || "",
    github_username: profile.github_username || "",
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setSuccess(false);

    const supabase = createClient();
    const profileUpdate = { display_name: form.display_name, bio: form.bio || null, website: form.website || null, github_username: form.github_username || null };
    // @ts-expect-error -- Supabase type inference limitation with manual Database type
    await supabase.from("profiles").update(profileUpdate).eq("id", profile.id);

    setLoading(false);
    setSuccess(true);
    router.refresh();
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-8">
      {/* Profile header */}
      <div className="flex items-center gap-4 pb-6 border-b border-border/50">
        <Avatar src={profile.avatar_url} fallback={profile.display_name} size="lg" />
        <div>
          <p className="text-sm font-medium">{profile.display_name}</p>
          <p className="text-xs text-muted-foreground font-mono">@{profile.username}</p>
        </div>
      </div>

      {/* Form fields */}
      <div className="space-y-5">
        <h2 className="text-lg font-semibold">Public Profile</h2>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">Display Name</label>
            <Input
              value={form.display_name}
              onChange={(e) => setForm((f) => ({ ...f, display_name: e.target.value }))}
              required
            />
          </div>
          <div>
            <label className="text-[13px] font-medium mb-1.5 block">GitHub Username</label>
            <Input
              value={form.github_username}
              onChange={(e) => setForm((f) => ({ ...f, github_username: e.target.value }))}
              placeholder="octocat"
            />
          </div>
        </div>

        <div>
          <label className="text-[13px] font-medium mb-1.5 block">Bio</label>
          <Textarea
            value={form.bio}
            onChange={(e) => setForm((f) => ({ ...f, bio: e.target.value }))}
            placeholder="Tell others about yourself..."
            rows={3}
          />
        </div>

        <div>
          <label className="text-[13px] font-medium mb-1.5 block">Website</label>
          <Input
            type="url"
            value={form.website}
            onChange={(e) => setForm((f) => ({ ...f, website: e.target.value }))}
            placeholder="https://..."
          />
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-3 pt-2">
        <Button type="submit" disabled={loading} size="sm" className="gap-2">
          {loading ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Saving...
            </>
          ) : (
            "Save Changes"
          )}
        </Button>
        {success && (
          <span className="text-[13px] text-success flex items-center gap-1.5">
            <Check className="h-3.5 w-3.5" />
            Profile updated
          </span>
        )}
      </div>
    </form>
  );
}
