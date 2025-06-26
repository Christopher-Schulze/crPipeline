ALTER TABLE documents
ADD COLUMN display_name TEXT;

-- Backfill display_name with existing filename for old records
UPDATE documents SET display_name = filename WHERE display_name IS NULL;

ALTER TABLE documents
ALTER COLUMN display_name SET NOT NULL;
