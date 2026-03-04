export type Json =
  | string
  | number
  | boolean
  | null
  | { [key: string]: Json | undefined }
  | Json[];

export interface Database {
  public: {
    Tables: {
      profiles: {
        Row: {
          id: string;
          username: string;
          display_name: string;
          avatar_url: string | null;
          bio: string | null;
          website: string | null;
          github_username: string | null;
          verified_publisher: boolean;
          reputation: number;
          follower_count: number;
          following_count: number;
          post_count: number;
          pinned_extension_id: string | null;
          pinned_post_id: string | null;
          is_moderator: boolean;
          show_on_donors_list: boolean;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id: string;
          username: string;
          display_name: string;
          avatar_url?: string | null;
          bio?: string | null;
          website?: string | null;
          github_username?: string | null;
          verified_publisher?: boolean;
          reputation?: number;
          follower_count?: number;
          following_count?: number;
          post_count?: number;
          pinned_extension_id?: string | null;
          pinned_post_id?: string | null;
          is_moderator?: boolean;
          show_on_donors_list?: boolean;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          id?: string;
          username?: string;
          display_name?: string;
          avatar_url?: string | null;
          bio?: string | null;
          website?: string | null;
          github_username?: string | null;
          verified_publisher?: boolean;
          reputation?: number;
          follower_count?: number;
          following_count?: number;
          post_count?: number;
          pinned_extension_id?: string | null;
          pinned_post_id?: string | null;
          is_moderator?: boolean;
          show_on_donors_list?: boolean;
          updated_at?: string;
        };
        Relationships: [];
      };
      extensions: {
        Row: {
          id: string;
          publisher_id: string;
          name: string;
          description: string;
          short_description: string;
          icon_url: string | null;
          banner_url: string | null;
          screenshots: string[];
          homepage: string | null;
          repository: string | null;
          license: string | null;
          categories: string[];
          tags: string[];
          trust_level: "verified" | "community" | "unverified";
          featured: boolean;
          total_downloads: number;
          average_rating: number;
          review_count: number;
          latest_version: string | null;
          published: boolean;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id: string;
          publisher_id: string;
          name: string;
          description: string;
          short_description: string;
          icon_url?: string | null;
          banner_url?: string | null;
          screenshots?: string[];
          homepage?: string | null;
          repository?: string | null;
          license?: string | null;
          categories?: string[];
          tags?: string[];
          trust_level?: "verified" | "community" | "unverified";
          featured?: boolean;
          total_downloads?: number;
          average_rating?: number;
          review_count?: number;
          latest_version?: string | null;
          published?: boolean;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          name?: string;
          description?: string;
          short_description?: string;
          icon_url?: string | null;
          banner_url?: string | null;
          screenshots?: string[];
          homepage?: string | null;
          repository?: string | null;
          license?: string | null;
          categories?: string[];
          tags?: string[];
          trust_level?: "verified" | "community" | "unverified";
          featured?: boolean;
          total_downloads?: number;
          average_rating?: number;
          review_count?: number;
          latest_version?: string | null;
          published?: boolean;
          updated_at?: string;
        };
        Relationships: [];
      };
      extension_versions: {
        Row: {
          id: string;
          extension_id: string;
          version: string;
          changelog: string | null;
          wasm_url: string;
          wasm_size_bytes: number;
          checksum: string;
          source_url: string | null;
          min_omni_version: string | null;
          permissions: Json;
          tools: Json;
          manifest: Json;
          signature: string | null;
          scan_status: "pending" | "scanning" | "passed" | "failed" | "flagged";
          scan_score: number | null;
          scan_completed_at: string | null;
          published: boolean;
          created_at: string;
        };
        Insert: {
          id?: string;
          extension_id: string;
          version: string;
          changelog?: string | null;
          wasm_url: string;
          wasm_size_bytes: number;
          checksum: string;
          source_url?: string | null;
          min_omni_version?: string | null;
          permissions?: Json;
          tools?: Json;
          manifest: Json;
          signature?: string | null;
          scan_status?: "pending" | "scanning" | "passed" | "failed" | "flagged";
          scan_score?: number | null;
          scan_completed_at?: string | null;
          published?: boolean;
          created_at?: string;
        };
        Update: {
          changelog?: string | null;
          source_url?: string | null;
          scan_status?: "pending" | "scanning" | "passed" | "failed" | "flagged";
          scan_score?: number | null;
          scan_completed_at?: string | null;
          published?: boolean;
        };
        Relationships: [];
      };
      reviews: {
        Row: {
          id: string;
          extension_id: string;
          user_id: string;
          rating: number;
          title: string | null;
          body: string | null;
          version: string | null;
          helpful_count: number;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          extension_id: string;
          user_id: string;
          rating: number;
          title?: string | null;
          body?: string | null;
          version?: string | null;
          helpful_count?: number;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          rating?: number;
          title?: string | null;
          body?: string | null;
          helpful_count?: number;
          updated_at?: string;
        };
        Relationships: [];
      };
      downloads: {
        Row: {
          id: string;
          extension_id: string;
          version: string;
          user_id: string | null;
          ip_hash: string | null;
          source: "website" | "cli" | "app";
          created_at: string;
        };
        Insert: {
          id?: string;
          extension_id: string;
          version: string;
          user_id?: string | null;
          ip_hash?: string | null;
          source?: "website" | "cli" | "app";
          created_at?: string;
        };
        Update: never;
        Relationships: [];
      };
      download_stats: {
        Row: {
          extension_id: string;
          date: string;
          count: number;
        };
        Insert: {
          extension_id: string;
          date: string;
          count?: number;
        };
        Update: {
          count?: number;
        };
        Relationships: [];
      };
      scan_results: {
        Row: {
          id: string;
          version_id: string;
          extension_id: string;
          version: string;
          signature_score: number | null;
          signature_matches: Json;
          heuristic_score: number | null;
          heuristic_details: Json;
          ai_score: number | null;
          ai_analysis: string | null;
          ai_flags: Json;
          sandbox_score: number | null;
          sandbox_results: Json;
          overall_score: number;
          verdict: "clean" | "suspicious" | "malicious" | "error";
          auto_approved: boolean;
          manual_reviewer_id: string | null;
          manual_review_notes: string | null;
          scan_duration_ms: number | null;
          created_at: string;
        };
        Insert: {
          id?: string;
          version_id: string;
          extension_id: string;
          version: string;
          signature_score?: number | null;
          signature_matches?: Json;
          heuristic_score?: number | null;
          heuristic_details?: Json;
          ai_score?: number | null;
          ai_analysis?: string | null;
          ai_flags?: Json;
          sandbox_score?: number | null;
          sandbox_results?: Json;
          overall_score: number;
          verdict: "clean" | "suspicious" | "malicious" | "error";
          auto_approved?: boolean;
          manual_reviewer_id?: string | null;
          manual_review_notes?: string | null;
          scan_duration_ms?: number | null;
          created_at?: string;
        };
        Update: {
          manual_reviewer_id?: string | null;
          manual_review_notes?: string | null;
        };
        Relationships: [];
      };
      api_keys: {
        Row: {
          id: string;
          user_id: string;
          name: string;
          key_hash: string;
          key_prefix: string;
          permissions: string[];
          last_used_at: string | null;
          expires_at: string | null;
          revoked: boolean;
          created_at: string;
        };
        Insert: {
          id?: string;
          user_id: string;
          name: string;
          key_hash: string;
          key_prefix: string;
          permissions?: string[];
          last_used_at?: string | null;
          expires_at?: string | null;
          revoked?: boolean;
          created_at?: string;
        };
        Update: {
          name?: string;
          last_used_at?: string | null;
          revoked?: boolean;
        };
        Relationships: [];
      };
      forum_categories: {
        Row: {
          id: string;
          name: string;
          description: string | null;
          icon: string | null;
          sort_order: number;
          post_count: number;
        };
        Insert: {
          id: string;
          name: string;
          description?: string | null;
          icon?: string | null;
          sort_order?: number;
          post_count?: number;
        };
        Update: {
          name?: string;
          description?: string | null;
          icon?: string | null;
          sort_order?: number;
          post_count?: number;
        };
        Relationships: [];
      };
      forum_posts: {
        Row: {
          id: string;
          author_id: string;
          category_id: string | null;
          extension_id: string | null;
          title: string;
          body: string;
          pinned: boolean;
          locked: boolean;
          solved: boolean;
          accepted_reply_id: string | null;
          vote_score: number;
          reply_count: number;
          view_count: number;
          last_activity_at: string;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          author_id: string;
          category_id?: string | null;
          extension_id?: string | null;
          title: string;
          body: string;
          pinned?: boolean;
          locked?: boolean;
          solved?: boolean;
          accepted_reply_id?: string | null;
          vote_score?: number;
          reply_count?: number;
          view_count?: number;
          last_activity_at?: string;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          title?: string;
          body?: string;
          pinned?: boolean;
          locked?: boolean;
          solved?: boolean;
          accepted_reply_id?: string | null;
          vote_score?: number;
          reply_count?: number;
          view_count?: number;
          last_activity_at?: string;
          updated_at?: string;
        };
        Relationships: [];
      };
      forum_replies: {
        Row: {
          id: string;
          post_id: string;
          author_id: string;
          parent_reply_id: string | null;
          body: string;
          is_accepted: boolean;
          vote_score: number;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          post_id: string;
          author_id: string;
          parent_reply_id?: string | null;
          body: string;
          is_accepted?: boolean;
          vote_score?: number;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          body?: string;
          is_accepted?: boolean;
          vote_score?: number;
          updated_at?: string;
        };
        Relationships: [];
      };
      forum_votes: {
        Row: {
          id: string;
          user_id: string;
          post_id: string | null;
          reply_id: string | null;
          value: number;
          created_at: string;
        };
        Insert: {
          id?: string;
          user_id: string;
          post_id?: string | null;
          reply_id?: string | null;
          value: number;
          created_at?: string;
        };
        Update: {
          value?: number;
        };
        Relationships: [];
      };
      user_followers: {
        Row: {
          follower_id: string;
          following_id: string;
          created_at: string;
        };
        Insert: {
          follower_id: string;
          following_id: string;
          created_at?: string;
        };
        Update: never;
        Relationships: [];
      };
      user_badges: {
        Row: {
          id: string;
          user_id: string;
          badge_id: string;
          earned_at: string;
        };
        Insert: {
          id?: string;
          user_id: string;
          badge_id: string;
          earned_at?: string;
        };
        Update: never;
        Relationships: [];
      };
      blog_posts: {
        Row: {
          id: string;
          author_id: string;
          slug: string;
          title: string;
          body: string;
          excerpt: string | null;
          cover_image_url: string | null;
          category: string;
          tags: string[];
          meta_title: string | null;
          meta_description: string | null;
          og_image_url: string | null;
          canonical_url: string | null;
          published: boolean;
          featured: boolean;
          view_count: number;
          read_time_minutes: number;
          published_at: string | null;
          created_at: string;
          updated_at: string;
        };
        Insert: {
          id?: string;
          author_id: string;
          slug: string;
          title: string;
          body: string;
          excerpt?: string | null;
          cover_image_url?: string | null;
          category?: string;
          tags?: string[];
          meta_title?: string | null;
          meta_description?: string | null;
          og_image_url?: string | null;
          canonical_url?: string | null;
          published?: boolean;
          featured?: boolean;
          view_count?: number;
          read_time_minutes?: number;
          published_at?: string | null;
          created_at?: string;
          updated_at?: string;
        };
        Update: {
          slug?: string;
          title?: string;
          body?: string;
          excerpt?: string | null;
          cover_image_url?: string | null;
          category?: string;
          tags?: string[];
          meta_title?: string | null;
          meta_description?: string | null;
          og_image_url?: string | null;
          canonical_url?: string | null;
          published?: boolean;
          featured?: boolean;
          view_count?: number;
          read_time_minutes?: number;
          published_at?: string | null;
          updated_at?: string;
        };
        Relationships: [];
      };
      donations: {
        Row: {
          id: string;
          user_id: string | null;
          stripe_session_id: string;
          amount_cents: number;
          currency: string;
          recurring: boolean;
          donor_name: string | null;
          show_on_list: boolean;
          created_at: string;
        };
        Insert: {
          id?: string;
          user_id?: string | null;
          stripe_session_id: string;
          amount_cents: number;
          currency?: string;
          recurring?: boolean;
          donor_name?: string | null;
          show_on_list?: boolean;
          created_at?: string;
        };
        Update: {
          donor_name?: string | null;
          show_on_list?: boolean;
        };
        Relationships: [];
      };
    };
    Views: {
      [_ in never]: never;
    };
    Functions: {
      [_ in never]: never;
    };
    Enums: {
      [_ in never]: never;
    };
    CompositeTypes: {
      [_ in never]: never;
    };
  };
}

