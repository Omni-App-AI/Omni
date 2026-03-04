-- ============================================================
-- Migration 00011: RLS Performance Fixes
-- Fixes Supabase linter warnings:
--   1. auth_rls_initplan — wrap auth.uid() in (select ...) so
--      it is evaluated once per query, not once per row.
--   2. multiple_permissive_policies — merge duplicate SELECT
--      policies on extensions and extension_versions into one.
-- ============================================================


-- ============================================================
-- 1. PROFILES — 1 policy
-- ============================================================

DROP POLICY IF EXISTS "Users can update own profile" ON public.profiles;
CREATE POLICY "Users can update own profile"
    ON public.profiles FOR UPDATE
    USING ((select auth.uid()) = id);


-- ============================================================
-- 2. EXTENSIONS — 5 policies (includes merging 2 SELECT → 1)
-- ============================================================

-- Merge "Published extensions are publicly readable" +
-- "Publishers can view own unpublished extensions" into one SELECT policy
DROP POLICY IF EXISTS "Published extensions are publicly readable" ON public.extensions;
DROP POLICY IF EXISTS "Publishers can view own unpublished extensions" ON public.extensions;
CREATE POLICY "Extensions are readable if published or owned"
    ON public.extensions FOR SELECT
    USING (published = true OR (select auth.uid()) = publisher_id);

DROP POLICY IF EXISTS "Publishers can insert extensions" ON public.extensions;
CREATE POLICY "Publishers can insert extensions"
    ON public.extensions FOR INSERT
    WITH CHECK ((select auth.uid()) = publisher_id);

DROP POLICY IF EXISTS "Publishers can update own extensions" ON public.extensions;
CREATE POLICY "Publishers can update own extensions"
    ON public.extensions FOR UPDATE
    USING ((select auth.uid()) = publisher_id);

DROP POLICY IF EXISTS "Publishers can delete own extensions" ON public.extensions;
CREATE POLICY "Publishers can delete own extensions"
    ON public.extensions FOR DELETE
    USING ((select auth.uid()) = publisher_id);


-- ============================================================
-- 3. EXTENSION_VERSIONS — 3 policies (includes merging 2 SELECT → 1)
-- ============================================================

-- Merge "Published versions are publicly readable" +
-- "Publishers can view own unpublished versions" into one SELECT policy
DROP POLICY IF EXISTS "Published versions are publicly readable" ON public.extension_versions;
DROP POLICY IF EXISTS "Publishers can view own unpublished versions" ON public.extension_versions;
CREATE POLICY "Versions are readable if published or owned"
    ON public.extension_versions FOR SELECT
    USING (
        published = true
        OR EXISTS (
            SELECT 1 FROM public.extensions
            WHERE id = extension_versions.extension_id
            AND publisher_id = (select auth.uid())
        )
    );

DROP POLICY IF EXISTS "Publishers can insert versions for own extensions" ON public.extension_versions;
CREATE POLICY "Publishers can insert versions for own extensions"
    ON public.extension_versions FOR INSERT
    WITH CHECK (
        EXISTS (
            SELECT 1 FROM public.extensions
            WHERE id = extension_versions.extension_id
            AND publisher_id = (select auth.uid())
        )
    );


-- ============================================================
-- 4. REVIEWS — 3 policies
-- ============================================================

DROP POLICY IF EXISTS "Authenticated users can create reviews" ON public.reviews;
CREATE POLICY "Authenticated users can create reviews"
    ON public.reviews FOR INSERT
    WITH CHECK ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can update own reviews" ON public.reviews;
CREATE POLICY "Users can update own reviews"
    ON public.reviews FOR UPDATE
    USING ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can delete own reviews" ON public.reviews;
CREATE POLICY "Users can delete own reviews"
    ON public.reviews FOR DELETE
    USING ((select auth.uid()) = user_id);


-- ============================================================
-- 5. API_KEYS — 4 policies
-- ============================================================

