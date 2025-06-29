use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub text: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Stage {
    #[serde(rename = "type")]
    pub stage_type: String,
    pub command: Option<String>,
    pub prompt_name: Option<String>,
    pub ocr_engine: Option<String>,
    pub ocr_stage_endpoint: Option<String>,
    pub ocr_stage_key: Option<String>,
    pub config: Option<Value>,
}

pub mod ai;
pub mod ocr;
pub mod report;

use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use std::env;
use std::path::PathBuf;

/// Upload a blob to S3 or the local filesystem when `LOCAL_S3_DIR` is set.
pub async fn upload_bytes(
    s3: &S3Client,
    bucket: &str,
    key: &str,
    data: Vec<u8>,
) -> Result<(), anyhow::Error> {
    if let Ok(local_dir) = env::var("LOCAL_S3_DIR") {
        let mut path = PathBuf::from(local_dir);
        path.push(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, data).await?;
        Ok(())
    } else {
        s3.put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(data))
            .send()
            .await?;
        Ok(())
    }
}

use crate::models::{JobStageOutput, NewJobStageOutput};
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Save stage output to storage and create a database record.
pub async fn save_stage_output(
    pool: &PgPool,
    s3: &S3Client,
    job_id: Uuid,
    stage_name: &str,
    output_type: &str,
    bucket: &str,
    content: Vec<u8>,
    file_ext: &str,
) -> Result<(), anyhow::Error> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let key = format!("jobs/{}/outputs/{}_{}.{}", job_id, stage_name, ts, file_ext);
    upload_bytes(s3, bucket, &key, content).await?;

    let rec = NewJobStageOutput {
        job_id,
        stage_name: stage_name.to_string(),
        output_type: output_type.to_string(),
        s3_bucket: bucket.to_string(),
        s3_key: key,
    };
    JobStageOutput::create(pool, rec).await?;
    Ok(())
}
