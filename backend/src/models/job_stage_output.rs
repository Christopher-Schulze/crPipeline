use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Serialize, FromRow, Debug)]
pub struct JobStageOutput {
    pub id: Uuid,
    pub job_id: Uuid,
    pub stage_name: String,
    pub output_type: String, // "json", "pdf", "txt", etc.
    pub s3_bucket: String,
    pub s3_key: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)] // Added derive Debug for NewJobStageOutput as well
pub struct NewJobStageOutput {
    pub job_id: Uuid,
    pub stage_name: String,
    pub output_type: String,
    pub s3_bucket: String,
    pub s3_key: String,
}

impl JobStageOutput {
    pub async fn create(pool: &PgPool, new_output: NewJobStageOutput) -> sqlx::Result<JobStageOutput> {
        sqlx::query_as::<_, JobStageOutput>(
            "INSERT INTO job_stage_outputs (job_id, stage_name, output_type, s3_bucket, s3_key) \
             VALUES ($1, $2, $3, $4, $5) RETURNING *"
        )
        .bind(new_output.job_id)
        .bind(new_output.stage_name)
        .bind(new_output.output_type)
        .bind(new_output.s3_bucket)
        .bind(new_output.s3_key) // Corrected: s3_key
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_job_id(pool: &PgPool, job_id: Uuid) -> sqlx::Result<Vec<JobStageOutput>> {
        sqlx::query_as::<_, JobStageOutput>(
            "SELECT * FROM job_stage_outputs WHERE job_id = $1 ORDER BY created_at ASC"
        )
        .bind(job_id)
        .fetch_all(pool)
        .await
    }
}
