-- Enable Row Level Security on all tables
ALTER TABLE public.profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.extensions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.extension_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.reviews ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.downloads ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.download_stats ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.scan_results ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.api_keys ENABLE ROW LEVEL SECURITY;

-- ============ PROFILES ============

CREATE POLICY "Profiles are publicly readable"
    ON public.profiles FOR SELECT
    USING (true);

CREATE POLICY "Users can update own profile"
    ON public.profiles FOR UPDATE
    USING (auth.uid() = id);

-- ============ EXTENSIONS ============

CREATE POLICY "Published extensions are publicly readable"
    ON public.extensions FOR SELECT
    USING (published = true);

CREATE POLICY "Publishers can view own unpublished extensions"
    ON public.extensions FOR SELECT
    USING (auth.uid() = publisher_id);

CREATE POLICY "Publishers can insert extensions"
    ON public.extensions FOR INSERT
    WITH CHECK (auth.uid() = publisher_id);

CREATE POLICY "Publishers can update own extensions"
    ON public.extensions FOR UPDATE
    USING (auth.uid() = publisher_id);

CREATE POLICY "Publishers can delete own extensions"
    ON public.extensions FOR DELETE
    USING (auth.uid() = publisher_id);

-- ============ EXTENSION VERSIONS ============

CREATE POLICY "Published versions are publicly readable"
    ON public.extension_versions FOR SELECT
    USING (published = true);

CREATE POLICY "Publishers can view own unpublished versions"
    ON public.extension_versions FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM public.extensions
            WHERE id = extension_versions.extension_id
            AND publisher_id = auth.uid()
        )
    );

CREATE POLICY "Publishers can insert versions for own extensions"
    ON public.extension_versions FOR INSERT
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM public.extensions
            WHERE id = extension_versions.extension_id
            AND publisher_id = auth.uid()
        )
    );

-- ============ REVIEWS ============

CREATE POLICY "Reviews are publicly readable"
    ON public.reviews FOR SELECT
    USING (true);

CREATE POLICY "Authenticated users can create reviews"
    ON public.reviews FOR INSERT
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own reviews"
    ON public.reviews FOR UPDATE
    USING (auth.uid() = user_id);

CREATE POLICY "Users can delete own reviews"
    ON public.reviews FOR DELETE
    USING (auth.uid() = user_id);

-- ============ DOWNLOADS ============

-- Downloads are tracked by service role; anonymous read for stats
CREATE POLICY "Anyone can view downloads"
    ON public.downloads FOR SELECT
    USING (true);

CREATE POLICY "Service role inserts downloads"
    ON public.downloads FOR INSERT
    WITH CHECK (true);

-- ============ DOWNLOAD STATS ============

CREATE POLICY "Download stats are publicly readable"
    ON public.download_stats FOR SELECT
    USING (true);

-- ============ SCAN RESULTS ============

CREATE POLICY "Scan results are publicly readable"
    ON public.scan_results FOR SELECT
    USING (true);

-- ============ API KEYS ============

CREATE POLICY "Users can view own API keys"
    ON public.api_keys FOR SELECT
    USING (auth.uid() = user_id);

CREATE POLICY "Users can create own API keys"
    ON public.api_keys FOR INSERT
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update own API keys"
    ON public.api_keys FOR UPDATE
    USING (auth.uid() = user_id);

CREATE POLICY "Users can delete own API keys"
    ON public.api_keys FOR DELETE
    USING (auth.uid() = user_id);

-- ============ STORAGE BUCKETS ============

-- Create storage buckets for extensions
INSERT INTO storage.buckets (id, name, public)
VALUES
    ('extension-wasm', 'extension-wasm', false),
    ('extension-icons', 'extension-icons', true),
    ('extension-screenshots', 'extension-screenshots', true);

-- Public read for icons and screenshots
CREATE POLICY "Extension icons are publicly readable"
    ON storage.objects FOR SELECT
    USING (bucket_id = 'extension-icons');

CREATE POLICY "Extension screenshots are publicly readable"
    ON storage.objects FOR SELECT
    USING (bucket_id = 'extension-screenshots');

-- Publishers can upload to their extension folders
CREATE POLICY "Publishers can upload extension files"
    ON storage.objects FOR INSERT
    WITH CHECK (
        auth.uid() IS NOT NULL AND
        bucket_id IN ('extension-wasm', 'extension-icons', 'extension-screenshots')
    );
