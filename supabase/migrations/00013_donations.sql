-- ============================================================
-- 00013 · Donations
-- ============================================================

-- donations table
CREATE TABLE public.donations (
  id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       UUID        REFERENCES public.profiles(id) ON DELETE SET NULL,
  stripe_session_id TEXT    UNIQUE NOT NULL,
  amount_cents  INTEGER     NOT NULL CHECK (amount_cents > 0),
  currency      TEXT        NOT NULL DEFAULT 'usd',
  recurring     BOOLEAN     NOT NULL DEFAULT false,
  donor_name    TEXT,
  show_on_list  BOOLEAN     NOT NULL DEFAULT false,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Add donor list visibility preference to profiles
ALTER TABLE public.profiles
  ADD COLUMN show_on_donors_list BOOLEAN NOT NULL DEFAULT false;

-- Indexes
CREATE INDEX idx_donations_created_at ON public.donations(created_at DESC);
CREATE INDEX idx_donations_user_id    ON public.donations(user_id);

-- ── RLS ──────────────────────────────────────────────────────
ALTER TABLE public.donations ENABLE ROW LEVEL SECURITY;

-- Public can read donations where the donor opted in (for recent donors list)
CREATE POLICY "Anyone can read public donations"
  ON public.donations FOR SELECT
  USING (show_on_list = true);

-- Authenticated users can read their own donations regardless of show_on_list
CREATE POLICY "Users can read own donations"
  ON public.donations FOR SELECT
  TO authenticated
  USING (user_id = auth.uid());

-- Only service role can insert (via Stripe webhook) — no user INSERT policy

-- ── Aggregation view (public stats, no PII) ─────────────────
CREATE OR REPLACE VIEW public.donation_stats AS
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
