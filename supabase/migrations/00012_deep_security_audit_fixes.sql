-- ============================================================
-- Migration 00012: Deep Security Audit Fixes
-- ============================================================
--
-- AUDIT FINDINGS (17 issues, by severity):
--
-- ===== CRITICAL (P0) =====
--
-- [C1] cleanup_rate_limits() callable by any user via PostgREST RPC
--      Location: anti_bot.sql:130
--      Risk:     Any anonymous user can call SELECT public.cleanup_rate_limits()
--               to wipe all rate_limits records (< 24h), completely bypassing
--               rate limiting. Also deletes 90-day security_events (audit log).
--      Fix:      REVOKE EXECUTE from public/anon/authenticated roles.
--
-- [C2] Profile UPDATE — no column restriction → admin impersonation
--      Location: 00007_rls_policies.sql:17-19
--      Risk:     The UPDATE policy is USING(auth.uid()=id) with no column guard.
--               Any user can UPDATE their own: verified_publisher=true,
--               is_moderator=true, reputation=999999, follower_count, etc.
--               ** is_moderator=true grants access to ALL admin endpoints **
--               (admin/extensions, admin/stats, admin/users/ban, admin/moderation,
--               admin/security/events, admin/security/ips).
--      Fix:      BEFORE UPDATE trigger resets server-controlled columns.
--
-- [C3] Extensions UPDATE — no column restriction → trust/ranking manipulation
--      Location: 00007_rls_policies.sql:35-37
--      Risk:     Publishers can UPDATE their own extension's: trust_level='verified',
--               featured=true, total_downloads=999999, average_rating=5.00,
--               review_count=9999, moderation_status='active' (un-takedown).
--      Fix:      BEFORE UPDATE trigger resets server-controlled columns.
--
-- [C4] Storage: any authenticated user can write/overwrite/delete ANY user's files
--      Location: 00007_rls_policies.sql:147-152, extension_images.sql:31-50
--      Risk:     INSERT policy only checks auth.uid() IS NOT NULL — no ownership.
--               User A can upload to User B's extension path in extension-wasm,
--               replacing their WASM with malware. extension-images UPDATE/DELETE
--               policies also have no ownership check.
--      Fix:      Require storage path starts with an extension ID owned by uploader.
--               UPDATE/DELETE restricted to file owner (storage.objects.owner).
--
-- ===== HIGH (P1) =====
--
-- [H1] update_extension_rating() not SECURITY DEFINER — broken denormalization
--      Location: 00003_reviews.sql:24-42 (re-created in 00010:60-79)
--      Risk:     Trigger fires on reviews INSERT/UPDATE/DELETE by authenticated users.
--               Without SECURITY DEFINER, the UPDATE on extensions table is subject
--               to RLS (requires publisher_id = auth.uid()). When user A reviews
--               user B's extension, the UPDATE silently matches 0 rows.
--               Result: average_rating and review_count NEVER update.
--      Fix:      Add SECURITY DEFINER to update_extension_rating,
--               update_latest_version, and increment_download_count.
--
-- [H2] forum_posts UPDATE — authors can set admin-only columns
--      Location: 00008_forum_and_profiles.sql:162-164
--      Risk:     Authors can set pinned=true, locked=true, vote_score=999999,
--               reply_count=0, view_count=999999 on their own posts.
--      Fix:      BEFORE UPDATE trigger resets admin/counter columns.
--
-- [H3] forum_replies UPDATE — authors can set is_accepted
--      Location: 00008_forum_and_profiles.sql:170
--      Risk:     Reply authors can mark is_accepted=true on their own reply.
--               Only the post author (via server-side logic) should set this.
--      Fix:      BEFORE UPDATE trigger resets server-controlled columns.
--
-- [H4] downloads SELECT leaks PII (ip_hash + user_id per download)
--      Location: 00007_rls_policies.sql:90-92
--      Risk:     ip_hash is a stable fingerprint for cross-extension user tracking.
--               user_id directly reveals which users downloaded which extensions.
--               download_stats already provides aggregated counts without PII.
--      Fix:      Restrict downloads SELECT to service_role only.
--
-- [H5] release_downloads SELECT leaks PII (ip_hash + user_agent)
--      Location: 00009_app_releases.sql:60-63
--      Risk:     user_agent strings enable device fingerprinting.
--      Fix:      Restrict release_downloads SELECT to service_role only.
--
-- [H6] scan_results SELECT exposes detection internals
--      Location: 00007_rls_policies.sql:106-108
--      Risk:     signature_matches, heuristic_details, ai_analysis, ai_flags,
--               sandbox_results are all publicly readable. Malware authors can
--               see exactly which rules flagged their code and iterate until
--               they evade all detection layers.
--      Fix:      Restrict to publisher (own extensions) + service_role.
--               Public verdict/score is already on extension_versions table.
--
-- [H7] shadow_hidden not enforced in forum SELECT RLS
--      Location: anti_bot.sql:147-149, 00008:161,168,175
--      Risk:     Shadow-hidden posts/replies/reviews are visible via direct
--               PostgREST queries, defeating the shadow ban mechanism.
--      Fix:      Add shadow_hidden filter to SELECT policies (authors still
--               see their own content to maintain the shadow ban illusion).
--
-- [H8] handle_new_user() — no username sanitization → signup failures
--      Location: 00001_profiles.sql:18-38
--      Risk:     GitHub usernames with uppercase letters (e.g., "MyName") fail
--               the CHECK constraint (^[a-z0-9_-]{3,39}$). COALESCE of
--               full_name doesn't strip special chars. Username collisions
--               cause the entire signup transaction to fail.
--      Fix:      LOWER + REGEXP_REPLACE + conflict loop with suffix.
--
-- ===== MEDIUM (P2) =====
--
-- [M1] downloads.extension_id missing ON DELETE CASCADE
--      Location: 00004_downloads.sql:4
--      Risk:     Extension deletion blocked by FK; other tables all cascade.
--      Fix:      Re-create FK with ON DELETE CASCADE.
--
-- [M2] No text length limits on user-controlled text fields
--      Risk:     Users can INSERT megabytes into bio, body, description, etc.
--      Fix:      Add CHECK constraints on key fields.
--
-- [M3] No URL validation on URL columns (avatar_url, icon_url, etc.)
--      Risk:     javascript: or data: URIs could be stored and rendered.
--      Note:     Should be validated in API layer; DB constraint too rigid.
--
-- ============================================================


