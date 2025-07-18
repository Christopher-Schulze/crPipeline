use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use backend::config::WorkerConfig;
use backend::models::{AnalysisJob, Document, OrgSettings, Pipeline};
use backend::processing;
use backend::worker::metrics::{
    spawn_metrics_server, JOB_COUNTER, JOB_HISTOGRAM, OCR_HISTOGRAM, RUNNING_JOBS_GAUGE,
    STAGE_HISTOGRAM,
};
use backend::worker::{self, Stage, WorkerRuntimeConfig};
use serde_json::json;
use serde_json::{self, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::process::Command;
use tokio::signal;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};
use uuid::Uuid;

async fn publish_status_event(job_id: Uuid, org_id: Uuid, status: &str) {
    if let Ok(url) = std::env::var("REDIS_URL") {
        if let Ok(client) = redis::Client::open(url) {
            if let Ok(mut conn) = client.get_async_connection().await {
                let payload =
                    json!({"job_id": job_id, "org_id": org_id, "status": status}).to_string();
                let _: Result<(), _> = redis::cmd("PUBLISH")
                    .arg("job_status")
                    .arg(payload)
                    .query_async(&mut conn)
                    .await;
            }
        }
    }
}

/// Execute all stages of a job. Returns `Ok` on success or `Err` on the first stage failure.
#[tracing::instrument(skip(pool, s3_client, job, doc, stages, org_settings, local, txt_path))]
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
        let mut break_after = false;
        let stage_result: Result<(), anyhow::Error> = match stage.stage_type.as_str() {
            "ocr" => {
                let engine = stage.ocr_engine.as_deref().unwrap_or("local");
                let ocr_start = Instant::now();
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
                    break_after = true;
                }
                let ocr_elapsed = ocr_start.elapsed().as_secs_f64();
                OCR_HISTOGRAM
                    .with_label_values(&[engine])
                    .observe(ocr_elapsed);
                Ok(())
            }
            "parse" => {
                if !txt_path.exists() {
                    warn!(job_id=%job.id, stage=%stage.stage_type, "Input text file {:?} not found for parse stage. Skipping.", txt_path);
                } else {
                    let text_content = tokio::fs::read_to_string(txt_path).await?;
                    json_result =
                        processing::parse::run_parse_stage(&text_content, stage.config.as_ref())
                            .await?;
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
                Ok(())
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
                Ok(())
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
                Ok(())
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
                Ok(())
            }
        };
        let elapsed = start.elapsed().as_secs_f64();
        STAGE_HISTOGRAM
            .with_label_values(&[stage.stage_type.as_str()])
            .observe(elapsed);
        match stage_result {
            Ok(_) => {
                info!(job_id=%job.id, stage=%stage.stage_type, duration=%elapsed, "stage finished");
            }
            Err(e) => {
                error!(job_id=%job.id, stage=%stage.stage_type, duration=%elapsed, "stage failed: {:?}", e);
                return Err(e);
            }
        }
        if break_after {
            break;
        }
    }
    Ok(())
}

#[tracing::instrument(skip(path))]
async fn remove_with_retry(path: &Path, job_id: Uuid, desc: &str) {
    const ATTEMPTS: u8 = 2;
    for attempt in 1..=ATTEMPTS {
        match tokio::fs::remove_file(path).await {
            Ok(_) => return,
            Err(e) if attempt < ATTEMPTS => {
                warn!(job_id=%job_id, path=?path, attempt, "Failed to remove {}: {:?}, retrying", desc, e);
                sleep(Duration::from_millis(100 * attempt as u64)).await;
            }
            Err(e) => {
                error!(job_id=%job_id, path=?path, "Failed to remove {}: {:?}", desc, e);
            }
        }
    }
}

