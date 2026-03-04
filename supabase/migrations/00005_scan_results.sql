-- AV scan results
CREATE TABLE public.scan_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES public.extension_versions(id) ON DELETE CASCADE,
    extension_id TEXT NOT NULL,
    version TEXT NOT NULL,
    -- Layer 1: Signature scanning
    signature_score NUMERIC(5,2),
    signature_matches JSONB DEFAULT '[]',
    -- Layer 2: Heuristic analysis
    heuristic_score NUMERIC(5,2),
    heuristic_details JSONB DEFAULT '[]',
    -- Layer 3: AI code review
    ai_score NUMERIC(5,2),
    ai_analysis TEXT,
    ai_flags JSONB DEFAULT '[]',
    -- Layer 4: Sandbox execution
    sandbox_score NUMERIC(5,2),
    sandbox_results JSONB DEFAULT '{}',
    -- Aggregate
    overall_score NUMERIC(5,2) NOT NULL,
    verdict TEXT NOT NULL CHECK (verdict IN ('clean', 'suspicious', 'malicious', 'error')),
    auto_approved BOOLEAN DEFAULT FALSE,
    manual_reviewer_id UUID REFERENCES public.profiles(id),
    manual_review_notes TEXT,
    scan_duration_ms INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_scans_version ON public.scan_results(version_id);
CREATE INDEX idx_scans_extension ON public.scan_results(extension_id);
CREATE INDEX idx_scans_verdict ON public.scan_results(verdict);
CREATE INDEX idx_scans_created ON public.scan_results(created_at DESC);