-- ============================================================
-- [C1] REVOKE EXECUTE on cleanup_rate_limits from all public roles
-- ============================================================
-- This prevents any user from calling the function via PostgREST RPC.
-- Only postgres/superuser can call it (e.g., via pg_cron or direct SQL).
REVOKE EXECUTE ON FUNCTION public.cleanup_rate_limits() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION public.cleanup_rate_limits() FROM anon;
REVOKE EXECUTE ON FUNCTION public.cleanup_rate_limits() FROM authenticated;


-- ============================================================
-- [C2] Profile privilege escalation — protect server-controlled columns
-- ============================================================
CREATE OR REPLACE FUNCTION public.protect_profile_columns()
RETURNS TRIGGER AS $$
BEGIN
    -- Only service_role / superuser can modify these columns.
    -- Regular authenticated users get their changes silently reverted.
    IF current_user IN ('anon', 'authenticated') THEN
        NEW.verified_publisher := OLD.verified_publisher;
        NEW.is_moderator       := OLD.is_moderator;
        NEW.is_banned          := OLD.is_banned;
        NEW.ban_reason         := OLD.ban_reason;
        NEW.reputation         := OLD.reputation;
        NEW.follower_count     := OLD.follower_count;
        NEW.following_count    := OLD.following_count;
        NEW.post_count         := OLD.post_count;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SET search_path = '';

CREATE TRIGGER protect_profile_cols
    BEFORE UPDATE ON public.profiles
    FOR EACH ROW EXECUTE FUNCTION public.protect_profile_columns();


