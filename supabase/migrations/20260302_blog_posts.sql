-- ============================================================
-- Blog Posts — Dynamic blog system for moderators
-- ============================================================

CREATE TABLE IF NOT EXISTS blog_posts (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  author_id uuid NOT NULL REFERENCES auth.users(id),
  slug text NOT NULL UNIQUE,
  title text NOT NULL,
  body text NOT NULL,                         -- Markdown content
  excerpt text,                               -- Short summary for listing cards
  cover_image_url text,                       -- Hero/banner image
  category text NOT NULL DEFAULT 'general',
  tags text[] NOT NULL DEFAULT '{}',
  -- SEO fields
  meta_title text,                            -- Falls back to title if null
  meta_description text,                      -- Falls back to excerpt if null
  og_image_url text,                          -- Falls back to cover_image_url if null
  canonical_url text,
  -- Status
  published boolean NOT NULL DEFAULT false,
  featured boolean NOT NULL DEFAULT false,
  -- Counters
  view_count integer NOT NULL DEFAULT 0,
  read_time_minutes integer NOT NULL DEFAULT 5,
  -- Timestamps
  published_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_blog_posts_published ON blog_posts (published, published_at DESC);
CREATE INDEX IF NOT EXISTS idx_blog_posts_slug ON blog_posts (slug) WHERE published = true;
CREATE INDEX IF NOT EXISTS idx_blog_posts_category ON blog_posts (category) WHERE published = true;

-- RLS
ALTER TABLE blog_posts ENABLE ROW LEVEL SECURITY;

-- Public can read published posts
CREATE POLICY "blog_posts_public_read" ON blog_posts
  FOR SELECT USING (published = true);

-- All writes go through service role (moderator API endpoints)
CREATE POLICY "blog_posts_service_write" ON blog_posts
  FOR ALL USING (false);
