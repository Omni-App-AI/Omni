ALTER TABLE permission_policies ADD COLUMN use_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE permission_policies ADD COLUMN last_used TEXT;
