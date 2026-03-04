-- ============================================================
-- Fix blog_posts multiple permissive SELECT policies (lint 0006)
-- ============================================================
-- blog_posts_service_write is FOR ALL USING (false) — a no-op that
-- forces PostgreSQL to evaluate an extra policy on every SELECT.
-- Service role bypasses RLS anyway, so this policy does nothing.

DROP POLICY IF EXISTS "blog_posts_service_write" ON blog_posts;
