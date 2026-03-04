-- Download tracking
CREATE TABLE public.downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    extension_id TEXT NOT NULL REFERENCES public.extensions(id),
    version TEXT NOT NULL,
    user_id UUID REFERENCES public.profiles(id),
    ip_hash TEXT,
    source TEXT NOT NULL DEFAULT 'website'
        CHECK (source IN ('website', 'cli', 'app')),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_downloads_extension ON public.downloads(extension_id);
CREATE INDEX idx_downloads_date ON public.downloads(created_at);

-- Daily aggregated stats for dashboard charts
CREATE TABLE public.download_stats (
    extension_id TEXT NOT NULL REFERENCES public.extensions(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    count INTEGER DEFAULT 0,
    PRIMARY KEY (extension_id, date)
);

-- Function to increment total_downloads on extension
CREATE OR REPLACE FUNCTION public.increment_download_count()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE public.extensions
    SET total_downloads = total_downloads + 1
    WHERE id = NEW.extension_id;

    INSERT INTO public.download_stats (extension_id, date, count)
    VALUES (NEW.extension_id, CURRENT_DATE, 1)
    ON CONFLICT (extension_id, date)
    DO UPDATE SET count = download_stats.count + 1;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER download_count_increment
    AFTER INSERT ON public.downloads
    FOR EACH ROW EXECUTE FUNCTION public.increment_download_count();
