use std::{time::{Duration, SystemTime, UNIX_EPOCH}, env, path::PathBuf}; // Added SystemTime, UNIX_EPOCH
use sqlx::{postgres::PgPoolOptions, PgPool}; // Added PgPool for helper fn type
use backend::models::{AnalysisJob, Pipeline, Document, OrgSettings, NewJobStageOutput, JobStageOutput}; // Added JobStageOutput models
use backend::processing;
use tokio::time::sleep;
use aws_sdk_s3::primitives::ByteStream; // For uploading from bytes
use uuid::Uuid; // For helper fn type
use tokio::process::Command;
use tracing::{info, error};
use serde_json::{self, Value}; // Ensure Value is imported
use serde::Deserialize;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;

// Struct for deserializing prompt templates from OrgSettings
#[derive(Deserialize, Debug, Clone)]
struct PromptTemplate {
    name: String,
    text: String,
}

#[derive(Deserialize, Debug, Clone)] // Added Clone here
struct Stage {
    #[serde(rename = "type")]
    stage_type: String,
    command: Option<String>,
    prompt_name: Option<String>,

    // New fields for OCR stage specific configuration
    ocr_engine: Option<String>, // e.g., "default", "external"
    ocr_stage_endpoint: Option<String>,
    ocr_stage_key: Option<String>,

    // New generic config field for structured configurations
    config: Option<Value>,
}

