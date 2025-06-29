-- Remove indexes added in the up migration
DROP INDEX IF EXISTS idx_analysis_jobs_status;
DROP INDEX IF EXISTS idx_documents_org_id;

DROP TABLE IF EXISTS analysis_jobs;
DROP TABLE IF EXISTS pipelines;
DROP TABLE IF EXISTS documents;
