CREATE EXTENSION IF NOT EXISTS "uuid-ossp"; -- Ensure uuid_generate_v4() is available

CREATE TABLE job_stage_outputs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id UUID NOT NULL REFERENCES analysis_jobs(id) ON DELETE CASCADE,
    stage_name TEXT NOT NULL, -- e.g., "ocr", "parse", "ai", "report" or user-defined stage name
    output_type TEXT NOT NULL, -- e.g., "json", "pdf", "txt", "log"
    s3_bucket TEXT NOT NULL,
    s3_key TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_job_stage_outputs_job_id ON job_stage_outputs(job_id);
