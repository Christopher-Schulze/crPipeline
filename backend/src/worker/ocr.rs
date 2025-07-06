use crate::models::{AnalysisJob, OrgSettings};
use crate::processing;
use crate::worker::{metrics::API_ERROR_COUNTER, save_stage_output, Stage};
use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;
use std::path::Path;
use tracing::{error, info};

#[tracing::instrument(skip(pool, s3, job, stage, org_settings, local, txt_path))]
pub async fn handle_ocr_stage(
    pool: &PgPool,
    s3: &S3Client,
    job: &AnalysisJob,
    stage: &Stage,
    org_settings: Option<&OrgSettings>,
    bucket: &str,
    local: &Path,
    txt_path: &Path,
) -> Result<bool> {
    info!(job_id=%job.id, stage=%stage.stage_type, "start ocr stage");
    let timer = crate::worker::metrics::STAGE_HISTOGRAM
        .with_label_values(&[stage.stage_type.as_str()])
        .start_timer();
    // Check if this stage should use an external OCR engine
    let use_external = stage.ocr_engine.as_deref() == Some("external");

    let text_result = if use_external {
        let endpoint = stage
            .ocr_stage_endpoint
            .clone()
            .or_else(|| org_settings.and_then(|s| s.ocr_api_endpoint.clone()))
            .unwrap_or_else(|| std::env::var("OCR_API_URL").unwrap_or_default());
        let key = stage
            .ocr_stage_key
            .clone()
            .or_else(|| org_settings.and_then(|s| s.ocr_api_key.clone()))
            .unwrap_or_else(|| std::env::var("OCR_API_KEY").unwrap_or_default());

        let pdf_bytes = match tokio::fs::read(local).await {
            Ok(b) => b,
            Err(e) => {
                error!(job_id=%job.id, "Failed to read input PDF for external OCR: {:?}", e);
                API_ERROR_COUNTER.with_label_values(&["ocr"]).inc();
                timer.observe_duration();
                return Ok(true);
            }
        };

        match processing::ocr::run_external_ocr(
            &endpoint,
            if key.is_empty() {
                None
            } else {
                Some(key.as_str())
            },
            pdf_bytes,
            local
                .file_name()
                .map(|f| f.to_string_lossy())
                .unwrap_or_else(|| "input.pdf".into())
                .as_ref(),
        )
        .await
        {
            Ok(text) => text,
            Err(e) => {
                error!(job_id=%job.id, "External OCR failed: {:?}", e);
                API_ERROR_COUNTER.with_label_values(&["ocr"]).inc();
                timer.observe_duration();
                return Ok(true);
            }
        }
    } else {
        if let Err(e) = processing::ocr::run_ocr(local, txt_path).await {
            error!(job_id=%job.id, "OCR failed: {:?}", e);
            API_ERROR_COUNTER.with_label_values(&["ocr"]).inc();
            timer.observe_duration();
            return Ok(true);
        }
        match tokio::fs::read_to_string(txt_path).await {
            Ok(t) => t,
            Err(e) => {
                error!(job_id=%job.id, "Failed to read OCR output: {:?}", e);
                API_ERROR_COUNTER.with_label_values(&["ocr"]).inc();
                timer.observe_duration();
                return Ok(true);
            }
        }
    };

    if let Err(e) = tokio::fs::write(txt_path, &text_result).await {
        error!(job_id=%job.id, "Failed to write OCR text: {:?}", e);
    }
    let _ = save_stage_output(
        pool,
        s3,
        job.id,
        &stage.stage_type,
        "txt",
        bucket,
        text_result.clone().into_bytes(),
        "txt",
    )
    .await;
    let _ = tokio::fs::remove_file(txt_path).await;
    info!(job_id=%job.id, stage=%stage.stage_type, "finished ocr stage");
    timer.observe_duration();
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::meta::region::RegionProviderChain;
    use aws_sdk_s3::Client as S3Client;
    use serial_test::serial;
    use sqlx::postgres::PgPoolOptions;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

    fn dummy_stage() -> Stage {
        Stage {
            stage_type: "ocr".into(),
            command: None,
            prompt_name: None,
            ocr_engine: None,
            ocr_stage_endpoint: None,
            ocr_stage_key: None,
            config: None,
        }
    }

    fn dummy_job() -> AnalysisJob {
        AnalysisJob {
            id: uuid::Uuid::new_v4(),
            org_id: uuid::Uuid::new_v4(),
            document_id: uuid::Uuid::new_v4(),
            pipeline_id: uuid::Uuid::new_v4(),
            status: "pending".into(),
            created_at: chrono::Utc::now(),
        }
    }

    async fn dummy_clients() -> (sqlx::Pool<sqlx::Postgres>, S3Client) {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://user@localhost/db")
            .unwrap();
        let rp = RegionProviderChain::default_provider().or_else("us-east-1");
        let shared = aws_config::from_env().region(rp).load().await;
        let cfg = aws_sdk_s3::config::Builder::from(&shared)
            .endpoint_url("http://localhost")
            .force_path_style(true)
            .build();
        (pool, S3Client::from_conf(cfg))
    }

    #[actix_rt::test]
    #[serial]
    async fn ocr_stage_error_when_tesseract_missing() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        std::env::set_var("PATH", "");
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let (pool, s3) = dummy_clients().await;
        let job = dummy_job();
        let stage = dummy_stage();
        let input = dir.path().join("in.pdf");
        tokio::fs::write(&input, b"pdf").await.unwrap();
        let txt = dir.path().join("out.txt");
        let res = handle_ocr_stage(&pool, &s3, &job, &stage, None, "bucket", &input, &txt)
            .await
            .unwrap();
        assert!(res);
        assert_eq!(server.received_requests().await.unwrap().len(), 0);
    }

    #[actix_rt::test]
    #[serial]
    async fn ocr_stage_success_with_dummy_script() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let bin_dir = dir.path().join("bin");
        tokio::fs::create_dir(&bin_dir).await.unwrap();
        let script = bin_dir.join("tesseract");
        tokio::fs::write(&script, "#!/bin/sh\necho hello > $2")
            .await
            .unwrap();
        tokio::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .await
            .unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", bin_dir.display(), old_path));
        let (pool, s3) = dummy_clients().await;
        let job = dummy_job();
        let stage = dummy_stage();
        let input = dir.path().join("in.pdf");
        tokio::fs::write(&input, b"pdf").await.unwrap();
        let txt = dir.path().join("out.txt");
        let res = handle_ocr_stage(&pool, &s3, &job, &stage, None, "bucket", &input, &txt)
            .await
            .unwrap();
        assert!(!res);
        assert_eq!(server.received_requests().await.unwrap().len(), 0);
    }

    #[actix_rt::test]
    #[serial]
    async fn ocr_stage_external_with_stage_fields() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ext"))
            .mount(&server)
            .await;
        let (pool, s3) = dummy_clients().await;
        let job = dummy_job();
        let stage = Stage {
            stage_type: "ocr".into(),
            command: None,
            prompt_name: None,
            ocr_engine: Some("external".into()),
            ocr_stage_endpoint: Some(format!("{}/ocr", server.uri())),
            ocr_stage_key: Some("k".into()),
            config: None,
        };
        let input = dir.path().join("in.pdf");
        tokio::fs::write(&input, b"pdf").await.unwrap();
        let txt = dir.path().join("out.txt");
        let res = handle_ocr_stage(&pool, &s3, &job, &stage, None, "bucket", &input, &txt)
            .await
            .unwrap();
        assert!(!res);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[actix_rt::test]
    #[serial]
    async fn ocr_stage_external_with_org_settings() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ext2"))
            .mount(&server)
            .await;
        let (pool, s3) = dummy_clients().await;
        let job = dummy_job();
        let settings = OrgSettings {
            org_id: job.org_id,
            monthly_upload_quota: 100,
            monthly_analysis_quota: 100,
            accent_color: "#fff".into(),
            ai_api_endpoint: None,
            ai_api_key: None,
            ocr_api_endpoint: Some(format!("{}/ocr2", server.uri())),
            ocr_api_key: Some("k2".into()),
            prompt_templates: None,
            ai_custom_headers: None,
        };
        let stage = Stage {
            stage_type: "ocr".into(),
            command: None,
            prompt_name: None,
            ocr_engine: Some("external".into()),
            ocr_stage_endpoint: None,
            ocr_stage_key: None,
            config: None,
        };
        let input = dir.path().join("in.pdf");
        tokio::fs::write(&input, b"pdf").await.unwrap();
        let txt = dir.path().join("out.txt");
        let res = handle_ocr_stage(
            &pool,
            &s3,
            &job,
            &stage,
            Some(&settings),
            "bucket",
            &input,
            &txt,
        )
        .await
        .unwrap();
        assert!(!res);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
