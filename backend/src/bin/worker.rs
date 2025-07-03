use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use backend::models::{AnalysisJob, Document, OrgSettings, Pipeline};
use backend::processing;
use backend::worker::metrics::{spawn_metrics_server, JOB_COUNTER, STAGE_HISTOGRAM};
use backend::worker::{self, Stage};
use serde_json::{self, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Execute all stages of a job. Returns `Ok` on success or `Err` on the first stage failure.
async fn run_stages(
    pool: &PgPool,
    s3_client: &S3Client,
    job: &AnalysisJob,
    doc: &Document,
    stages: &[Stage],
    org_settings: Option<&OrgSettings>,
    bucket: &str,
    local: &PathBuf,
    txt_path: &PathBuf,
) -> Result<()> {
    let mut json_result = Value::default();
    for stage in stages {
        info!(job_id=%job.id, stage=?stage.stage_type, command=?stage.command, prompt_name=?stage.prompt_name, ocr_engine=?stage.ocr_engine, "running stage");
        let start = Instant::now();
        match stage.stage_type.as_str() {
            "ocr" => {
                if worker::ocr::handle_ocr_stage(
                    pool,
                    s3_client,
                    job,
                    stage,
                    org_settings,
                    bucket,
                    local,
                    txt_path,
                )
                .await?
                {
                    break;
                }
            }
            "parse" => {
                if !txt_path.exists() {
                    warn!(job_id=%job.id, stage=%stage.stage_type, "Input text file {:?} not found for parse stage. Skipping.", txt_path);
                } else {
                    let text_content = tokio::fs::read_to_string(txt_path).await?;
                    json_result =
                        processing::run_parse_stage(&text_content, stage.config.as_ref()).await?;
                }
                if let Ok(b) = serde_json::to_vec_pretty(&json_result) {
                    let _ = worker::save_stage_output(
                        pool,
                        s3_client,
                        job.id,
                        &stage.stage_type,
                        "json",
                        bucket,
                        b,
                        "json",
                    )
                    .await;
                }
            }
            "ai" => {
                json_result = worker::ai::handle_ai_stage(
                    pool,
                    s3_client,
                    job,
                    stage,
                    org_settings,
                    bucket,
                    json_result.clone(),
                    local,
                )
                .await?;
            }
            "report" => {
                worker::report::handle_report_stage(
                    pool,
                    s3_client,
                    job,
                    doc,
                    stage,
                    bucket,
                    &json_result,
                    local,
                )
                .await?;
            }
            _ => {
                if let Some(cmd) = stage.command.as_ref() {
                    let mut parts = cmd.split_whitespace();
                    if let Some(program) = parts.next() {
                        let args: Vec<&str> = parts.collect();
                        Command::new(program).args(args).status().await?;
                    }
                } else {
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
        STAGE_HISTOGRAM
            .with_label_values(&[stage.stage_type.as_str()])
            .observe(start.elapsed().as_secs_f64());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
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

    let process_once = env::var("PROCESS_ONE_JOB").is_ok();
    let metrics_port = env::var("METRICS_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(9100);
    spawn_metrics_server(metrics_port);

    loop {
        let (_queue, job_id_str): (String, String) = redis::cmd("BLPOP")
            .arg("jobs")
            .arg(0)
            .query_async(&mut conn)
            .await?;
        let job_id = match Uuid::parse_str(&job_id_str) {
            Ok(id) => id,
            Err(_) => continue,
        };
        let job = match AnalysisJob::find(&pool, job_id).await {
            Ok(j) => j,
            Err(e) => {
                error!(job_id=%job_id_str, "Failed to fetch job details: {:?}", e);
                continue;
            }
        };

        let org_settings = match OrgSettings::find(&pool, job.org_id).await {
            Ok(settings) => Some(settings),
            Err(e) => {
                error!(job_id=%job.id, org_id=%job.org_id, "Failed to fetch org settings: {:?}", e);
                None
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

        let pipeline: Pipeline = sqlx::query_as("SELECT * FROM pipelines WHERE id=$1")
            .bind(job.pipeline_id)
            .fetch_one(&pool)
            .await?;
        let stages: Vec<Stage> = serde_json::from_value(pipeline.stages)?;

        let bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
        let mut local = std::env::temp_dir();
        local.push(format!("{}-input.pdf", job.id));
        processing::download_pdf(&s3_client, &bucket, &doc.filename, &local).await?;
        let mut txt_path = local.clone();
        txt_path.set_extension("txt");

        let result = run_stages(
            &pool,
            &s3_client,
            &job,
            &doc,
            &stages,
            org_settings.as_ref(),
            &bucket,
            &local,
            &txt_path,
        )
        .await;

        match result {
            Ok(_) => {
                AnalysisJob::update_status(&pool, job.id, "completed").await?;
                JOB_COUNTER.with_label_values(&["success"]).inc();
                info!(job_id=%job.id, "Job processing completed successfully.");
            }
            Err(e) => {
                error!(job_id=%job.id, "Job processing failed: {:?}", e);
                AnalysisJob::update_status(&pool, job.id, "failed").await?;
                JOB_COUNTER.with_label_values(&["failed"]).inc();
            }
        }

        if local.exists() {
            if let Err(e) = tokio::fs::remove_file(&local).await {
                warn!(job_id=%job.id, path=?local, "Failed to clean up input PDF: {:?}", e);
            }
        }
        if txt_path.exists() {
            let _ = tokio::fs::remove_file(&txt_path).await;
        }

        if process_once {
            break;
        }
    }
    Ok(())
}
