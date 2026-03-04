-- ============================================================
-- 00014 · Fix donation_stats SECURITY DEFINER view
-- ============================================================
-- The donation_stats VIEW implicitly runs as the view owner,
-- bypassing RLS on the donations table (Supabase lint 0010).
--
-- Fix: drop the view and replace it with a SECURITY DEFINER
-- function. Functions with SECURITY DEFINER are the accepted
-- Supabase pattern for intentional, auditable RLS bypass.
-- The function returns aggregate stats (no PII) so this is safe.

DROP VIEW IF EXISTS public.donation_stats;

CREATE OR REPLACE FUNCTION public.get_donation_stats()
RETURNS TABLE (
  total_cents  BIGINT,
  today_cents  BIGINT,
  month_cents  BIGINT,
  year_cents   BIGINT,
  total_count  BIGINT
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = ''
AS $$
  SELECT
    COALESCE(SUM(amount_cents), 0)::BIGINT           AS total_cents,
    COALESCE(SUM(amount_cents) FILTER (
      WHERE created_at >= CURRENT_DATE
    ), 0)::BIGINT                                     AS today_cents,
    COALESCE(SUM(amount_cents) FILTER (
      WHERE created_at >= date_trunc('month', CURRENT_DATE)
    ), 0)::BIGINT                                     AS month_cents,
    COALESCE(SUM(amount_cents) FILTER (
      WHERE created_at >= date_trunc('year', CURRENT_DATE)
    ), 0)::BIGINT                                     AS year_cents,
    COUNT(*)::BIGINT                                   AS total_count
  FROM public.donations;
$$;

-- Allow anon + authenticated to call the function
GRANT EXECUTE ON FUNCTION public.get_donation_stats() TO anon, authenticated;
