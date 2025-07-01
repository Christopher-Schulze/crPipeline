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
    use super::*;
    use aws_config::meta::region::RegionProviderChain;
    use aws_sdk_s3::Client as S3Client;
    use sqlx::postgres::PgPoolOptions;
    use tempfile::tempdir;
    use std::os::unix::fs::PermissionsExt;
    use serial_test::serial;

    fn dummy_stage() -> Stage {
        Stage { stage_type: "ocr".into(), command: None, prompt_name: None, ocr_engine: None, ocr_stage_endpoint: None, ocr_stage_key: None, config: None }
    }

    fn dummy_job() -> AnalysisJob {
        AnalysisJob { id: uuid::Uuid::new_v4(), org_id: uuid::Uuid::new_v4(), document_id: uuid::Uuid::new_v4(), pipeline_id: uuid::Uuid::new_v4(), status: "pending".into(), created_at: chrono::Utc::now() }
    }

    async fn dummy_clients() -> (sqlx::Pool<sqlx::Postgres>, S3Client) {
        let pool = PgPoolOptions::new().connect_lazy("postgres://user@localhost/db").unwrap();
        let rp = RegionProviderChain::default_provider().or_else("us-east-1");
        let shared = aws_config::from_env().region(rp).load().await;
        let cfg = aws_sdk_s3::config::Builder::from(&shared).endpoint_url("http://localhost").force_path_style(true).build();
        (pool, S3Client::from_conf(cfg))
    }

    #[actix_rt::test]
    #[serial]
    async fn ocr_stage_error_when_tesseract_missing() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        std::env::set_var("PATH", "");
        let (pool, s3) = dummy_clients().await;
        let job = dummy_job();
        let stage = dummy_stage();
        let input = dir.path().join("in.pdf");
        tokio::fs::write(&input, b"pdf").await.unwrap();
        let txt = dir.path().join("out.txt");
        let res = handle_ocr_stage(&pool, &s3, &job, &stage, None, "bucket", &input, &txt).await.unwrap();
        assert!(res);
    }

    #[actix_rt::test]
    #[serial]
    async fn ocr_stage_success_with_dummy_script() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        let bin_dir = dir.path().join("bin");
        tokio::fs::create_dir(&bin_dir).await.unwrap();
        let script = bin_dir.join("tesseract");
        tokio::fs::write(&script, "#!/bin/sh\necho hello > $2").await.unwrap();
        tokio::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).await.unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", bin_dir.display(), old_path));
        let (pool, s3) = dummy_clients().await;
        let job = dummy_job();
        let stage = dummy_stage();
        let input = dir.path().join("in.pdf");
        tokio::fs::write(&input, b"pdf").await.unwrap();
        let txt = dir.path().join("out.txt");
        let res = handle_ocr_stage(&pool, &s3, &job, &stage, None, "bucket", &input, &txt).await.unwrap();
        assert!(!res);
    }
}
