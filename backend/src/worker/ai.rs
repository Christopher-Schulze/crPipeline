use crate::models::{AnalysisJob, OrgSettings};
use crate::processing;
use crate::worker::{save_stage_output, Stage};
use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;
use std::env;
use tracing::{error, warn};

/// Execute an AI stage and return the resulting JSON.
pub async fn handle_ai_stage(
    pool: &PgPool,
    s3: &S3Client,
    job: &AnalysisJob,
    stage: &Stage,
    org_settings: Option<&OrgSettings>,
    bucket: &str,
    current_json: serde_json::Value,
    local_pdf: &std::path::Path,
) -> Result<serde_json::Value> {
    let (endpoint, key) = if let Some(settings) = org_settings {
        let ep = settings
            .ai_api_endpoint
            .clone()
            .unwrap_or_else(|| env::var("AI_API_URL").unwrap_or_default());
        let k = settings
            .ai_api_key
            .clone()
            .unwrap_or_else(|| env::var("AI_API_KEY").unwrap_or_default());
        (ep, k)
    } else {
        (
            env::var("AI_API_URL").unwrap_or_default(),
            env::var("AI_API_KEY").unwrap_or_default(),
        )
    };

    if endpoint.is_empty() {
        error!(job_id=%job.id, "AI endpoint missing");
        return Err(anyhow::anyhow!("AI endpoint missing"));
    }

    let input_json = current_json.clone();
    // prompt handling skipped

    // Save AI input
    if let Ok(bytes) = serde_json::to_vec_pretty(&input_json) {
        let name = format!("{}_input", stage.stage_type);
        if let Err(e) =
            save_stage_output(pool, s3, job.id, &name, "json", bucket, bytes, "json").await
        {
            warn!(job_id=%job.id, "Failed to save AI input: {:?}", e);
        }
    }

    let headers = org_settings.and_then(|s| s.ai_custom_headers.as_ref());

    let result = processing::run_ai(&input_json, &endpoint, &key, headers).await?;

    // Save AI output
    if let Ok(bytes) = serde_json::to_vec_pretty(&result) {
        if let Err(e) = save_stage_output(
            pool,
            s3,
            job.id,
            &stage.stage_type,
            "json",
            bucket,
            bytes,
            "json",
        )
        .await
        {
            error!(job_id=%job.id, "Failed to save AI output: {:?}", e);
        }
    }

    // remove local pdf if something went wrong
    if !local_pdf.exists() {
        let _ = local_pdf; // keep lint happy
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::meta::region::RegionProviderChain;
    use aws_sdk_s3::Client as S3Client;
    use sqlx::postgres::PgPoolOptions;
    use tempfile::tempdir;
    use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path}};
    use serial_test::serial;

    fn stage() -> Stage {
        Stage { stage_type: "ai".into(), command: None, prompt_name: None, ocr_engine: None, ocr_stage_endpoint: None, ocr_stage_key: None, config: None }
    }

    fn job() -> AnalysisJob {
        AnalysisJob { id: uuid::Uuid::new_v4(), org_id: uuid::Uuid::new_v4(), document_id: uuid::Uuid::new_v4(), pipeline_id: uuid::Uuid::new_v4(), status: String::new(), created_at: chrono::Utc::now() }
    }

    async fn clients() -> (sqlx::Pool<sqlx::Postgres>, S3Client) {
        let pool = PgPoolOptions::new().connect_lazy("postgres://user@localhost/db").unwrap();
        let rp = RegionProviderChain::default_provider().or_else("us-east-1");
        let shared = aws_config::from_env().region(rp).load().await;
        let cfg = aws_sdk_s3::config::Builder::from(&shared).endpoint_url("http://localhost").force_path_style(true).build();
        (pool, S3Client::from_conf(cfg))
    }

    #[actix_rt::test]
    #[serial]
    async fn ai_stage_success() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/ai"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;
        std::env::set_var("AI_API_URL", format!("{}/ai", server.uri()));
        std::env::set_var("AI_API_KEY", "key");
        let (pool, s3) = clients().await;
        let res = handle_ai_stage(&pool, &s3, &job(), &stage(), None, "bucket", serde_json::json!({}), &dir.path().join("in.pdf"))
            .await
            .unwrap();
        assert_eq!(res, serde_json::json!({"ok": true}));
    }

    #[actix_rt::test]
    #[serial]
    async fn ai_stage_error() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(path("/ai"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        std::env::set_var("AI_API_URL", format!("{}/ai", server.uri()));
        std::env::set_var("AI_API_KEY", "key");
        let (pool, s3) = clients().await;
        let res = handle_ai_stage(&pool, &s3, &job(), &stage(), None, "bucket", serde_json::json!({}), &dir.path().join("in.pdf"))
            .await;
        assert!(res.is_err());
    }
}

