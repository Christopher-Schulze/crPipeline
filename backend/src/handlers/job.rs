use crate::middleware::auth::AuthUser;
use crate::models::{AnalysisJob, Document, JobStageOutput, Pipeline};
use crate::models::analysis_job::JobWithNames;
use actix_web::{get, web, HttpResponse, http::StatusCode, ResponseError};
use crate::error::ApiError;
use actix_web_lab::sse::{self, ChannelStream, Sse};
use aws_sdk_s3::presigning::PresigningConfig;
use serde::Serialize;
use sqlx::PgPool;
use std::time::Duration; // Already here, used for Sse and now PresigningConfig
use uuid::Uuid; // For S3 presigned URLs

#[derive(serde::Deserialize)]
struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
}

/// Combined details returned by [`get_job_details`].
#[derive(Serialize)]
struct JobDetailsResponse {
    // From AnalysisJob
    id: Uuid,
    org_id: Uuid,
    document_id: Uuid,
    pipeline_id: Uuid,
    status: String,
    job_created_at: chrono::DateTime<chrono::Utc>,

    // From Document
    document_name: String,

    // From Pipeline
    pipeline_name: String,

    // From JobStageOutput
    stage_outputs: Vec<JobStageOutput>,
}

/// Return all jobs for an organization.
#[get("/jobs/{org_id}")]
async fn list_jobs(
    path: web::Path<Uuid>,
    query: web::Query<Pagination>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let org_id = *path;
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).max(1).min(100);
    let offset = (page - 1) * per_page;

    let total_items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM analysis_jobs WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(pool.as_ref())
        .await
        .unwrap_or(0);

    match sqlx::query_as::<_, JobWithNames>(
        r#"
        SELECT aj.*, d.display_name AS document_name, p.name AS pipeline_name
        FROM analysis_jobs aj
        JOIN documents d ON aj.document_id = d.id
        JOIN pipelines p ON aj.pipeline_id = p.id
        WHERE aj.org_id = $1
        ORDER BY aj.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(org_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(list) => {
            let total_pages = ((total_items as f64) / (per_page as f64)).ceil() as i64;
            HttpResponse::Ok().json(serde_json::json!({
                "items": list,
                "total_items": total_items,
                "page": page,
                "per_page": per_page,
                "total_pages": total_pages
            }))
        }
        Err(_) => ApiError::new("Failed to list jobs", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    }
}

/// Server-sent events stream sending job status updates.
#[get("/jobs/{id}/events")]
async fn job_events(path: web::Path<Uuid>, pool: web::Data<PgPool>) -> Sse<ChannelStream> {
    let (tx, rx) = sse::channel(10);
    let job_id = *path;
    actix_web::rt::spawn(async move {
        loop {
            match AnalysisJob::find(pool.as_ref(), job_id).await {
                Ok(job) => {
                    if tx.send(sse::Data::new(job.status.clone())).await.is_err() {
                        break;
                    }
                    if job.status == "completed" || job.status == "failed" {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(sse::Data::new("error")).await;
                    break;
                }
            }
            actix_web::rt::time::sleep(Duration::from_secs(2)).await;
        }
    });
    rx
}

/// Retrieve job metadata along with document and pipeline info.
#[get("/jobs/{job_id}/details")]
async fn get_job_details(
    path: web::Path<Uuid>,
    user: AuthUser, // For authorization
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let job_id = path.into_inner();

    // 1. Fetch AnalysisJob
    let job = match sqlx::query_as::<_, AnalysisJob>("SELECT * FROM analysis_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(j) => j,
        Err(sqlx::Error::RowNotFound) => {
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Job not found"}))
        }
        Err(e) => {
            log::error!("Failed to fetch job {}: {:?}", job_id, e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to fetch job details"}));
        }
    };

    // 2. Authorization: Ensure the user's org_id matches the job's org_id
    if job.org_id != user.org_id {
        log::warn!(
            "Unauthorized attempt to access job {} by user {} (org {} vs job org {})",
            job_id,
            user.user_id,
            user.org_id,
            job.org_id
        );
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "You are not authorized to view this job"}));
    }

    // 3. Fetch Document
    let document = match sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id = $1")
        .bind(job.document_id)
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(d) => d,
        Err(e) => {
            log::error!(
                "Failed to fetch document {} for job {}: {:?}",
                job.document_id,
                job_id,
                e
            );
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to fetch associated document"}));
        }
    };

    // 4. Fetch Pipeline
    let pipeline = match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE id = $1")
        .bind(job.pipeline_id)
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(p) => p,
        Err(e) => {
            log::error!(
                "Failed to fetch pipeline {} for job {}: {:?}",
                job.pipeline_id,
                job_id,
                e
            );
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to fetch associated pipeline"}));
        }
    };

    // 5. Fetch JobStageOutputs
    let stage_outputs = match JobStageOutput::find_by_job_id(pool.as_ref(), job_id).await {
        Ok(outputs) => outputs,
        Err(e) => {
            log::error!("Failed to fetch stage outputs for job {}: {:?}", job_id, e);
            // Not necessarily a critical error, can return empty list or an error depending on requirements
            // For now, returning empty and letting client know via logs. Could also return 500.
            vec![]
        }
    };

    // 6. Construct and return response
    let response = JobDetailsResponse {
        id: job.id,
        org_id: job.org_id,
        document_id: job.document_id,
        pipeline_id: job.pipeline_id,
        status: job.status,
        job_created_at: job.created_at, // Assuming AnalysisJob has created_at
        document_name: document.display_name, // Changed from document.filename
        pipeline_name: pipeline.name,
        stage_outputs,
    };

    HttpResponse::Ok().json(response)
}

