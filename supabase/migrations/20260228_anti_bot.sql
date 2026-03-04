-- ============================================================
-- Anti-Bot Defense System — Database Schema
-- ============================================================

-- ============================================================
-- 1. Rate Limits (sliding window tracking)
-- ============================================================
CREATE TABLE IF NOT EXISTS rate_limits (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  key text NOT NULL,
  action text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_rate_limits_lookup ON rate_limits (key, action, created_at DESC);

-- ============================================================
-- 2. Content Flags (user reports + auto-flags)
-- ============================================================
DO $$ BEGIN
  CREATE TYPE flag_status AS ENUM ('pending', 'reviewed', 'actioned', 'dismissed');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
  CREATE TYPE flag_reason AS ENUM ('spam', 'harassment', 'misinformation', 'off_topic', 'malicious', 'other', 'auto_spam', 'auto_suspicious');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS content_flags (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  content_type text NOT NULL CHECK (content_type IN ('post', 'reply', 'review', 'extension')),
  content_id text NOT NULL,
  reporter_id uuid REFERENCES auth.users(id),
  reason flag_reason NOT NULL,
  details text,
  spam_score integer,
  status flag_status NOT NULL DEFAULT 'pending',
  moderator_id uuid REFERENCES auth.users(id),
  moderator_note text,
  resolved_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_flags_status ON content_flags (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_flags_content ON content_flags (content_type, content_id);

-- ============================================================
-- 3. Security Events (audit log)
-- ============================================================
CREATE TABLE IF NOT EXISTS security_events (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  event_type text NOT NULL,
  actor_id uuid REFERENCES auth.users(id),
  ip_address text,
  ip_hash text,
  user_agent text,
  metadata jsonb DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_security_events_type_time ON security_events (event_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_security_events_actor ON security_events (actor_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_security_events_ip ON security_events (ip_hash, created_at DESC);

-- ============================================================
-- 4. IP Reputation (tracked IPs with risk scores)
-- ============================================================
CREATE TABLE IF NOT EXISTS ip_reputation (
  ip_hash text PRIMARY KEY,
  risk_score integer NOT NULL DEFAULT 0,
  total_flags integer NOT NULL DEFAULT 0,
  total_rate_limits integer NOT NULL DEFAULT 0,
  total_honeypots integer NOT NULL DEFAULT 0,
  total_turnstile_fails integer NOT NULL DEFAULT 0,
  blocked boolean NOT NULL DEFAULT false,
  first_seen_at timestamptz NOT NULL DEFAULT now(),
  last_seen_at timestamptz NOT NULL DEFAULT now(),
  notes text
);

-- ============================================================
-- 5. User Bans (temporary + permanent + shadow)
-- ============================================================
DO $$ BEGIN
  CREATE TYPE ban_type AS ENUM ('temporary', 'permanent', 'shadow');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS user_bans (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id uuid NOT NULL REFERENCES auth.users(id),
  ban_type ban_type NOT NULL,
  reason text NOT NULL,
  banned_by uuid REFERENCES auth.users(id),
  expires_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  revoked_by uuid REFERENCES auth.users(id)
);
CREATE INDEX IF NOT EXISTS idx_user_bans_active ON user_bans (user_id) WHERE revoked_at IS NULL;

-- ============================================================
-- 6. RLS Policies
-- ============================================================
ALTER TABLE rate_limits ENABLE ROW LEVEL SECURITY;
ALTER TABLE content_flags ENABLE ROW LEVEL SECURITY;
ALTER TABLE security_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE ip_reputation ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_bans ENABLE ROW LEVEL SECURITY;

-- rate_limits: service role only
CREATE POLICY "rate_limits_service_only" ON rate_limits FOR ALL USING (false);

-- content_flags: users can create and read own flags
CREATE POLICY "flags_insert_own" ON content_flags
  FOR INSERT WITH CHECK (auth.uid() = reporter_id);
CREATE POLICY "flags_select_own" ON content_flags
  FOR SELECT USING (auth.uid() = reporter_id);

-- security_events: service role only
CREATE POLICY "security_events_service_only" ON security_events FOR ALL USING (false);

-- ip_reputation: service role only
CREATE POLICY "ip_reputation_service_only" ON ip_reputation FOR ALL USING (false);

-- user_bans: service role only
CREATE POLICY "user_bans_service_only" ON user_bans FOR ALL USING (false);

-- ============================================================
-- 7. Cleanup function (call via pg_cron or scheduled edge fn)
-- ============================================================
CREATE OR REPLACE FUNCTION cleanup_rate_limits() RETURNS void AS $$
BEGIN
  DELETE FROM rate_limits WHERE created_at < now() - interval '24 hours';
  DELETE FROM security_events WHERE created_at < now() - interval '90 days';
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================================
-- 8. Profile moderation columns
-- ============================================================
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS is_moderator boolean NOT NULL DEFAULT false;
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS is_banned boolean NOT NULL DEFAULT false;
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS ban_reason text;

-- ============================================================
-- 9. Shadow ban support on content tables
-- ============================================================
ALTER TABLE forum_posts ADD COLUMN IF NOT EXISTS shadow_hidden boolean NOT NULL DEFAULT false;
ALTER TABLE forum_replies ADD COLUMN IF NOT EXISTS shadow_hidden boolean NOT NULL DEFAULT false;
ALTER TABLE reviews ADD COLUMN IF NOT EXISTS shadow_hidden boolean NOT NULL DEFAULT false;
