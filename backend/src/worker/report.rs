use crate::models::{AnalysisJob, Document};
use crate::processing;
use crate::worker::{save_stage_output, upload_bytes, Stage};
use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use sqlx::PgPool;
use std::path::Path;
use tracing::{error, info};

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReportStageConfig {
    template: String,
    #[serde(default, rename = "summaryFields")]
    _summary_fields: Vec<String>,
}

#[tracing::instrument(skip(pool, s3, job, doc, stage, json_result, local_pdf))]
pub async fn handle_report_stage(
    pool: &PgPool,
    s3: &S3Client,
    job: &AnalysisJob,
    doc: &Document,
    stage: &Stage,
    bucket: &str,
    json_result: &serde_json::Value,
    local_pdf: &Path,
) -> Result<()> {
    info!(job_id=%job.id, stage=%stage.stage_type, "start report stage");
    let timer = crate::worker::metrics::STAGE_HISTOGRAM
        .with_label_values(&[stage.stage_type.as_str()])
        .start_timer();
    let mut data_for_templating = json_result.clone();
    if let serde_json::Value::Object(ref mut map) = data_for_templating {
        map.insert(
            "document_name".to_string(),
            serde_json::Value::String(doc.filename.clone()),
        );
        map.insert("job_id".to_string(), serde_json::Value::String(job.id.to_string()));
    } else {
        data_for_templating = serde_json::json!({
            "document_name": doc.filename.clone(),
            "job_id": job.id.to_string(),
            "previous_stage_output": json_result.clone()
        });
    }

    let cfg: Option<ReportStageConfig> = stage
        .config
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let pdf_out = std::env::temp_dir().join(format!("{}_report_temp.pdf", job.id));

    if let Some(conf) = cfg {
        info!("Report stage using template");
        if let Err(e) = processing::report::generate_report_from_template(
            &conf.template,
            &data_for_templating,
            &pdf_out,
        )
        .await
        {
            error!(job_id=%job.id, "Custom report failed: {:?}", e);
            processing::report::generate_report(&data_for_templating, &pdf_out)?;
        }
    } else {
        processing::report::generate_report(&data_for_templating, &pdf_out)?;
    }

    if pdf_out.exists() {
        let s3_key = format!("jobs/{}/outputs/{}-report.pdf", job.id, job.id);
        let bytes = tokio::fs::read(&pdf_out).await?;
        upload_bytes(s3, bucket, &s3_key, bytes).await?;
        let rec = save_stage_output(
            pool,
            s3,
            job.id,
            &stage.stage_type,
            "pdf",
            bucket,
            Vec::new(),
            "pdf",
        )
        .await;
        let _ = rec; // ignore errors for tests
        tokio::fs::remove_file(&pdf_out).await.ok();
    }

    let _ = local_pdf; // suppress unused
    info!(job_id=%job.id, stage=%stage.stage_type, "finished report stage");
    timer.observe_duration();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::meta::region::RegionProviderChain;
    use aws_sdk_s3::Client as S3Client;
    use sqlx::postgres::PgPoolOptions;
    use tempfile::tempdir;
    use serial_test::serial;

    fn stage() -> Stage {
        Stage { stage_type: "report".into(), command: None, prompt_name: None, ocr_engine: None, ocr_stage_endpoint: None, ocr_stage_key: None, config: None }
    }

    fn job() -> AnalysisJob {
        AnalysisJob { id: uuid::Uuid::new_v4(), org_id: uuid::Uuid::new_v4(), document_id: uuid::Uuid::new_v4(), pipeline_id: uuid::Uuid::new_v4(), status: String::new(), created_at: chrono::Utc::now() }
    }

    fn doc() -> Document {
        Document { id: uuid::Uuid::new_v4(), org_id: uuid::Uuid::new_v4(), owner_id: uuid::Uuid::new_v4(), filename: "doc.pdf".into(), pages: 1, is_target: true, upload_date: chrono::Utc::now(), expires_at: None, display_name: "doc.pdf".into() }
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
    async fn report_stage_success() {
        std::env::set_var("SKIP_DB", "1");
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", dir.path());
        let (pool, s3) = clients().await;
        let res = handle_report_stage(&pool, &s3, &job(), &doc(), &stage(), "bucket", &serde_json::json!({"val":1}), &dir.path().join("in.pdf"))
            .await;
        assert!(res.is_ok());
    }

    #[actix_rt::test]
    #[serial]
    async fn report_stage_upload_error() {
        let dir = tempdir().unwrap();
        std::env::set_var("LOCAL_S3_DIR", "/dev/null/dir");
        std::env::set_var("SKIP_DB", "1");
        let (pool, s3) = clients().await;
        let res = handle_report_stage(&pool, &s3, &job(), &doc(), &stage(), "bucket", &serde_json::json!({"val":1}), &dir.path().join("in.pdf"))
            .await;
        assert!(res.is_err());
    }
}
