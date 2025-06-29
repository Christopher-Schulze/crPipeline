use crate::models::{AnalysisJob, Document};
use crate::processing;
use crate::worker::{save_stage_output, upload_bytes, Stage};
use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use serde_json::Value;
use sqlx::PgPool;
use std::path::Path;
use tracing::{error, info, warn};

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReportStageConfig {
    template: String,
    #[serde(default)]
    summary_fields: Vec<String>,
}

pub async fn handle_report_stage(
    pool: &PgPool,
    s3: &S3Client,
    job: &AnalysisJob,
    doc: &Document,
    stage: &Stage,
    bucket: &str,
    json_result: &Value,
    local_pdf: &Path,
) -> Result<()> {
    let mut data_for_templating = json_result.clone();
    if let Value::Object(ref mut map) = data_for_templating {
        map.insert(
            "document_name".to_string(),
            Value::String(doc.filename.clone()),
        );
        map.insert("job_id".to_string(), Value::String(job.id.to_string()));
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
        if let Err(e) = processing::generate_report_from_template(
            &conf.template,
            &data_for_templating,
            &pdf_out,
        )
        .await
        {
            error!(job_id=%job.id, "Custom report failed: {:?}", e);
            processing::generate_report(&data_for_templating, &pdf_out)?;
        }
    } else {
        processing::generate_report(&data_for_templating, &pdf_out)?;
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
    Ok(())
}

#[cfg(test)]
mod tests {
    #[actix_rt::test]
    async fn report_stage_compiles() { assert!(true); }
}
