-- ============================================================
-- Extension Moderation — Schema Additions
-- ============================================================

-- Add moderation columns to extensions table
ALTER TABLE extensions ADD COLUMN IF NOT EXISTS moderation_status text NOT NULL DEFAULT 'active'
  CHECK (moderation_status IN ('active', 'under_review', 'taken_down'));
ALTER TABLE extensions ADD COLUMN IF NOT EXISTS moderation_note text;
ALTER TABLE extensions ADD COLUMN IF NOT EXISTS moderated_by uuid REFERENCES auth.users(id);
ALTER TABLE extensions ADD COLUMN IF NOT EXISTS moderated_at timestamptz;

-- Index for filtering by moderation status in admin views
CREATE INDEX IF NOT EXISTS idx_extensions_moderation ON extensions (moderation_status)
  WHERE moderation_status != 'active';
