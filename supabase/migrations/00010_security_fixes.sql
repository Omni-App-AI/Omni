-- ============================================================
-- Migration 00010: Security Fixes
-- Fixes Supabase linter warnings:
--   1. Function search_path mutable (6 functions)
--   2. pg_trgm extension in public schema
--   3. Overly permissive RLS INSERT policies
-- ============================================================

-- ============================================================
-- 1. Fix function search_path — pin to '' (empty) so they
--    cannot be exploited via search_path manipulation.
--    We re-create each function with SET search_path = ''.
-- ============================================================

-- 1a. handle_new_user (SECURITY DEFINER — most critical)
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO public.profiles (id, username, display_name, avatar_url)
    VALUES (
        NEW.id,
        COALESCE(
            NEW.raw_user_meta_data->>'user_name',
            NEW.raw_user_meta_data->>'preferred_username',
            LOWER(REPLACE(COALESCE(NEW.raw_user_meta_data->>'full_name', SPLIT_PART(NEW.email, '@', 1)), ' ', '-'))
        ),
        COALESCE(
            NEW.raw_user_meta_data->>'full_name',
            NEW.raw_user_meta_data->>'name',
            SPLIT_PART(NEW.email, '@', 1)
        ),
        NEW.raw_user_meta_data->>'avatar_url'
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = '';

-- 1b. update_updated_at
CREATE OR REPLACE FUNCTION public.update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SET search_path = '';

-- 1c. update_latest_version
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
$$ LANGUAGE plpgsql SET search_path = '';

-- 1d. update_extension_rating
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
$$ LANGUAGE plpgsql SET search_path = '';

-- 1e. increment_download_count
CREATE OR REPLACE FUNCTION public.increment_download_count()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE public.extensions
    SET total_downloads = total_downloads + 1
    WHERE id = NEW.extension_id;

    INSERT INTO public.download_stats (extension_id, date, count)
    VALUES (NEW.extension_id, CURRENT_DATE, 1)
    ON CONFLICT (extension_id, date)
    DO UPDATE SET count = public.download_stats.count + 1;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SET search_path = '';

-- 1f. cleanup_rate_limits (SECURITY DEFINER — critical)
CREATE OR REPLACE FUNCTION public.cleanup_rate_limits() RETURNS void AS $$
BEGIN
    DELETE FROM public.rate_limits WHERE created_at < now() - interval '24 hours';
    DELETE FROM public.security_events WHERE created_at < now() - interval '90 days';
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = '';


-- ============================================================
-- 2. Move pg_trgm extension out of public schema.
--    Create the extensions schema, re-create pg_trgm there,
--    and rebuild the dependent trigram index.
-- ============================================================

CREATE SCHEMA IF NOT EXISTS extensions;

-- Drop the index that depends on pg_trgm ops in public
DROP INDEX IF EXISTS public.idx_extensions_name_trgm;

-- Move the extension to the extensions schema
DROP EXTENSION IF EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS pg_trgm SCHEMA extensions;

-- Re-create the trigram index using the new schema-qualified operator class
CREATE INDEX idx_extensions_name_trgm ON public.extensions
    USING GIN (name extensions.gin_trgm_ops);


-- ============================================================
-- 3. Fix overly permissive RLS INSERT policies.
--    Replace WITH CHECK (true) with service_role-only access.
-- ============================================================

-- 3a. downloads: restrict inserts to service_role only
DROP POLICY IF EXISTS "Service role inserts downloads" ON public.downloads;
CREATE POLICY "Service role inserts downloads"
    ON public.downloads FOR INSERT
    TO service_role
    WITH CHECK (true);

-- 3b. release_downloads: restrict inserts to service_role only
DROP POLICY IF EXISTS "release_downloads_insert" ON public.release_downloads;
CREATE POLICY "release_downloads_insert"
    ON public.release_downloads FOR INSERT
    TO service_role
    WITH CHECK (true);
