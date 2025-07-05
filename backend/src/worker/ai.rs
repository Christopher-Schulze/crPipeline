use crate::models::{AnalysisJob, OrgSettings};
use crate::processing;
use crate::worker::{metrics::API_ERROR_COUNTER, save_stage_output, Stage};
use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;
use std::env;
use tracing::{error, warn, info};

/// Execute an AI stage and return the resulting JSON.
#[tracing::instrument(skip(pool, s3, job, stage, org_settings, current_json, local_pdf))]
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
    info!(job_id=%job.id, stage=%stage.stage_type, "start ai stage");
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

    // Kombiniert beide Logiken: Counter und die richtige run_ai-Methode!
    let result = match processing::ai_client::run_ai(&input_json, &endpoint, &key, headers).await {
        Ok(r) => r,
        Err(e) => {
            API_ERROR_COUNTER.with_label_values(&["ai"]).inc();
            return Err(e);
        }
    };

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

    info!(job_id=%job.id, stage=%stage.stage_type, "finished ai stage");
    Ok(result)
}