// Convenience type aliases
export type Profile = Database["public"]["Tables"]["profiles"]["Row"];
export type Extension = Database["public"]["Tables"]["extensions"]["Row"];
export type ExtensionVersion = Database["public"]["Tables"]["extension_versions"]["Row"];
export type Review = Database["public"]["Tables"]["reviews"]["Row"];
export type Download = Database["public"]["Tables"]["downloads"]["Row"];
export type DownloadStat = Database["public"]["Tables"]["download_stats"]["Row"];
export type ScanResult = Database["public"]["Tables"]["scan_results"]["Row"];
export type ApiKey = Database["public"]["Tables"]["api_keys"]["Row"];
export type ForumCategory = Database["public"]["Tables"]["forum_categories"]["Row"];
export type ForumPost = Database["public"]["Tables"]["forum_posts"]["Row"];
export type ForumReply = Database["public"]["Tables"]["forum_replies"]["Row"];
export type ForumVote = Database["public"]["Tables"]["forum_votes"]["Row"];
export type UserFollower = Database["public"]["Tables"]["user_followers"]["Row"];
export type UserBadge = Database["public"]["Tables"]["user_badges"]["Row"];
export type Donation = Database["public"]["Tables"]["donations"]["Row"];
export type BlogPost = Database["public"]["Tables"]["blog_posts"]["Row"];

// Join types for common queries
export type ExtensionWithPublisher = Extension & {
  publisher: Profile;
};

export type ExtensionVersionWithScan = ExtensionVersion & {
  scan_results: ScanResult[];
};

export type ForumPostWithAuthor = ForumPost & {
  author: Profile;
};

export type ForumReplyWithAuthor = ForumReply & {
  author: Profile;
};

export type BlogPostWithAuthor = BlogPost & {
  author: Profile;
};

export type ForumPostFull = ForumPost & {
  author: Profile;
  category: ForumCategory | null;
  extension: Pick<Extension, "id" | "name" | "icon_url"> | null;
};
