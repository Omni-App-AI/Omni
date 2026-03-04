-- Forum & Enhanced Profile System Migration
-- Creates: forum_categories, forum_posts, forum_replies, forum_votes, user_followers, user_badges
-- Modifies: profiles (adds reputation, follower/following counts, pinned items, post_count)

-- ============================================================
-- 1. Profile Enhancements — add new columns to existing profiles table
-- ============================================================
ALTER TABLE profiles
  ADD COLUMN IF NOT EXISTS reputation       int DEFAULT 0,
  ADD COLUMN IF NOT EXISTS follower_count   int DEFAULT 0,
  ADD COLUMN IF NOT EXISTS following_count  int DEFAULT 0,
  ADD COLUMN IF NOT EXISTS post_count       int DEFAULT 0,
  ADD COLUMN IF NOT EXISTS pinned_extension_id text,
  ADD COLUMN IF NOT EXISTS pinned_post_id      uuid;

-- ============================================================
-- 2. forum_categories — Predefined forum categories
-- ============================================================
CREATE TABLE IF NOT EXISTS forum_categories (
  id          text PRIMARY KEY,
  name        text NOT NULL,
  description text,
  icon        text,
  sort_order  int DEFAULT 0,
  post_count  int DEFAULT 0
);

-- Seed default categories
INSERT INTO forum_categories (id, name, description, icon, sort_order) VALUES
  ('announcements',    'Announcements',        'Official updates from the Omni team',                     'Megaphone',      0),
  ('help',             'Help & Support',        'Get help with Omni, extensions, and the SDK',             'HelpCircle',     1),
  ('showcase',         'Showcase',              'Share what you''ve built with Omni',                      'Sparkles',       2),
  ('feature-requests', 'Feature Requests',      'Suggest and vote on new features',                        'Lightbulb',      3),
  ('extensions',       'Extension Development', 'Discuss building and debugging extensions',               'Code2',          4),
  ('general',          'General Discussion',    'Chat about anything Omni-related',                        'MessageCircle',  5)
ON CONFLICT (id) DO NOTHING;

