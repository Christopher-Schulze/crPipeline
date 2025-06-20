CREATE TABLE org_settings (
  org_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
  monthly_upload_quota INT NOT NULL DEFAULT 100,
  monthly_analysis_quota INT NOT NULL DEFAULT 100,
  accent_color TEXT NOT NULL DEFAULT '#30D5C8'
);
