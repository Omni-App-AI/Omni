-- ============================================================
-- 00015 · RLS performance fixes (lint 0003 + 0006)
-- ============================================================

-- ── donations: fix auth_rls_initplan + multiple_permissive_policies ──

-- Drop existing policies
DROP POLICY IF EXISTS "Anyone can read public donations" ON public.donations;
DROP POLICY IF EXISTS "Users can read own donations" ON public.donations;

-- Anon: can only see donations where donor opted in
CREATE POLICY "Anon can read public donations"
  ON public.donations FOR SELECT
  TO anon
  USING (show_on_list = true);

-- Authenticated: can see public donations OR their own
-- Uses (select auth.uid()) to evaluate once per query, not per row
CREATE POLICY "Authenticated can read donations"
  ON public.donations FOR SELECT
  TO authenticated
  USING (show_on_list = true OR user_id = (select auth.uid()));