-- ============================================================
-- 3. forum_posts — Top-level threads
-- ============================================================
CREATE TABLE IF NOT EXISTS forum_posts (
  id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  author_id         uuid NOT NULL REFERENCES profiles(id),
  category_id       text REFERENCES forum_categories(id),
  extension_id      text REFERENCES extensions(id),
  title             text NOT NULL,
  body              text NOT NULL,
  pinned            boolean DEFAULT false,
  locked            boolean DEFAULT false,
  solved            boolean DEFAULT false,
  accepted_reply_id uuid,
  vote_score        int DEFAULT 0,
  reply_count       int DEFAULT 0,
  view_count        int DEFAULT 0,
  last_activity_at  timestamptz DEFAULT now(),
  created_at        timestamptz DEFAULT now(),
  updated_at        timestamptz DEFAULT now(),

  -- Either category_id OR extension_id must be non-null (but not both)
  CONSTRAINT forum_posts_scope_check CHECK (
    (category_id IS NOT NULL AND extension_id IS NULL) OR
    (category_id IS NULL AND extension_id IS NOT NULL)
  )
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_forum_posts_category ON forum_posts(category_id);
CREATE INDEX IF NOT EXISTS idx_forum_posts_extension ON forum_posts(extension_id);
CREATE INDEX IF NOT EXISTS idx_forum_posts_author ON forum_posts(author_id);
CREATE INDEX IF NOT EXISTS idx_forum_posts_last_activity ON forum_posts(last_activity_at DESC);
CREATE INDEX IF NOT EXISTS idx_forum_posts_vote_score ON forum_posts(vote_score DESC);
CREATE INDEX IF NOT EXISTS idx_forum_posts_created ON forum_posts(created_at DESC);

-- ============================================================
-- 4. forum_replies — Replies to posts
-- ============================================================
CREATE TABLE IF NOT EXISTS forum_replies (
  id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  post_id          uuid NOT NULL REFERENCES forum_posts(id) ON DELETE CASCADE,
  author_id        uuid NOT NULL REFERENCES profiles(id),
  parent_reply_id  uuid REFERENCES forum_replies(id),
  body             text NOT NULL,
  is_accepted      boolean DEFAULT false,
  vote_score       int DEFAULT 0,
  created_at       timestamptz DEFAULT now(),
  updated_at       timestamptz DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_forum_replies_post ON forum_replies(post_id);
CREATE INDEX IF NOT EXISTS idx_forum_replies_author ON forum_replies(author_id);
CREATE INDEX IF NOT EXISTS idx_forum_replies_parent ON forum_replies(parent_reply_id);

-- Add FK for accepted_reply_id now that forum_replies exists
ALTER TABLE forum_posts
  ADD CONSTRAINT fk_forum_posts_accepted_reply
  FOREIGN KEY (accepted_reply_id) REFERENCES forum_replies(id)
  ON DELETE SET NULL;

-- ============================================================
-- 5. forum_votes — Upvotes/downvotes on posts and replies
-- ============================================================
CREATE TABLE IF NOT EXISTS forum_votes (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id     uuid NOT NULL REFERENCES profiles(id),
  post_id     uuid REFERENCES forum_posts(id) ON DELETE CASCADE,
  reply_id    uuid REFERENCES forum_replies(id) ON DELETE CASCADE,
  value       int NOT NULL CHECK (value IN (1, -1)),
  created_at  timestamptz DEFAULT now(),

  -- Exactly one of post_id or reply_id must be non-null
  CONSTRAINT forum_votes_target_check CHECK (
    (post_id IS NOT NULL AND reply_id IS NULL) OR
    (post_id IS NULL AND reply_id IS NOT NULL)
  )
);

-- One vote per user per post, one vote per user per reply
CREATE UNIQUE INDEX IF NOT EXISTS idx_forum_votes_user_post
  ON forum_votes(user_id, post_id) WHERE post_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_forum_votes_user_reply
  ON forum_votes(user_id, reply_id) WHERE reply_id IS NOT NULL;

-- ============================================================
-- 6. user_followers — Follow relationships
-- ============================================================
CREATE TABLE IF NOT EXISTS user_followers (
  follower_id   uuid NOT NULL REFERENCES profiles(id),
  following_id  uuid NOT NULL REFERENCES profiles(id),
  created_at    timestamptz DEFAULT now(),

  PRIMARY KEY (follower_id, following_id),
  CONSTRAINT user_followers_no_self_follow CHECK (follower_id != following_id)
);

CREATE INDEX IF NOT EXISTS idx_user_followers_following ON user_followers(following_id);

-- ============================================================
-- 7. user_badges — Earned badges per user
-- ============================================================
CREATE TABLE IF NOT EXISTS user_badges (
  id        uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id   uuid NOT NULL REFERENCES profiles(id),
  badge_id  text NOT NULL,
  earned_at timestamptz DEFAULT now(),

  CONSTRAINT user_badges_unique UNIQUE (user_id, badge_id)
);

CREATE INDEX IF NOT EXISTS idx_user_badges_user ON user_badges(user_id);

-- ============================================================
-- 8. Row Level Security (RLS)
-- ============================================================

-- forum_categories: public read, admin-only write
ALTER TABLE forum_categories ENABLE ROW LEVEL SECURITY;
CREATE POLICY "forum_categories_read" ON forum_categories FOR SELECT USING (true);

-- forum_posts: public read, authenticated create, author edit/delete
ALTER TABLE forum_posts ENABLE ROW LEVEL SECURITY;
CREATE POLICY "forum_posts_read" ON forum_posts FOR SELECT USING (true);
CREATE POLICY "forum_posts_insert" ON forum_posts FOR INSERT WITH CHECK (auth.uid() = author_id);
CREATE POLICY "forum_posts_update" ON forum_posts FOR UPDATE USING (auth.uid() = author_id);
CREATE POLICY "forum_posts_delete" ON forum_posts FOR DELETE USING (auth.uid() = author_id);

-- forum_replies: public read, authenticated create, author edit/delete
ALTER TABLE forum_replies ENABLE ROW LEVEL SECURITY;
CREATE POLICY "forum_replies_read" ON forum_replies FOR SELECT USING (true);
CREATE POLICY "forum_replies_insert" ON forum_replies FOR INSERT WITH CHECK (auth.uid() = author_id);
CREATE POLICY "forum_replies_update" ON forum_replies FOR UPDATE USING (auth.uid() = author_id);
CREATE POLICY "forum_replies_delete" ON forum_replies FOR DELETE USING (auth.uid() = author_id);

-- forum_votes: public read, authenticated create/delete own votes
ALTER TABLE forum_votes ENABLE ROW LEVEL SECURITY;
CREATE POLICY "forum_votes_read" ON forum_votes FOR SELECT USING (true);
CREATE POLICY "forum_votes_insert" ON forum_votes FOR INSERT WITH CHECK (auth.uid() = user_id);
CREATE POLICY "forum_votes_update" ON forum_votes FOR UPDATE USING (auth.uid() = user_id);
CREATE POLICY "forum_votes_delete" ON forum_votes FOR DELETE USING (auth.uid() = user_id);

-- user_followers: public read, authenticated follow/unfollow
ALTER TABLE user_followers ENABLE ROW LEVEL SECURITY;
CREATE POLICY "user_followers_read" ON user_followers FOR SELECT USING (true);
CREATE POLICY "user_followers_insert" ON user_followers FOR INSERT WITH CHECK (auth.uid() = follower_id);
CREATE POLICY "user_followers_delete" ON user_followers FOR DELETE USING (auth.uid() = follower_id);

-- user_badges: public read (earned via server-side logic only)
ALTER TABLE user_badges ENABLE ROW LEVEL SECURITY;
CREATE POLICY "user_badges_read" ON user_badges FOR SELECT USING (true);

-- ============================================================
-- 9. Service role bypass for denormalized counter updates
-- ============================================================
-- The API routes use createServiceClient() (service role key) which
-- bypasses RLS, allowing server-side updates to denormalized counters
-- like vote_score, reply_count, view_count, follower_count, reputation, etc.
