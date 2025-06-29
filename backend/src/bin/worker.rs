use std::{time::Duration, env, path::PathBuf};
use sqlx::{postgres::PgPoolOptions, PgPool};
use backend::models::{AnalysisJob, Pipeline, Document, OrgSettings};
use backend::processing;
use backend::worker::{self, Stage};
use tokio::time::sleep;
use uuid::Uuid;
use tokio::process::Command;
use tracing::{info, error, warn};
use serde_json::{self};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;

/// Prompt template used during AI stages.
#[derive(Deserialize, Debug, Clone)]
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
                error!(job_id=%job_id_str, "Failed to fetch job details: {:?}", e);
                continue;
            }
        };

        // Fetch OrgSettings for the job's organization
        let org_settings = match OrgSettings::find(&pool, job.org_id).await {
            Ok(settings) => Some(settings),
            Err(e) => {
                error!(job_id=%job.id, org_id=%job.org_id, "Failed to fetch org settings: {:?}", e);
                // Decide if job should fail or proceed with default/env settings
                // For now, proceeding with None, which will lead to env fallbacks
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
            info!(job_id=%job.id, stage=?stage.stage_type, command=?stage.command, prompt_name=?stage.prompt_name, ocr_engine=?stage.ocr_engine, "running stage");
            match stage.stage_type.as_str() {
                "ocr" => {
                    if worker::ocr::handle_ocr_stage(
                        &pool,
                        &s3_client,
                        &job,
                        &stage,
                        org_settings.as_ref(),
                        &bucket,
                        &local,
                        &txt_path,
                    ).await? {
                        break;
                    }
                }
                "parse" => {
                    if !txt_path.exists() {
                        warn!(job_id=%job.id, stage=%stage.stage_type, "Input text file {:?} not found for parse stage. Skipping.", txt_path);
                    } else {
                        match tokio::fs::read_to_string(&txt_path).await {
                            Ok(text_content) => {
                                match processing::run_parse_stage(&text_content, stage.config.as_ref()).await {
                                    Ok(parsed_val) => json_result = parsed_val,
                                    Err(e) => {
                                        error!(job_id=%job.id, stage=%stage.stage_type, "Parse stage failed: {:?}", e);
                                        AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!(job_id=%job.id, stage=%stage.stage_type, "Failed to read input text for parse stage: {:?}", e);
                                AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                break;
                            }
                        }
                    }
                    if let Ok(b) = serde_json::to_vec_pretty(&json_result) {
                        let _ = worker::save_stage_output(&pool, &s3_client, job.id, &stage.stage_type, "json", &bucket, b, "json").await;
                    }
                }
                "ai" => {
                    json_result = worker::ai::handle_ai_stage(
                        &pool,
                        &s3_client,
                        &job,
                        &stage,
                        org_settings.as_ref(),
                        &bucket,
                        json_result.clone(),
                        &local,
                    ).await?;
                }
                "report" => {
                    worker::report::handle_report_stage(
                        &pool,
                        &s3_client,
                        &job,
                        &doc,
                        &stage,
                        &bucket,
                        &json_result,
                        &local,
                    ).await?;
                }
                _ => {
                    if let Some(cmd) = stage.command.as_ref() {
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
        // Determine final job status - if it wasn't set to "failed" by a break, it's "completed"
        // This part needs to be careful: if a break happened, status is already "failed".
        // The AnalysisJob::update_status call after the loop should reflect the true outcome.
        // If the loop completed without break, it's completed. If break, it's failed.
        // The `break` statements already update status to "failed".
        // So, if we reach here *without* breaking, the job is "completed".
        // However, `AnalysisJob::update_status` is called with "failed" inside the loop on error.
        // We need to ensure "completed" is only set if no stage explicitly failed it.

        // A simple way: query current job status. If it's still "in_progress", then it completed.
        let current_job_status = AnalysisJob::find(&pool, job.id).await?.status;
        if current_job_status == "in_progress" {
             AnalysisJob::update_status(&pool, job.id, "completed").await?;
             info!(job_id=%job.id, "Job processing completed successfully.");
        } else {
            // Status was already set to "failed" by a stage that broke the loop.
            // Or it was already "completed" if some logic error occurred.
            info!(job_id=%job.id, "Job processing finished with status: {}", current_job_status);
        }

        // Cleanup the downloaded input PDF for the current job
        if local.exists() {
            if let Err(e) = tokio::fs::remove_file(&local).await {
                error!(job_id=%job.id, path=?local, "Failed to clean up input PDF: {:?}", e);
            } else {
                info!(job_id=%job.id, path=?local, "Cleaned up input PDF.");
            }
        }
        // Any other job-level temp files would be cleaned here.

        info!(job_id=%job.id, "Finished processing job lifecycle.");
        if process_once {
            break;
        }
    } // End of main loop
}
