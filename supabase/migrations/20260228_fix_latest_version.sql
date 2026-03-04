-- Fix latest_version for all extensions by setting it to the most recent version
UPDATE extensions e
SET latest_version = sub.version
FROM (
  SELECT DISTINCT ON (extension_id) extension_id, version
  FROM extension_versions
  ORDER BY extension_id, created_at DESC
) sub
WHERE e.id = sub.extension_id
  AND (e.latest_version IS NULL OR e.latest_version = '');
