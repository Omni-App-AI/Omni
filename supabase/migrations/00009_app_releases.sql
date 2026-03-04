-- ============================================================
-- Migration 00009: App Releases & Download Tracking
-- ============================================================

-- App release metadata (one row per published version)
CREATE TABLE public.app_releases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version TEXT NOT NULL UNIQUE,
    channel TEXT NOT NULL DEFAULT 'stable'
        CHECK (channel IN ('stable', 'beta', 'nightly')),
    release_notes TEXT,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    github_release_id BIGINT,
    is_draft BOOLEAN NOT NULL DEFAULT FALSE,
    is_prerelease BOOLEAN NOT NULL DEFAULT FALSE,
    platforms JSONB NOT NULL DEFAULT '{}',
    min_supported_version TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_app_releases_channel ON public.app_releases(channel);
CREATE INDEX idx_app_releases_published ON public.app_releases(published_at DESC);
CREATE INDEX idx_app_releases_version ON public.app_releases(version);

-- Release download tracking (analytics)
CREATE TABLE public.release_downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    release_id UUID NOT NULL REFERENCES public.app_releases(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    ip_hash TEXT,
    user_agent TEXT,
    downloaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_release_downloads_release ON public.release_downloads(release_id);
CREATE INDEX idx_release_downloads_platform ON public.release_downloads(platform);
CREATE INDEX idx_release_downloads_date ON public.release_downloads(downloaded_at DESC);

-- RLS policies
ALTER TABLE public.app_releases ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.release_downloads ENABLE ROW LEVEL SECURITY;

-- Published releases are publicly readable
CREATE POLICY "app_releases_public_read"
    ON public.app_releases FOR SELECT
    USING (is_draft = false);

-- Service role manages all releases (used by CI publish endpoint)
CREATE POLICY "app_releases_service_manage"
    ON public.app_releases FOR ALL
    TO service_role
    USING (true)
    WITH CHECK (true);

-- Anyone can insert download records (tracking)
CREATE POLICY "release_downloads_insert"
    ON public.release_downloads FOR INSERT
    WITH CHECK (true);

-- Download records are readable (analytics)
CREATE POLICY "release_downloads_read"
    ON public.release_downloads FOR SELECT
    USING (true);
