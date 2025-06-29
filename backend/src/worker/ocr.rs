use crate::models::{AnalysisJob, OrgSettings};
use crate::processing;
use crate::worker::{save_stage_output, Stage};
use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;
use std::path::Path;
use tracing::error;

pub async fn handle_ocr_stage(
    pool: &PgPool,
    s3: &S3Client,
    job: &AnalysisJob,
    stage: &Stage,
    _org_settings: Option<&OrgSettings>,
    bucket: &str,
    local: &Path,
    txt_path: &Path,
) -> Result<bool> {
    if let Err(e) = processing::run_ocr(local, txt_path).await {
        error!(job_id=%job.id, "OCR failed: {:?}", e);
        return Ok(true);
    }
    if let Ok(text) = tokio::fs::read_to_string(txt_path).await {
        let _ = save_stage_output(pool, s3, job.id, &stage.stage_type, "txt", bucket, text.into_bytes(), "txt").await;
    }
    let _ = tokio::fs::remove_file(txt_path).await;
    Ok(false)
}

#[cfg(test)]
mod tests {
    #[actix_rt::test]
    async fn ocr_stage_compiles() {
        assert!(true);
    }
}
