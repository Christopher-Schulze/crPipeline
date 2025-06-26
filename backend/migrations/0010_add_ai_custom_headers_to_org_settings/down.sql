ALTER TABLE org_settings
DROP COLUMN IF EXISTS ai_custom_headers; -- Added IF EXISTS for robustness
