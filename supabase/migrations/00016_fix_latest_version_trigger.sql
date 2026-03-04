-- Fix: update_latest_version trigger was blindly overwriting latest_version
-- without semver comparison. If an older version's scan passed AFTER a newer
-- version was published, the trigger would regress latest_version to the old
-- version, causing the download endpoint to serve stale WASM binaries.
--
-- This fix adds a semver comparison so the trigger only updates latest_version
-- when the newly-passed version is actually greater than the current one.

CREATE OR REPLACE FUNCTION public.update_latest_version()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.scan_status = 'passed' AND NEW.published = TRUE THEN
        -- Only update latest_version if the newly-passed version is greater
        -- than (or equal to, for first publish) the current latest_version.
        -- Uses Postgres string_to_array + integer casting for semver comparison
        -- to avoid regressing latest_version when older versions pass scan
        -- out of order.
        UPDATE public.extensions
        SET latest_version = NEW.version, updated_at = NOW()
        WHERE id = NEW.extension_id
          AND (
            latest_version IS NULL
            OR (
              string_to_array(NEW.version, '.')::int[]
              >= string_to_array(latest_version, '.')::int[]
            )
          );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = '';
