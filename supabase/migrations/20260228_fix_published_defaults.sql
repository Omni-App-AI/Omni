-- ============================================================
-- Fix Published Defaults
-- ============================================================
-- The publish API was not setting published=true on extension
-- and version inserts, so existing data needs to be fixed.
-- Also update DB defaults so future rows default to true
-- (the API now explicitly sets published=true anyway).

-- Fix existing extensions: mark all as published
UPDATE extensions SET published = true WHERE published = false;

-- Fix existing versions: mark all as published
UPDATE extension_versions SET published = true WHERE published = false;

-- Update default for extensions.published to true
ALTER TABLE extensions ALTER COLUMN published SET DEFAULT true;

-- Update default for extension_versions.published to true
ALTER TABLE extension_versions ALTER COLUMN published SET DEFAULT true;
