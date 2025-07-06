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
pub mod metrics;
pub mod ocr;
pub mod report;

/// Runtime configuration for the worker.
#[derive(Debug, Clone)]
pub struct WorkerRuntimeConfig {
    /// Number of jobs to process concurrently.
    pub concurrency: usize,
}

impl WorkerRuntimeConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let concurrency = std::env::var("WORKER_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        Self { concurrency }
    }
}

use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};

/// Spawn a task listening for `SIGHUP` to reload runtime settings.
#[cfg(unix)]
pub fn spawn_config_reloader(concurrency: Arc<AtomicUsize>) {
    tokio::spawn(async move {
        if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()) {
            while sig.recv().await.is_some() {
                let cfg = WorkerRuntimeConfig::from_env();
                let val = cfg.concurrency.max(1);
                concurrency.store(val, Ordering::SeqCst);
                tracing::info!(concurrency=val, "Reloaded worker configuration");
            }
        }
    });
}

#[cfg(not(unix))]
pub fn spawn_config_reloader(_concurrency: Arc<AtomicUsize>) {}

use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use crate::worker::metrics::{S3_ERROR_COUNTER, WORKER_SHUTDOWN_COUNTER};
use std::env;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

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
        let mut attempts = 0;
        loop {
            match s3
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(ByteStream::from(data.clone()))
                .send()
                .await
            {
                Ok(_) => break Ok(()),
                Err(_e) if attempts < 3 => {
                    S3_ERROR_COUNTER.with_label_values(&["upload"]).inc();
                    attempts += 1;
                    sleep(Duration::from_millis(500 * attempts as u64)).await;
                    continue;
                }
                Err(e) => {
                    S3_ERROR_COUNTER.with_label_values(&["upload"]).inc();
                    break Err(e.into());
                }
            }
        }
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
    tracing::info!(job_id=%job_id, stage=%stage_name, "saving stage output");
    // Allow tests to run without a database by skipping the insert when
    // the SKIP_DB environment variable is set.
    #[cfg(test)]
    if std::env::var("SKIP_DB").is_ok() {
        upload_bytes(s3, bucket, "test", content).await?;
        return Ok(());
    }
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
    tracing::info!(job_id=%job_id, stage=%stage_name, "stage output saved");
    Ok(())
}

/// Log that the worker shuts down after being idle and update metrics.
pub fn log_idle_shutdown() {
    tracing::info!("Idle timeout reached, shutting down");
    WORKER_SHUTDOWN_COUNTER.inc();
}