-- ============================================================
-- [C3] Extensions privilege escalation — protect server-controlled columns
-- ============================================================
CREATE OR REPLACE FUNCTION public.protect_extension_columns()
RETURNS TRIGGER AS $$
BEGIN
    IF current_user IN ('anon', 'authenticated') THEN
        NEW.trust_level        := OLD.trust_level;
        NEW.featured           := OLD.featured;
        NEW.total_downloads    := OLD.total_downloads;
        NEW.average_rating     := OLD.average_rating;
        NEW.review_count       := OLD.review_count;
        NEW.moderation_status  := OLD.moderation_status;
        NEW.moderation_note    := OLD.moderation_note;
        NEW.moderated_by       := OLD.moderated_by;
        NEW.moderated_at       := OLD.moderated_at;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SET search_path = '';

CREATE TRIGGER protect_extension_cols
    BEFORE UPDATE ON public.extensions
    FOR EACH ROW EXECUTE FUNCTION public.protect_extension_columns();


-- ============================================================
-- [C4] Storage policies — restrict to extension owners
-- ============================================================
-- Path convention (confirmed in publish/route.ts:117):
--   <extension_id>/<version>/<filename>
-- So SPLIT_PART(name, '/', 1) = extension_id.

-- Drop ALL existing overly-permissive storage INSERT/UPDATE/DELETE policies
DROP POLICY IF EXISTS "Publishers can upload extension files" ON storage.objects;
DROP POLICY IF EXISTS "Publishers can insert extension images" ON storage.objects;
DROP POLICY IF EXISTS "Publishers can update extension images" ON storage.objects;
DROP POLICY IF EXISTS "Publishers can delete extension images" ON storage.objects;

-- INSERT: user must own the extension referenced by the first path segment
CREATE POLICY "Publishers can upload own extension files"
    ON storage.objects FOR INSERT
    WITH CHECK (
        (select auth.uid()) IS NOT NULL
        AND bucket_id IN ('extension-wasm', 'extension-icons', 'extension-screenshots', 'extension-images')
        AND EXISTS (
            SELECT 1 FROM public.extensions e
            WHERE e.publisher_id = (select auth.uid())
            AND SPLIT_PART(name, '/', 1) = e.id
        )
    );

-- UPDATE: only the original uploader (file owner) can modify
CREATE POLICY "File owners can update extension files"
    ON storage.objects FOR UPDATE
    USING (
        (select auth.uid()) = owner
        AND bucket_id IN ('extension-wasm', 'extension-icons', 'extension-screenshots', 'extension-images')
    );

-- DELETE: only the original uploader (file owner) can delete
CREATE POLICY "File owners can delete extension files"
    ON storage.objects FOR DELETE
    USING (
        (select auth.uid()) = owner
        AND bucket_id IN ('extension-wasm', 'extension-icons', 'extension-screenshots', 'extension-images')
    );


-- ============================================================
-- [H1] Make trigger functions SECURITY DEFINER so they can
--      update across RLS boundaries (e.g., reviewer → extensions table)
-- ============================================================

-- update_extension_rating: MUST be SECURITY DEFINER — currently broken
-- for all non-publisher reviews (silently fails to update ratings)
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
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = '';

-- update_latest_version: defense in depth (currently works via service_role)
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
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = '';

-- increment_download_count: defense in depth (currently works via service_role)
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
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = '';


-- ============================================================
-- [H2] forum_posts — protect admin/counter columns from authors
-- ============================================================
CREATE OR REPLACE FUNCTION public.protect_forum_post_columns()
RETURNS TRIGGER AS $$
BEGIN
    IF current_user IN ('anon', 'authenticated') THEN
        NEW.pinned     := OLD.pinned;
        NEW.locked     := OLD.locked;
        NEW.vote_score := OLD.vote_score;
        NEW.reply_count := OLD.reply_count;
        NEW.view_count  := OLD.view_count;
        NEW.shadow_hidden := OLD.shadow_hidden;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SET search_path = '';

