use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use sqlx::PgPool;
use uuid::Uuid;
use tracing::info;

use crate::models::{NewJobStageOutput, JobStageOutput};

/// Upload a blob to S3 or write to `LOCAL_S3_DIR` when set.
pub async fn upload_bytes(s3_client: &S3Client, bucket: &str, key: &str, data: Vec<u8>) -> Result<(), anyhow::Error> {
    if let Ok(local_dir) = std::env::var("LOCAL_S3_DIR") {
        let mut path = PathBuf::from(local_dir);
        path.push(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, data).await?;
        Ok(())
    } else {
        s3_client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(data))
            .send()
            .await?;
        Ok(())
    }
}

/// Upload stage output and record it in the database.
///
/// Uses `upload_bytes`, which honors `LOCAL_S3_DIR` when present.
pub async fn save_stage_output(
    pool: &PgPool,
    s3_client: &S3Client,
    job_id: Uuid,
    stage_name: &str,
    output_type: &str, // "json", "pdf", "txt"
    s3_bucket_name: &str,
    content: Vec<u8>,
    file_extension: &str, // "json", "pdf", "txt"
) -> Result<(), anyhow::Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let s3_key = format!("jobs/{}/outputs/{}_{}.{}", job_id, stage_name, timestamp, file_extension);

    info!(job_id=%job_id, stage=%stage_name, s3_key=%s3_key, "Uploading stage output to storage.");
    upload_bytes(s3_client, s3_bucket_name, &s3_key, content).await?;
    info!(job_id=%job_id, stage=%stage_name, s3_key=%s3_key, "Successfully uploaded stage output.");

    let new_output_db_record = NewJobStageOutput {
        job_id,
        stage_name: stage_name.to_string(),
        output_type: output_type.to_string(),
        s3_bucket: s3_bucket_name.to_string(),
        s3_key,
    };
    JobStageOutput::create(pool, new_output_db_record).await?;
    info!(job_id=%job_id, stage=%stage_name, "Successfully saved stage output metadata to DB.");
    Ok(())
}
