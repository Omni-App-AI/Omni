-- ============================================================
-- Fix blog_posts FK: add relationship to profiles for PostgREST joins
-- ============================================================
-- PostgREST (Supabase) needs a FK to profiles for the
-- `author:profiles(...)` join syntax to work.

ALTER TABLE blog_posts
  ADD CONSTRAINT blog_posts_author_profile_fk
  FOREIGN KEY (author_id) REFERENCES profiles(id);