CREATE TRIGGER protect_forum_post_cols
    BEFORE UPDATE ON public.forum_posts
    FOR EACH ROW EXECUTE FUNCTION public.protect_forum_post_columns();


-- ============================================================
-- [H3] forum_replies — protect server-controlled columns
-- ============================================================
CREATE OR REPLACE FUNCTION public.protect_forum_reply_columns()
RETURNS TRIGGER AS $$
BEGIN
    IF current_user IN ('anon', 'authenticated') THEN
        NEW.is_accepted   := OLD.is_accepted;
        NEW.vote_score    := OLD.vote_score;
        NEW.shadow_hidden := OLD.shadow_hidden;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SET search_path = '';

CREATE TRIGGER protect_forum_reply_cols
    BEFORE UPDATE ON public.forum_replies
    FOR EACH ROW EXECUTE FUNCTION public.protect_forum_reply_columns();


-- ============================================================
-- [H4] downloads SELECT — restrict to service_role only
-- (download_stats table already provides aggregate counts publicly)
-- ============================================================
DROP POLICY IF EXISTS "Anyone can view downloads" ON public.downloads;
CREATE POLICY "Service role reads downloads"
    ON public.downloads FOR SELECT
    TO service_role
    USING (true);


-- ============================================================
-- [H5] release_downloads SELECT — restrict to service_role only
-- ============================================================
DROP POLICY IF EXISTS "release_downloads_read" ON public.release_downloads;
CREATE POLICY "Service role reads release_downloads"
    ON public.release_downloads FOR SELECT
    TO service_role
    USING (true);


-- ============================================================
-- [H6] scan_results SELECT — restrict to publisher + service_role
-- (public verdict/score available on extension_versions.scan_status/scan_score)
-- ============================================================
DROP POLICY IF EXISTS "Scan results are publicly readable" ON public.scan_results;

-- Publishers can view scan results for their own extensions
CREATE POLICY "Publishers can view own scan results"
    ON public.scan_results FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM public.extensions e
            WHERE e.id = scan_results.extension_id
            AND e.publisher_id = (select auth.uid())
        )
    );

-- Service role has full access (for scan pipeline)
CREATE POLICY "Service role reads scan results"
    ON public.scan_results FOR SELECT
    TO service_role
    USING (true);


-- ============================================================
-- [H7] shadow_hidden enforcement in SELECT policies
-- Authors still see their own content (shadow ban illusion).
-- Moderators can see all content.
-- ============================================================

-- forum_posts
DROP POLICY IF EXISTS "forum_posts_read" ON public.forum_posts;
CREATE POLICY "forum_posts_read"
    ON public.forum_posts FOR SELECT
    USING (
        shadow_hidden = false
        OR (select auth.uid()) = author_id
        OR EXISTS (
            SELECT 1 FROM public.profiles
            WHERE id = (select auth.uid()) AND is_moderator = true
        )
    );

-- forum_replies
DROP POLICY IF EXISTS "forum_replies_read" ON public.forum_replies;
CREATE POLICY "forum_replies_read"
    ON public.forum_replies FOR SELECT
    USING (
        shadow_hidden = false
        OR (select auth.uid()) = author_id
        OR EXISTS (
            SELECT 1 FROM public.profiles
            WHERE id = (select auth.uid()) AND is_moderator = true
        )
    );

-- reviews
DROP POLICY IF EXISTS "Reviews are publicly readable" ON public.reviews;
CREATE POLICY "Reviews are publicly readable"
    ON public.reviews FOR SELECT
    USING (
        shadow_hidden = false
        OR (select auth.uid()) = user_id
        OR EXISTS (
            SELECT 1 FROM public.profiles
            WHERE id = (select auth.uid()) AND is_moderator = true
        )
    );


-- ============================================================
-- [H8] handle_new_user — robust username sanitization
-- ============================================================
-- Fixes:
--   1. LOWER() all username sources
--   2. REGEXP_REPLACE to strip invalid chars
--   3. SUBSTR to enforce 3-39 char limit
--   4. Fallback to UUID prefix if username too short
--   5. Conflict loop with numeric suffix on collision
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER AS $$
DECLARE
    base_username TEXT;
    final_username TEXT;
    suffix INTEGER := 0;