#[tracing::instrument(skip(pool, s3_client, job, doc, stages, org_settings))]
async fn process_job(
    pool: Arc<PgPool>,
    s3_client: Arc<S3Client>,
    job: AnalysisJob,
    doc: Document,
    stages: Vec<Stage>,
    org_settings: Option<OrgSettings>,
    bucket: String,
) {
    RUNNING_JOBS_GAUGE.inc();
    let mut local = std::env::temp_dir();
    local.push(format!("{}-input.pdf", job.id));
    if let Err(e) = processing::ocr::download_pdf(&s3_client, &bucket, &doc.filename, &local).await
    {
        error!(job_id=%job.id, "Failed to download PDF: {:?}", e);
        let _ = AnalysisJob::update_status(&pool, job.id, "failed").await;
        publish_status_event(job.id, job.org_id, "failed").await;
        return;
    }
    let mut txt_path = local.clone();
    txt_path.set_extension("txt");

    let job_timer = Instant::now();
    let res = run_stages(
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

    match res {
        Ok(_) => {
            let _ = AnalysisJob::update_status(&pool, job.id, "completed").await;
            publish_status_event(job.id, job.org_id, "completed").await;
            JOB_COUNTER.with_label_values(&["success"]).inc();
            JOB_HISTOGRAM
                .with_label_values(&["success"])
                .observe(job_timer.elapsed().as_secs_f64());
            info!(job_id=%job.id, "Job processing completed successfully.");
        }
        Err(e) => {
            error!(job_id=%job.id, "Job processing failed: {:?}", e);
            let _ = AnalysisJob::update_status(&pool, job.id, "failed").await;
            publish_status_event(job.id, job.org_id, "failed").await;
            JOB_COUNTER.with_label_values(&["failed"]).inc();
            JOB_HISTOGRAM
                .with_label_values(&["failed"])
                .observe(job_timer.elapsed().as_secs_f64());
        }
    }

    if local.exists() {
        remove_with_retry(&local, job.id, "input PDF").await;
    }
    if txt_path.exists() {
        remove_with_retry(&txt_path, job.id, "text file").await;
    }
    RUNNING_JOBS_GAUGE.dec();
}

#[tokio::main]
#[tracing::instrument(skip_all)]
async fn main() -> Result<()> {
    let cfg = match WorkerConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).json().init();

    let database_url = cfg.database_url;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let s3_client = S3Client::new(&shared_config);

    let client = redis::Client::open(cfg.redis_url)?;
    let mut conn = client.get_async_connection().await?;

    let process_once = cfg.process_one_job;
    spawn_metrics_server(cfg.metrics_port);

    let idle_duration = cfg.shutdown_after_idle.map(|m| Duration::from_secs(m * 60));
    let mut last_activity = Instant::now();
    let blpop_timeout = if idle_duration.is_some() { 60 } else { 0 };

    let mut shutdown_signal = signal::ctrl_c();
    tokio::pin!(shutdown_signal);

    let pool = Arc::new(pool);
    let s3_client = Arc::new(s3_client);
    let runtime_cfg = WorkerRuntimeConfig::from_env();
    let concurrency = Arc::new(AtomicUsize::new(runtime_cfg.concurrency.max(1)));
    tokio::spawn(worker::watch_config_changes(Arc::clone(&concurrency)));
    let mut tasks: JoinSet<()> = JoinSet::new();

    'outer: loop {
        if tasks.len() >= concurrency.load(Ordering::SeqCst) {
            tokio::select! {
                _ = &mut shutdown_signal => {
                    info!("Shutdown signal received");
                    break 'outer;
                }
                res = tasks.join_next() => {
                    if let Some(Err(e)) = res { error!("task failed: {:?}", e); }
                }
            }
            continue;
        }

        let mut cmd = redis::cmd("BLPOP");
        cmd.arg("jobs").arg(blpop_timeout);
        let job: Option<(String, String)> = tokio::select! {
            _ = &mut shutdown_signal => {
                info!("Shutdown signal received");
                break 'outer;
            }
            res = cmd.query_async::<_, Option<(String, String)>>(&mut conn) => {
                res?
            }
        };

        let Some((_, job_id_str)) = job else {
            if let Some(d) = idle_duration {
                if last_activity.elapsed() >= d && tasks.is_empty() {
                    worker::log_idle_shutdown();
                    break 'outer;
                }
            }
            if let Some(res) = tasks.join_next().await {
                if let Err(e) = res {
                    error!("task failed: {:?}", e);
                }
            }
            continue;
        };
        last_activity = Instant::now();
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
            .fetch_one(&*pool)
            .await?;
        let stages: Vec<Stage> = serde_json::from_value(pipeline.stages)?;
        let bucket = cfg.s3_bucket.clone();
        let pool_clone = Arc::clone(&pool);
        let s3_clone = Arc::clone(&s3_client);
        tasks.spawn(async move {
            process_job(pool_clone, s3_clone, job, doc, stages, org_settings, bucket).await;
        });

        if process_once {
            break;
        }
    }
    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            error!("task failed: {:?}", e);
        }
    }
    let _ = redis::cmd("QUIT").query_async::<_, ()>(&mut conn).await;
    Ok(())
}