DROP POLICY IF EXISTS "Users can view own API keys" ON public.api_keys;
CREATE POLICY "Users can view own API keys"
    ON public.api_keys FOR SELECT
    USING ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can create own API keys" ON public.api_keys;
CREATE POLICY "Users can create own API keys"
    ON public.api_keys FOR INSERT
    WITH CHECK ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can update own API keys" ON public.api_keys;
CREATE POLICY "Users can update own API keys"
    ON public.api_keys FOR UPDATE
    USING ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can delete own API keys" ON public.api_keys;
CREATE POLICY "Users can delete own API keys"
    ON public.api_keys FOR DELETE
    USING ((select auth.uid()) = user_id);


-- ============================================================
-- 6. FORUM_POSTS — 3 policies
-- ============================================================

DROP POLICY IF EXISTS "forum_posts_insert" ON public.forum_posts;
CREATE POLICY "forum_posts_insert"
    ON public.forum_posts FOR INSERT
    WITH CHECK ((select auth.uid()) = author_id);

DROP POLICY IF EXISTS "forum_posts_update" ON public.forum_posts;
CREATE POLICY "forum_posts_update"
    ON public.forum_posts FOR UPDATE
    USING ((select auth.uid()) = author_id);

DROP POLICY IF EXISTS "forum_posts_delete" ON public.forum_posts;
CREATE POLICY "forum_posts_delete"
    ON public.forum_posts FOR DELETE
    USING ((select auth.uid()) = author_id);


-- ============================================================
-- 7. FORUM_REPLIES — 3 policies
-- ============================================================

DROP POLICY IF EXISTS "forum_replies_insert" ON public.forum_replies;
CREATE POLICY "forum_replies_insert"
    ON public.forum_replies FOR INSERT
    WITH CHECK ((select auth.uid()) = author_id);

DROP POLICY IF EXISTS "forum_replies_update" ON public.forum_replies;
CREATE POLICY "forum_replies_update"
    ON public.forum_replies FOR UPDATE
    USING ((select auth.uid()) = author_id);

DROP POLICY IF EXISTS "forum_replies_delete" ON public.forum_replies;
CREATE POLICY "forum_replies_delete"
    ON public.forum_replies FOR DELETE
    USING ((select auth.uid()) = author_id);


-- ============================================================
-- 8. FORUM_VOTES — 3 policies
-- ============================================================

DROP POLICY IF EXISTS "forum_votes_insert" ON public.forum_votes;
CREATE POLICY "forum_votes_insert"
    ON public.forum_votes FOR INSERT
    WITH CHECK ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "forum_votes_update" ON public.forum_votes;
CREATE POLICY "forum_votes_update"
    ON public.forum_votes FOR UPDATE
    USING ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "forum_votes_delete" ON public.forum_votes;
CREATE POLICY "forum_votes_delete"
    ON public.forum_votes FOR DELETE
    USING ((select auth.uid()) = user_id);


-- ============================================================
-- 9. USER_FOLLOWERS — 2 policies
-- ============================================================

DROP POLICY IF EXISTS "user_followers_insert" ON public.user_followers;
CREATE POLICY "user_followers_insert"
    ON public.user_followers FOR INSERT
    WITH CHECK ((select auth.uid()) = follower_id);

DROP POLICY IF EXISTS "user_followers_delete" ON public.user_followers;
CREATE POLICY "user_followers_delete"
    ON public.user_followers FOR DELETE
    USING ((select auth.uid()) = follower_id);


-- ============================================================
-- 10. CONTENT_FLAGS — 2 policies
-- ============================================================

DROP POLICY IF EXISTS "flags_insert_own" ON public.content_flags;
CREATE POLICY "flags_insert_own"
    ON public.content_flags FOR INSERT
    WITH CHECK ((select auth.uid()) = reporter_id);

DROP POLICY IF EXISTS "flags_select_own" ON public.content_flags;
CREATE POLICY "flags_select_own"
    ON public.content_flags FOR SELECT
    USING ((select auth.uid()) = reporter_id);