BEGIN
    -- Derive username from OAuth metadata, sanitize for CHECK constraint
    base_username := LOWER(REGEXP_REPLACE(
        COALESCE(
            NEW.raw_user_meta_data->>'user_name',
            NEW.raw_user_meta_data->>'preferred_username',
            REPLACE(
                COALESCE(
                    NEW.raw_user_meta_data->>'full_name',
                    SPLIT_PART(NEW.email, '@', 1)
                ), ' ', '-'
            )
        ),
        '[^a-z0-9_-]', '', 'g'   -- strip everything not in allowed set
    ));

    -- Ensure minimum length (pad with random chars if needed)
    IF LENGTH(base_username) < 3 THEN
        base_username := base_username || SUBSTR(REPLACE(gen_random_uuid()::text, '-', ''), 1, 8);
    END IF;

    -- Truncate to max length
    base_username := SUBSTR(base_username, 1, 39);

    -- Handle uniqueness conflicts with numeric suffix
    final_username := base_username;
    WHILE EXISTS (SELECT 1 FROM public.profiles WHERE username = final_username) LOOP
        suffix := suffix + 1;
        -- Leave room for the suffix: base-N
        final_username := SUBSTR(base_username, 1, 39 - LENGTH(suffix::text) - 1) || '-' || suffix;
    END LOOP;

    INSERT INTO public.profiles (id, username, display_name, avatar_url)
    VALUES (
        NEW.id,
        final_username,
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


-- ============================================================
-- [M1] downloads.extension_id — add ON DELETE CASCADE
-- ============================================================
ALTER TABLE public.downloads DROP CONSTRAINT IF EXISTS downloads_extension_id_fkey;
ALTER TABLE public.downloads
    ADD CONSTRAINT downloads_extension_id_fkey
    FOREIGN KEY (extension_id) REFERENCES public.extensions(id) ON DELETE CASCADE;


-- ============================================================
-- [M2] Text length limits on user-controlled fields
-- ============================================================

-- profiles
ALTER TABLE public.profiles
    ADD CONSTRAINT profiles_display_name_length CHECK (char_length(display_name) <= 100),
    ADD CONSTRAINT profiles_bio_length CHECK (char_length(bio) <= 2000),
    ADD CONSTRAINT profiles_website_length CHECK (char_length(website) <= 500),
    ADD CONSTRAINT profiles_github_username_length CHECK (char_length(github_username) <= 39);

-- extensions (publisher-controlled fields only; name/description have FTS)
ALTER TABLE public.extensions
    ADD CONSTRAINT extensions_name_length CHECK (char_length(name) <= 100),
    ADD CONSTRAINT extensions_description_length CHECK (char_length(description) <= 10000),
    ADD CONSTRAINT extensions_homepage_length CHECK (char_length(homepage) <= 500),
    ADD CONSTRAINT extensions_repository_length CHECK (char_length(repository) <= 500),
    ADD CONSTRAINT extensions_license_length CHECK (char_length(license) <= 100);

-- forum
ALTER TABLE public.forum_posts
    ADD CONSTRAINT forum_posts_title_length CHECK (char_length(title) <= 300),
    ADD CONSTRAINT forum_posts_body_length CHECK (char_length(body) <= 50000);

ALTER TABLE public.forum_replies
    ADD CONSTRAINT forum_replies_body_length CHECK (char_length(body) <= 50000);

-- reviews
ALTER TABLE public.reviews
    ADD CONSTRAINT reviews_title_length CHECK (char_length(title) <= 200),
    ADD CONSTRAINT reviews_body_length CHECK (char_length(body) <= 5000);

-- extension_versions
ALTER TABLE public.extension_versions
    ADD CONSTRAINT versions_changelog_length CHECK (char_length(changelog) <= 10000);
