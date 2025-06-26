ALTER TABLE org_settings
ADD COLUMN ai_api_endpoint TEXT,
ADD COLUMN ai_api_key TEXT,
ADD COLUMN ocr_api_endpoint TEXT,
ADD COLUMN ocr_api_key TEXT,
ADD COLUMN prompt_templates JSONB;