/// Register job-related endpoints on the Actix configuration.
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_jobs)
        .service(job_events)
        .service(get_job_details)
        .service(get_stage_output_download_url); // Add the new service
}

/// Create a presigned URL for downloading an output file of a job stage.
#[get("/jobs/outputs/{output_id}/download_url")]
async fn get_stage_output_download_url(
    path: web::Path<Uuid>, // output_id from job_stage_outputs table
    user: AuthUser,
    pool: web::Data<PgPool>,
    s3_client: web::Data<aws_sdk_s3::Client>, // Get S3 client from app data
) -> HttpResponse {
    let output_id = path.into_inner();

    // 1. Fetch JobStageOutput record
    let stage_output =
        match sqlx::query_as::<_, JobStageOutput>("SELECT * FROM job_stage_outputs WHERE id = $1")
            .bind(output_id)
            .fetch_one(pool.as_ref())
            .await
        {
            Ok(so) => so,
            Err(sqlx::Error::RowNotFound) => {
                return HttpResponse::NotFound()
                    .json(serde_json::json!({"error": "Stage output not found"}))
            }
            Err(e) => {
                log::error!("Failed to fetch stage output {}: {:?}", output_id, e);
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to retrieve stage output details"}));
            }
        };

    // 2. Fetch associated AnalysisJob to get org_id for authorization
    // Selecting only necessary fields for efficiency
    let job_core_details = match sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT id, org_id FROM analysis_jobs WHERE id = $1",
    )
    .bind(stage_output.job_id)
    .fetch_one(pool.as_ref())
    .await
    {
        Ok((id, org_id_val)) => (id, org_id_val), // Destructure the tuple
        Err(e) => {
            log::error!(
                "Failed to fetch job {} for stage output {}: {:?}",
                stage_output.job_id,
                output_id,
                e
            );
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to verify job association"}));
        }
    };

    let job_org_id = job_core_details.1; // Get org_id from the tuple

    // 3. Authorization check
    if job_org_id != user.org_id {
        log::warn!(
            "Unauthorized attempt to access stage output {} (job {}) by user {} (org {} vs job org {})",
            output_id, stage_output.job_id, user.user_id, user.org_id, job_org_id
        );
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "You are not authorized to access this output."}));
    }

    // 4. Generate S3 pre-signed URL
    let presigning_config = match PresigningConfig::expires_in(Duration::from_secs(3600)) {
        // 1 hour expiry
        Ok(config) => config,
        Err(e) => {
            log::error!(
                "Failed to create presigning config for output {}: {:?}",
                output_id,
                e
            );
            return HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Could not generate download URL (config error)"}),
            );
        }
    };

    match s3_client
        .get_object()
        .bucket(&stage_output.s3_bucket)
        .key(&stage_output.s3_key)
        .presigned(presigning_config)
        .await
    {
        Ok(presigned_request) => {
            // Optional: log action for audit trail
            // log_action(&pool, user.org_id, user.user_id, "download_stage_output", Some(output_id), Some(serde_json::json!({"s3_key": stage_output.s3_key}))).await;
            HttpResponse::Ok()
                .json(serde_json::json!({ "url": presigned_request.uri().to_string() }))
        }
        Err(e) => {
            log::error!(
                "Failed to generate presigned URL for output {}: {:?}",
                output_id,
                e
            );
            HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Could not generate download URL (presign error)"}),
            )
        }
    }
}
