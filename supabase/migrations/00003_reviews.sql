-- Reviews and ratings
CREATE TABLE public.reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    extension_id TEXT NOT NULL REFERENCES public.extensions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES public.profiles(id),
    rating INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 5),
    title TEXT,
    body TEXT,
    version TEXT,
    helpful_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(extension_id, user_id)
);

CREATE INDEX idx_reviews_extension ON public.reviews(extension_id);
CREATE INDEX idx_reviews_rating ON public.reviews(rating);

CREATE TRIGGER reviews_updated_at
    BEFORE UPDATE ON public.reviews
    FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

-- Function to update extension average rating
CREATE OR REPLACE FUNCTION public.update_extension_rating()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE public.extensions
    SET
        average_rating = (
            SELECT COALESCE(AVG(rating), 0)
            FROM public.reviews
            WHERE extension_id = COALESCE(NEW.extension_id, OLD.extension_id)
        ),
        review_count = (
            SELECT COUNT(*)
            FROM public.reviews
            WHERE extension_id = COALESCE(NEW.extension_id, OLD.extension_id)
        )
    WHERE id = COALESCE(NEW.extension_id, OLD.extension_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER review_rating_update
    AFTER INSERT OR UPDATE OR DELETE ON public.reviews
    FOR EACH ROW EXECUTE FUNCTION public.update_extension_rating();
