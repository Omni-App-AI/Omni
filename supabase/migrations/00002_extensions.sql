-- Enable trigram extension for fuzzy search (must be before gin_trgm_ops index)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Extensions (core registry)
CREATE TABLE public.extensions (
    id TEXT PRIMARY KEY CHECK (id ~ '^[a-z0-9]+(\.[a-z0-9_-]+){2,}$'),
    publisher_id UUID NOT NULL REFERENCES public.profiles(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    short_description TEXT NOT NULL CHECK (char_length(short_description) <= 160),
    icon_url TEXT,
    homepage TEXT,
    repository TEXT,
    license TEXT,
    categories TEXT[] NOT NULL DEFAULT '{}',
    tags TEXT[] NOT NULL DEFAULT '{}',
    trust_level TEXT NOT NULL DEFAULT 'unverified'
        CHECK (trust_level IN ('verified', 'community', 'unverified')),
    featured BOOLEAN DEFAULT FALSE,
    total_downloads BIGINT DEFAULT 0,
    average_rating NUMERIC(3,2) DEFAULT 0,
    review_count INTEGER DEFAULT 0,
    latest_version TEXT,
    published BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_extensions_publisher ON public.extensions(publisher_id);
CREATE INDEX idx_extensions_categories ON public.extensions USING GIN(categories);
CREATE INDEX idx_extensions_tags ON public.extensions USING GIN(tags);
CREATE INDEX idx_extensions_trust ON public.extensions(trust_level);
CREATE INDEX idx_extensions_featured ON public.extensions(featured) WHERE featured = TRUE;
CREATE INDEX idx_extensions_downloads ON public.extensions(total_downloads DESC);
CREATE INDEX idx_extensions_name_trgm ON public.extensions USING GIN(name gin_trgm_ops);

-- Full-text search column
ALTER TABLE public.extensions ADD COLUMN fts tsvector
    GENERATED ALWAYS AS (
        setweight(to_tsvector('english', COALESCE(name, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(short_description, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(description, '')), 'C')
    ) STORED;

CREATE INDEX idx_extensions_fts ON public.extensions USING GIN(fts);

CREATE TRIGGER extensions_updated_at
    BEFORE UPDATE ON public.extensions
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

-- Extension versions
CREATE TABLE public.extension_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    extension_id TEXT NOT NULL REFERENCES public.extensions(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    changelog TEXT,
    wasm_url TEXT NOT NULL,
    wasm_size_bytes BIGINT NOT NULL,
    checksum TEXT NOT NULL,
    source_url TEXT,
    min_omni_version TEXT,
    permissions JSONB NOT NULL DEFAULT '[]',
    tools JSONB NOT NULL DEFAULT '[]',
    manifest JSONB NOT NULL,
    signature TEXT,
    scan_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (scan_status IN ('pending', 'scanning', 'passed', 'failed', 'flagged')),
    scan_score NUMERIC(5,2),
    scan_completed_at TIMESTAMPTZ,
    published BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(extension_id, version)
);

CREATE INDEX idx_versions_extension ON public.extension_versions(extension_id);
CREATE INDEX idx_versions_scan ON public.extension_versions(scan_status);
CREATE INDEX idx_versions_created ON public.extension_versions(created_at DESC);

-- Function to update latest_version on extension when a version passes scan
CREATE OR REPLACE FUNCTION public.update_latest_version()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.scan_status = 'passed' AND NEW.published = TRUE THEN
        UPDATE public.extensions
        SET latest_version = NEW.version, updated_at = NOW()
        WHERE id = NEW.extension_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER version_scan_passed
    AFTER UPDATE OF scan_status ON public.extension_versions
    FOR EACH ROW
    WHEN (NEW.scan_status = 'passed')
    EXECUTE FUNCTION public.update_latest_version();
