-- ============================================================
-- Extension Images — Schema Additions
-- ============================================================

-- Banner image URL (2:1 ratio, recommended 1280x640)
ALTER TABLE extensions ADD COLUMN IF NOT EXISTS banner_url text;

-- Screenshots array (up to 5 URLs, recommended 1280x800 each)
ALTER TABLE extensions ADD COLUMN IF NOT EXISTS screenshots text[] NOT NULL DEFAULT '{}';

-- ============================================================
-- Extension Images — Storage Bucket & RLS
-- ============================================================

-- Create the bucket if it doesn't exist
INSERT INTO
    storage.buckets (id, name, public)
VALUES (
        'extension-images',
        'extension-images',
        true
    ) ON CONFLICT (id) DO NOTHING;

-- Extension images are publicly readable
CREATE POLICY "Extension images are publicly readable" ON storage.objects FOR
SELECT USING (
        bucket_id = 'extension-images'
    );

-- Publishers can insert extension images
CREATE POLICY "Publishers can insert extension images" ON storage.objects FOR
INSERT
WITH
    CHECK (
        auth.uid () IS NOT NULL
        AND bucket_id = 'extension-images'
    );

-- Publishers can update extension images
CREATE POLICY "Publishers can update extension images" ON storage.objects FOR
UPDATE USING (
    auth.uid () IS NOT NULL
    AND bucket_id = 'extension-images'
);

-- Publishers can delete extension images
CREATE POLICY "Publishers can delete extension images" ON storage.objects FOR DELETE USING (
    auth.uid () IS NOT NULL
    AND bucket_id = 'extension-images'
);