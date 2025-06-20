use std::{time::Duration, env, path::PathBuf};
use sqlx::postgres::PgPoolOptions;
use backend::models::{AnalysisJob, Pipeline, Document};
use backend::processing;
use tokio::time::sleep;
use tokio::process::Command;
use tracing::{info, error};
use serde_json;
use serde::Deserialize;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;

#[derive(Deserialize)]
struct Stage {
    #[serde(rename = "type")]
    stage_type: String,
    command: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let s3_client = S3Client::new(&shared_config);
    let redis_url = env::var("REDIS_URL")?;
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_async_connection().await?;
    loop {
        let (_queue, job_id_str): (String, String) = redis::cmd("BLPOP")
            .arg("jobs")
            .arg(0)
            .query_async(&mut conn)
            .await?;
        let job_id = match uuid::Uuid::parse_str(&job_id_str) {
            Ok(id) => id,
            Err(_) => continue,
        };
        let job = match AnalysisJob::find(&pool, job_id).await {
            Ok(j) => j,
            Err(e) => {
                error!(?e, "job not found");
                continue;
            }
        };
        let doc = match sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id=$1")
            .bind(job.document_id)
            .fetch_one(&pool)
            .await
        {
            Ok(d) => d,
            Err(e) => {
                error!(?e, "document missing");
                continue;
            }
        };
        info!(job_id=%job.id, "processing job");
        AnalysisJob::update_status(&pool, job.id, "in_progress").await?;
        let pipeline: Pipeline = sqlx::query_as("SELECT * FROM pipelines WHERE id=$1")
            .bind(job.pipeline_id)
            .fetch_one(&pool)
            .await?;
        let stages: Vec<Stage> = serde_json::from_value(pipeline.stages)?;
        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
        let mut local = std::env::temp_dir();
        local.push(format!("{}-input.pdf", job.id));
        processing::download_pdf(&s3_client, &bucket, &doc.filename, &local).await?;
        let mut txt_path = local.clone();
        txt_path.set_extension("txt");
        let mut json_result = serde_json::json!({});
        for stage in stages {
            info!(job_id=%job.id, stage=?stage.stage_type, "running stage");
            match stage.stage_type.as_str() {
                "ocr" => {
                    if let Some(cmd) = stage.command {
                        let parts: Vec<&str> = cmd.split_whitespace().collect();
                        if let Some(program) = parts.first() {
                            let args = &parts[1..];
                            Command::new(program).args(args).status().await?;
                        }
                    } else {
                        processing::run_ocr(&local, &txt_path).await?;
                    }
                },
                "parse" => {
                    json_result = if let Some(cmd) = stage.command {
                        let parts: Vec<&str> = cmd.split_whitespace().collect();
                        if let Some(program) = parts.first() {
                            let args = &parts[1..];
                            Command::new(program).args(args).status().await?;
                        }
                        serde_json::json!({})
                    } else {
                        processing::parse_text(&txt_path).await?
                    };
                },
                "ai" => {
                    let endpoint = env::var("AI_API_URL").unwrap_or_default();
                    let key = env::var("AI_API_KEY").unwrap_or_default();
                    json_result = processing::run_ai(&json_result, &endpoint, &key).await?;
                },
                "report" => {
                    let mut pdf_out = local.clone();
                    pdf_out.set_extension("report.pdf");
                    processing::generate_report(&json_result, &pdf_out)?;
                    let body = tokio::fs::read(pdf_out).await?;
                    s3_client.put_object().bucket(&bucket).key(format!("{}-report.pdf", job.id)).body(body.into()).send().await?;
                },
                _ => {
                    if let Some(cmd) = stage.command {
                        let parts: Vec<&str> = cmd.split_whitespace().collect();
                        if let Some(program) = parts.first() {
                            let args = &parts[1..];
                            Command::new(program).args(args).status().await?;
                        }
                    } else {
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
        AnalysisJob::update_status(&pool, job.id, "completed").await?;
        info!(job_id=%job.id, "job completed");
    }
}
