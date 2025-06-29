-- Remove index created in the up migration
DROP INDEX IF EXISTS idx_job_stage_outputs_job_id;

DROP TABLE IF EXISTS job_stage_outputs;
