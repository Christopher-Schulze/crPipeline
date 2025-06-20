CREATE TABLE documents (
  id UUID PRIMARY KEY,
  org_id UUID REFERENCES organizations(id),
  owner_id UUID REFERENCES users(id),
  filename TEXT NOT NULL,
  pages INT DEFAULT 0,
  is_target BOOLEAN DEFAULT FALSE,
  upload_date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  expires_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE pipelines (
  id UUID PRIMARY KEY,
  org_id UUID REFERENCES organizations(id),
  name TEXT NOT NULL,
  stages JSONB NOT NULL
);

CREATE TABLE analysis_jobs (
  id UUID PRIMARY KEY,
  org_id UUID REFERENCES organizations(id),
  document_id UUID REFERENCES documents(id),
  pipeline_id UUID REFERENCES pipelines(id),
  status TEXT NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