// Helper function to save stage output
async fn save_stage_output(
    pool: &PgPool,
    s3_client: &S3Client,
    job_id: Uuid,
    stage_name: &str,
    output_type: &str, // "json", "pdf", "txt"
    s3_bucket_name: &str,
    content: Vec<u8>,
    file_extension: &str, // "json", "pdf", "txt"
) -> Result<(), anyhow::Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let s3_key = format!("jobs/{}/outputs/{}_{}.{}", job_id, stage_name, timestamp, file_extension);

    info!(job_id=%job_id, stage=%stage_name, s3_key=%s3_key, "Uploading stage output to S3.");
    s3_client.put_object()
        .bucket(s3_bucket_name)
        .key(&s3_key)
        .body(ByteStream::from(content))
        .send()
        .await?;
    info!(job_id=%job_id, stage=%stage_name, s3_key=%s3_key, "Successfully uploaded stage output to S3.");

    let new_output_db_record = NewJobStageOutput {
        job_id,
        stage_name: stage_name.to_string(),
        output_type: output_type.to_string(),
        s3_bucket: s3_bucket_name.to_string(),
        s3_key,
    };
    JobStageOutput::create(pool, new_output_db_record).await?;
    info!(job_id=%job_id, stage=%stage_name, "Successfully saved stage output metadata to DB.");
    Ok(())
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
            info!(job_id=%job.id, stage=?stage.stage_type, command=?stage.command, prompt_name=?stage.prompt_name, ocr_engine=?stage.ocr_engine, "running stage"); // Enhanced log
            match stage.stage_type.as_str() {
                "ocr" => {
                    let mut ocr_method_determined_and_executed = false; // Tracks if any OCR method ran successfully or failed critically
                    let mut critical_ocr_failure = false; // Tracks if a chosen OCR method critically failed

                    // txt_path is defined earlier in the loop:
                    // let mut txt_path = local.clone();
                    // txt_path.set_extension("txt");

                    // 1. Stage-defined custom command (Highest Priority)
                    if let Some(cmd_str) = stage.command.as_ref().filter(|s| !s.trim().is_empty()) {
                        tracing::debug!(job_id=%job.id, "OCR stage: Attempting STAGE-LEVEL custom command: {}", cmd_str);
                        ocr_method_determined_and_executed = true; // We are attempting a method

                        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
                        if let Some(program) = parts.first() {
                            let args = parts[1..].iter().map(|arg| {
                                arg.replace("{{input_pdf}}", &local.to_string_lossy())
                                   .replace("{{output_txt}}", &txt_path.to_string_lossy())
                            }).collect::<Vec<String>>();

                            match Command::new(program).args(args).status().await {
                                Ok(status) if status.success() => {
                                    info!(job_id=%job.id, "OCR stage: Custom OCR command completed successfully. Output expected at {:?}", txt_path);
                                }
                                Ok(status) => {
                                    tracing::error!(job_id=%job.id, "OCR stage: Custom OCR command failed with status: {}", status);
                                    AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                    if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}", e_c); }}
                                    critical_ocr_failure = true;
                                }
                                Err(e) => {
                                    tracing::error!(job_id=%job.id, "OCR stage: Failed to execute custom OCR command: {:?}", e);
                                    AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                    if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}", e_c); }}
                                    critical_ocr_failure = true;
                                }
                            }
                        } else {
                            tracing::error!(job_id=%job.id, "OCR stage: Custom OCR command is empty or invalid.");
                            AnalysisJob::update_status(&pool, job.id, "failed").await?;
                            if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}", e_c); }}
                            critical_ocr_failure = true;
                        }
                    }

                    // Helper closure for external OCR execution
                    let execute_external_ocr = |endpoint: &String, key_opt: Option<&String>, source_log: &str| async {
                        match tokio::fs::read(&local).await {
                            Ok(file_bytes) => {
                                let original_filename = local.file_name().unwrap_or_default().to_string_lossy().into_owned();
                                tracing::debug!(job_id=%job.id, "OCR stage: Calling run_external_ocr for {} (Source: {})", endpoint, source_log);
                                match processing::run_external_ocr(endpoint, key_opt.map(String::as_str), file_bytes, &original_filename).await {
                                    Ok(ocr_text_result) => {
                                        if let Err(e_write) = tokio::fs::write(&txt_path, ocr_text_result).await {
                                            tracing::error!(job_id=%job.id, path=?txt_path, "OCR stage: Failed to write external OCR ({}) result: {:?}", source_log, e_write);
                                            Err(anyhow::anyhow!("Failed to write external OCR result from {}", source_log))
                                        } else {
                                            info!(job_id=%job.id, "OCR stage: External OCR ({}) successful. Output at {:?}", source_log, txt_path);
                                            Ok(())
                                        }
                                    }
                                    Err(e_ocr) => {
                                        tracing::error!(job_id=%job.id, "OCR stage: External OCR ({}) processing failed: {:?}", source_log, e_ocr);
                                        Err(e_ocr.context(format!("External OCR ({}) processing failed", source_log)))
                                    }
                                }
                            }
                            Err(e_read) => {
                                tracing::error!(job_id=%job.id, path=?local, "OCR stage: Failed to read input PDF for external OCR ({}): {:?}", source_log, e_read);
                                Err(anyhow::Error::from(e_read).context(format!("Failed to read input PDF for external OCR ({})", source_log)))
                            }
                        }
                    };

                    // 2. Stage-defined External OCR
                    if !ocr_method_determined_and_executed && !critical_ocr_failure && stage.ocr_engine.as_deref() == Some("external") {
                        if let Some(endpoint) = stage.ocr_stage_endpoint.as_ref().filter(|s| !s.trim().is_empty()) {
                            tracing::debug!(job_id=%job.id, "OCR stage: Attempting STAGE-LEVEL external OCR. Endpoint: {}", endpoint);
                            ocr_method_determined_and_executed = true;
                            if let Err(_e) = execute_external_ocr(endpoint, stage.ocr_stage_key.as_ref(), "Stage").await {
                                AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}", e_c); }}
                                critical_ocr_failure = true;
                            }
                        } else {
                            tracing::warn!(job_id=%job.id, "OCR stage: Stage-level 'external' OCR selected, but endpoint is missing/empty. Falling back.");
                            // Note: ocr_method_determined_and_executed remains false, allowing fallback
                        }
                    }

                    // 3. Organization-level External OCR
                    if !ocr_method_determined_and_executed && !critical_ocr_failure {
                        if let Some(ref settings) = org_settings {
                            if let Some(endpoint) = settings.ocr_api_endpoint.as_ref().filter(|s| !s.trim().is_empty()) {
                                if stage.ocr_engine.as_deref() != Some("default") {
                                    tracing::debug!(job_id=%job.id, "OCR stage: Attempting ORGANIZATION-LEVEL external OCR. Endpoint: {}", endpoint);
                                    ocr_method_determined_and_executed = true;
                                    if let Err(_e) = execute_external_ocr(endpoint, settings.ocr_api_key.as_ref(), "Organization").await {
                                        AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                        if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}", e_c); }}
                                        critical_ocr_failure = true;
                                    }
                                } else {
                                    tracing::debug!(job_id=%job.id, "OCR stage: stage.ocr_engine is 'default', skipping org-level external OCR.");
                                }
                            }
                        }
                    }

                    // 4. Global Environment Variable External OCR
                    if !ocr_method_determined_and_executed && !critical_ocr_failure {
                        if let Ok(endpoint) = env::var("DEFAULT_EXTERNAL_OCR_ENDPOINT").filter(|s| !s.trim().is_empty()) {
                            if stage.ocr_engine.as_deref() != Some("default") {
                                tracing::debug!(job_id=%job.id, "OCR stage: Attempting GLOBAL ENV external OCR. Endpoint: {}", endpoint);
                                ocr_method_determined_and_executed = true;
                                let env_ocr_key = env::var("DEFAULT_EXTERNAL_OCR_API_KEY").ok();
                                if let Err(_e) = execute_external_ocr(&endpoint, env_ocr_key.as_ref(), "Global Env").await {
                                    AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                    if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}", e_c); }}
                                    critical_ocr_failure = true;
                                }
                            } else {
                                tracing::debug!(job_id=%job.id, "OCR stage: stage.ocr_engine is 'default', skipping global ENV external OCR.");
                            }
                        }
                    }

                    // 5. Default Local Tesseract
                    if !ocr_method_determined_and_executed && !critical_ocr_failure {
                        tracing::debug!(job_id=%job.id, "OCR stage: Using DEFAULT LOCAL Tesseract. Input: {:?}, Output: {:?}", local, txt_path);
                        ocr_method_determined_and_executed = true; // Considered attempted even if it's the last resort
                        if let Err(e) = processing::run_ocr(&local, &txt_path).await {
                            tracing::error!(job_id=%job.id, "OCR stage: Default local OCR (Tesseract) failed: {:?}", e);
                            AnalysisJob::update_status(&pool, job.id, "failed").await?;
                            if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}", e_c); }}
                            critical_ocr_failure = true;
                        } else {
                            info!(job_id=%job.id, "OCR stage: Default local OCR (Tesseract) completed.");
                        }
                    }

                    // If OCR task failed critically in any of the above steps, break from stage loop
                    if critical_ocr_failure {
                        // Job status and local PDF cleanup already handled in the failure block
                        break; // Exit this job's stage processing
                    }

                    // Save output from txt_path
                    if txt_path.exists() {
                        match tokio::fs::read_to_string(&txt_path).await {
                            Ok(ocr_content_string) => {
                                if let Err(e) = save_stage_output(
                                    &pool, &s3_client, job.id, &stage.stage_type, "txt", &bucket, ocr_content_string.into_bytes(), "txt"
                                ).await {
                                    tracing::warn!(job_id=%job.id, stage=%stage.stage_type, "Failed to save OCR stage output (from txt_path): {:?}", e);
                                } else {
                                    info!(job_id=%job.id, stage=%stage.stage_type, "OCR stage output (from txt_path) saved.");
                                    // Clean up local OCR text file (txt_path)
                                    if let Err(e) = tokio::fs::remove_file(&txt_path).await {
                                        tracing::warn!(job_id=%job.id, path=?txt_path, "Failed to clean up OCR text file: {:?}", e);
                                    } else {
                                        info!(job_id=%job.id, path=?txt_path, "Cleaned up OCR text file.");
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(job_id=%job.id, path=?txt_path, "OCR stage: Failed to read OCR output from txt_path for saving: {:?}. Output might not have been generated if OCR failed silently without creating a file.", e);
                            }
                        }
                    } else {
                        tracing::warn!(job_id=%job.id, path=?txt_path, "OCR stage: Output file txt_path does not exist. Skipping save_stage_output for OCR.");
                        // This implies an OCR method was chosen but failed to produce an output file,
                        // and it wasn't deemed a critical_ocr_failure (e.g. custom command that exits 0 but doesn't write file).
                        // Or, if ocr_method_determined_and_executed is false, it means no method was even attempted (should not happen with current logic).
                        if ocr_method_determined_and_executed && !critical_ocr_failure {
                             tracing::warn!(job_id=%job.id, "OCR method was executed but produced no output file and was not marked critical. This might be unexpected.");
                        }
                    }
                },
                "parse" => {
                    if !txt_path.exists() {
                        tracing::warn!(job_id=%job.id, stage=%stage.stage_type, "Input text file {:?} not found for parse stage. Skipping.", txt_path);
                        // Consider if json_result should be explicitly nulled or if job should fail
                        // json_result = Value::Null; // Example if clearing is desired
                    } else {
                        match tokio::fs::read_to_string(&txt_path).await {
                            Ok(text_content) => {
                                tracing::debug!(job_id=%job.id, stage=%stage.stage_type, "Read text content from {:?} for parsing.", txt_path);
                                match processing::run_parse_stage(&text_content, stage.config.as_ref()).await {
                                    Ok(parsed_val) => {
                                        json_result = parsed_val; // Update the main json_result
                                        info!(job_id=%job.id, stage=%stage.stage_type, "Parse stage completed.");
                                    }
                                    Err(e) => {
                                        tracing::error!(job_id=%job.id, stage=%stage.stage_type, "Parse stage processing failed: {:?}", e);
                                        AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                        if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}",e_c);}}
                                        break; // Critical failure for this stage
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(job_id=%job.id, stage=%stage.stage_type, "Failed to read input text from {:?} for parse stage: {:?}", txt_path, e);
                                AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}",e_c);}}
                                break; // Critical failure
                            }
                        }
                    }
                    // Save the output of the parse stage (json_result)
                    match serde_json::to_vec_pretty(&json_result) {
                        Ok(output_bytes) => {
                            if let Err(e) = save_stage_output(
                                &pool, &s3_client, job.id, &stage.stage_type, "json", &bucket, output_bytes, "json"
                            ).await {
                                tracing::warn!(job_id=%job.id, stage=%stage.stage_type, "Failed to save Parse stage output: {:?}", e);
                            } else {
                                info!(job_id=%job.id, stage=%stage.stage_type, "Parse stage output saved.");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(job_id=%job.id, stage=%stage.stage_type, "Failed to serialize Parse stage output for saving: {:?}", e);
                        }
                    }
                },
                "ai" => {
                    let ai_endpoint_to_use: String;
                    let ai_key_to_use: String;

                    if let Some(ref settings) = org_settings {
                        // Prefer org-specific endpoint if available and not empty
                        ai_endpoint_to_use = settings.ai_api_endpoint.as_ref()
                            .filter(|s| !s.trim().is_empty())
                            .map(|s| s.clone())
                            .unwrap_or_else(|| {
                                env::var("AI_API_URL").unwrap_or_else(|_| {
                                    error!(job_id=%job.id, org_id=%job.org_id, "AI_API_URL not set and org endpoint is empty or not set.");
                                    String::new()
                                })
                            });

                        // Prefer org-specific key if available and not empty
                        ai_key_to_use = settings.ai_api_key.as_ref()
                            .filter(|s| !s.trim().is_empty())
                            .map(|s| s.clone())
                            .unwrap_or_else(|| {
                                env::var("AI_API_KEY").unwrap_or_else(|_| {
                                    error!(job_id=%job.id, org_id=%job.org_id, "AI_API_KEY not set and org key is empty or not set.");
                                    String::new()
                                })
                            });
                    } else {
                        // Fallback to environment variables if org_settings couldn't be fetched
                        ai_endpoint_to_use = env::var("AI_API_URL").unwrap_or_else(|_| {
                            error!(job_id=%job.id, "AI_API_URL not set (no org settings).");
                            String::new()
                        });
                        ai_key_to_use = env::var("AI_API_KEY").unwrap_or_else(|_| {
                            error!(job_id=%job.id, "AI_API_KEY not set (no org settings).");
                            String::new()
                        });
                    }

                    if ai_endpoint_to_use.is_empty() {
                        error!(job_id=%job.id, "AI endpoint is not configured. Skipping AI stage.");
                        AnalysisJob::update_status(&pool, job.id, "failed").await?; // Mark job as failed due to config error
                        info!(job_id=%job.id, "Job failed due to missing AI configuration.");
                        // Depending on desired behavior, might 'continue' to next job or 'break' loop.
                        // For now, let it proceed to job completion which will reflect the failed status.
                        break;
                    }
                    // Note: An empty key might be permissible for some APIs, so we don't strictly check ai_key_to_use.is_empty() here
                    // unless the API specifically requires it and run_ai would fail informatively.

                    let mut final_ai_input = json_result.clone(); // Default to previous stage's output

                    if let Some(prompt_name_to_use) = stage.prompt_name.as_ref() {
                        if !prompt_name_to_use.trim().is_empty() {
                            let mut prompt_found_and_used = false;
                            if let Some(ref settings) = org_settings {
                                if let Some(Value::Array(templates_json_array)) = settings.prompt_templates.as_ref() {
                                    let prompt_templates: Vec<PromptTemplate> = templates_json_array.iter()
                                        .filter_map(|v| serde_json::from_value(v.clone()).ok())
                                        .collect();

                                    if let Some(found_template) = prompt_templates.iter().find(|p| &p.name == prompt_name_to_use) {
                                        info!(job_id=%job.id, "Using prompt template: {} for AI stage.", found_template.name);

                                        let previous_json_string = serde_json::to_string(&json_result).unwrap_or_default();
                                        let prompt_text = found_template.text
                                            .replace("{{json_input}}", &previous_json_string)
                                            .replace("{{content}}", &previous_json_string); // Allow both common placeholders

                                        // Construct the input for the AI service
                                        // This structure depends on what `processing::run_ai` and the AI service expect.
                                        // Option 1: Send only the formatted prompt if the AI service handles context itself.
                                        // final_ai_input = serde_json::Value::String(prompt_text);

                                        // Option 2: Send a structured JSON if AI service or run_ai expects it.
                                        final_ai_input = serde_json::json!({
                                            "prompt": prompt_text,
                                            "context_data": json_result // Pass original context too
                                        });
                                        prompt_found_and_used = true;
                                    } else {
                                        warn!(job_id=%job.id, "Prompt template '{}' not found in org settings. Using default AI input (previous stage output).", prompt_name_to_use);
                                    }
                                } else {
                                     warn!(job_id=%job.id, "No prompt templates defined in org settings, but prompt '{}' was specified. Using default AI input.", prompt_name_to_use);
                                }
                            } else {
                                warn!(job_id=%job.id, "Org settings not available, cannot use prompt template '{}'. Using default AI input.", prompt_name_to_use);
                            }
                            // If prompt_found_and_used is false, final_ai_input remains the original json_result
                        }
                        // If stage.prompt_name is None or empty, final_ai_input also remains json_result
                    }

                    let custom_headers_ref = org_settings.as_ref().and_then(|s| s.ai_custom_headers.as_ref());

                    // 1. Save the input to the AI stage (final_ai_input)
                    match serde_json::to_vec_pretty(&final_ai_input) {
                        Ok(input_bytes) => {
                            let ai_input_stage_name = format!("{}_input", stage.stage_type);
                            if let Err(e) = save_stage_output(
                                &pool,
                                &s3_client,
                                job.id,
                                &ai_input_stage_name,
                                "json", // output_type
                                &bucket, // s3_bucket_name
                                input_bytes,
                                "json", // file_extension
                            ).await {
                                // Use tracing::warn! as error! from log crate might not be in scope here if only tracing::error is used.
                                // Or ensure log::warn is available. For now, using tracing::warn.
                                tracing::warn!(job_id=%job.id, stage_name=%ai_input_stage_name, "Failed to save AI stage input to S3/DB: {:?}", e);
                                // Non-critical, job can proceed
                            } else {
                                info!(job_id=%job.id, stage_name=%ai_input_stage_name, "AI stage input saved successfully.");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(job_id=%job.id, stage_name=format!("{}_input", stage.stage_type), "Failed to serialize AI stage input for saving: {:?}", e);
                        }
                    }

                    // 2. Call processing::run_ai
                    let ai_processing_result = match processing::run_ai(
                        &final_ai_input,
                        &ai_endpoint_to_use,
                        &ai_key_to_use,
                        custom_headers_ref,
                    ).await {
                        Ok(res) => res,
                        Err(e) => {
                            let input_str = serde_json::to_string_pretty(&final_ai_input).unwrap_or_else(|_| "Failed to serialize AI input".to_string());
                            const MAX_LOG_LEN: usize = 512;
                            let truncated_input = if input_str.len() > MAX_LOG_LEN { format!("{}...", &input_str[..MAX_LOG_LEN]) } else { input_str };
                            error!(job_id=%job.id, stage=%stage.stage_type, ai_input=%truncated_input, "AI stage failed: {:?}", e);
                            AnalysisJob::update_status(&pool, job.id, "failed").await?;
                            // Cleanup local input PDF before breaking if this critical stage fails
                            if local.exists() {
                                if let Err(cleanup_err) = tokio::fs::remove_file(&local).await {
                                    error!(job_id=%job.id, path=?local, "Failed to clean up input PDF after AI stage failure: {:?}", cleanup_err);
                                } else {
                                    info!(job_id=%job.id, path=?local, "Cleaned up input PDF after AI stage failure.");
                                }
                            }
                            break;
                        }
                    };
                    json_result = ai_processing_result; // Assign result back

                    // 3. Save the output of the AI stage (json_result)
                    match serde_json::to_vec_pretty(&json_result) { // Use to_vec_pretty here
                        Ok(output_bytes) => {
                            if let Err(e) = save_stage_output(
                                &pool, &s3_client, job.id, &stage.stage_type, "json", &bucket, output_bytes, "json"
                            ).await {
                                error!(job_id=%job.id, stage=%stage.stage_type, "Failed to save AI output: {:?}", e);
                            }
                        }
                        Err(e) => {
                            error!(job_id=%job.id, stage=%stage.stage_type, "Failed to serialize AI output for saving: {:?}", e);
                        }
                    }
                },
                "report" => {
                    // Deserialize ReportStageConfig from stage.config
                    // Note: Need to import ReportStageConfig or define it locally if not accessible
                    // Assuming ReportStageConfig is not directly accessible, let's define a local minimal one for deserialization.
                    #[derive(Deserialize, Debug)]
                    #[serde(rename_all = "camelCase")]
                    struct ReportStageConfigLocal {
                        template: String,
                        #[serde(default)]
                        summary_fields: Vec<String>,
                    }

                    let mut report_config: Option<ReportStageConfigLocal> = None;
                    if let Some(config_val) = stage.config.as_ref() {
                        match serde_json::from_value::<ReportStageConfigLocal>(config_val.clone()) {
                            Ok(cfg) => report_config = Some(cfg),
                            Err(e) => {
                                tracing::warn!(job_id=%job.id, "Failed to parse report stage config: {:?}. Using default report.", e);
                            }
                        }
                    }

                    // Prepare data for templating - merge job/doc details with json_result from previous stage
                    let mut data_for_templating = json_result.clone();
                    if let Value::Object(ref mut map) = data_for_templating {
                        map.insert("document_name".to_string(), Value::String(doc.filename.clone()));
                        map.insert("job_id".to_string(), Value::String(job.id.to_string()));
                        // Add other relevant job/doc metadata as needed
                    } else if data_for_templating.is_null() || !data_for_templating.is_object() {
                        data_for_templating = serde_json::json!({
                            "document_name": doc.filename.clone(),
                            "job_id": job.id.to_string(),
                            "previous_stage_output": json_result.clone()
                        });
                    }

                    let pdf_out_path = std::env::temp_dir().join(format!("{}_report_temp.pdf", job.id));

                    if let Some(cfg) = report_config {
                        info!(job_id=%job.id, "Report stage: Using custom template.");
                        match processing::generate_report_from_template(&cfg.template, &data_for_templating, &pdf_out_path).await {
                            Ok(_) => { info!(job_id=%job.id, "Custom report generated successfully to {:?}", pdf_out_path); }
                            Err(e) => {
                                tracing::error!(job_id=%job.id, "Failed to generate custom report: {:?}. Attempting basic report.", e);
                                if let Err(e_basic) = processing::generate_report(&data_for_templating, &pdf_out_path) {
                                    tracing::error!(job_id=%job.id, "Basic report generation also failed: {:?}", e_basic);
                                    AnalysisJob::update_status(&pool, job.id, "failed").await?;
                                    if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}",e_c);}}
                                    break;
                                }
                            }
                        }

                        if !cfg.summary_fields.is_empty() {
                            let mut summary_map = serde_json::Map::new();
                            for field_path_str in cfg.summary_fields {
                                // Use jsonpath_rust::JsonPathQuery; - ensure it's in scope if not already.
                                // For direct use of path method on JsonValue:
                                use jsonpath_rust::JsonPathQuery; // Temporary import for this block
                                match data_for_templating.path(&format!("$.{}", field_path_str)) {
                                    Ok(nodes) => {
                                        if let Some(node) = nodes.first() {
                                            summary_map.insert(field_path_str.split('.').last().unwrap_or(&field_path_str).to_string(), (*node).clone());
                                        }
                                    }
                                    Err(e) => tracing::warn!(job_id=%job.id, "JSONPath error for summary field '{}': {:?}", field_path_str, e),
                                }
                            }
                            let summary_json_value = Value::Object(summary_map);
                            match serde_json::to_vec_pretty(&summary_json_value) {
                                Ok(summary_bytes) => {
                                    if let Err(e) = save_stage_output(&pool, &s3_client, job.id, "report_summary", "json", &bucket, summary_bytes, "json").await {
                                        tracing::warn!(job_id=%job.id, "Failed to save report summary JSON: {:?}", e);
                                    } else {
                                        info!(job_id=%job.id, "Report summary JSON saved.");
                                    }
                                }
                                Err(e) => tracing::warn!(job_id=%job.id, "Failed to serialize report summary JSON: {:?}", e),
                            }
                        }

                    } else {
                        info!(job_id=%job.id, "Report stage: No custom template. Using basic report generation.");
                        if let Err(e) = processing::generate_report(&data_for_templating, &pdf_out_path) {
                            tracing::error!(job_id=%job.id, "Basic report generation failed: {:?}", e);
                            AnalysisJob::update_status(&pool, job.id, "failed").await?;
                            if local.exists() { if let Err(e_c) = tokio::fs::remove_file(&local).await { tracing::error!(job_id=%job.id, "Cleanup error for input PDF: {:?}",e_c);}}
                            break;
                        }
                    }

                    if pdf_out_path.exists() {
                        let report_s3_key = format!("jobs/{}/outputs/{}-report.pdf", job.id, job.id);
                        match tokio::fs::read(&pdf_out_path).await {
                            Ok(pdf_bytes) => {
                                if let Err(e) = s3_client.put_object().bucket(&bucket).key(&report_s3_key).body(pdf_bytes.into()).send().await {
                                    tracing::error!(job_id=%job.id, "Failed to upload final report PDF to S3: {:?}", e);
                                } else {
                                    info!(job_id=%job.id, "Final report PDF uploaded to S3 key: {}", report_s3_key);
                                    let report_output_record = NewJobStageOutput {
                                        job_id: job.id,
                                        stage_name: stage.stage_type.clone(),
                                        output_type: "pdf".to_string(),
                                        s3_bucket: bucket.clone(),
                                        s3_key: report_s3_key,
                                    };
                                    if let Err(e) = JobStageOutput::create(&pool, report_output_record).await {
                                        tracing::warn!(job_id=%job.id, "Failed to save final report PDF metadata to DB: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => tracing::error!(job_id=%job.id, "Failed to read generated report PDF from disk for S3 upload: {:?}", e),
                        }
                        if let Err(e) = tokio::fs::remove_file(&pdf_out_path).await { tracing::warn!(job_id=%job.id, path=?pdf_out_path, "Failed to clean up generated report PDF: {:?}", e); }
                    } else {
                        tracing::warn!(job_id=%job.id, "Report PDF was not generated at {:?}, skipping S3 upload.", pdf_out_path);
                    }
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
        } // End of for stage in stages loop

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
    } // End of main loop
}
