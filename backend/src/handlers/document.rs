use actix_multipart::Multipart;
use actix_web::{post, web, HttpResponse};
use futures_util::StreamExt as _;
use aws_sdk_s3::Client;
use uuid::Uuid;
use lopdf::Document as PdfDoc;
use crate::models::{Document, NewDocument, AnalysisJob, NewAnalysisJob, OrgSettings, DocumentError};
use crate::utils::log_action;
use crate::middleware::auth::AuthUser;
use sqlx::PgPool;
use sanitize_filename; // Added for sanitizing filenames
use anyhow::Error;
use async_trait::async_trait;
use redis::AsyncCommands;

/// Abstraction over S3 deletion used for easier testing.
#[async_trait]
pub trait S3Deleter {
    async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), Error>;
}

#[async_trait]
impl S3Deleter for Client {
    async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), Error> {
        self.delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| Error::new(e))
    }
}

pub async fn cleanup_s3_object<S: S3Deleter + Sync>(s3: &S, bucket: &str, key: &str) {
    if let Err(e) = s3.delete_object(bucket, key).await {
        log::error!("Failed to delete {} from S3 bucket {} during cleanup: {:?}", key, bucket, e);
    }
}

#[derive(serde::Deserialize)]
pub struct UploadParams {
    pub org_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub is_target: Option<bool>,
}

const PDF_MAGIC_BYTES: &[u8] = b"%PDF-";

async fn check_upload_quota(pool: &PgPool, org_id: Uuid) -> Result<(), HttpResponse> {
    match OrgSettings::find(pool, org_id).await {
        Ok(settings) => {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM documents WHERE org_id=$1 AND is_target=true AND upload_date >= date_trunc('month', NOW())"
            )
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or((0,));
            if count >= settings.monthly_upload_quota as i64 {
                return Err(HttpResponse::TooManyRequests().json(serde_json::json!({"error": "Monthly upload quota for target documents exceeded."})));
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Could not verify organization settings for quota (org_id {}): {:?}", org_id, e);
            Err(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Could not verify organization settings for quota."})))
        }
    }
}

async fn check_analysis_quota(pool: &PgPool, org_id: Uuid) -> Result<(), HttpResponse> {
    match OrgSettings::find(pool, org_id).await {
        Ok(settings) => {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM analysis_jobs WHERE org_id=$1 AND created_at >= date_trunc('month', NOW())"
            )
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or((0,));
            if count >= settings.monthly_analysis_quota as i64 {
                return Err(HttpResponse::TooManyRequests().json(serde_json::json!({"error": "Monthly analysis quota exceeded. Document uploaded but not queued for analysis."})));
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Could not verify organization settings for analysis quota (org_id {}): {:?}", org_id, e);
            Err(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Could not verify organization settings for analysis quota."})))
        }
    }
}

async fn upload_to_s3(
    s3: &Client,
    bucket: &str,
    key: &str,
    bytes: Vec<u8>,
) -> Result<(), HttpResponse> {
    if s3
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(bytes.into())
        .send()
        .await
        .is_err()
    {
        log::error!("Failed to upload {} to S3 bucket {}", key, bucket);
        Err(HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "File upload to storage failed."})))
    } else {
        Ok(())
    }
}

const MAX_FILE_SIZE: usize = 200 * 1024 * 1024; // 200MB

async fn validate_document(
    user_filename: &str,
    file_content_type: &Option<String>,
    bytes_data: &[u8],
) -> Result<(String, i32), HttpResponse> {
    let base_filename = if let Some(f_name) = std::path::Path::new(user_filename)
        .file_name()
        .and_then(|s| s.to_str())
    {
        f_name.to_string()
    } else {
        user_filename.to_string()
    };

    if base_filename.is_empty() {
        return Err(HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Filename not provided or invalid."})));
    }
    if bytes_data.is_empty() {
        return Err(
            HttpResponse::BadRequest().json(serde_json::json!({"error": "File content is empty."}))
        );
    }
    if bytes_data.len() > MAX_FILE_SIZE {
        return Err(HttpResponse::PayloadTooLarge()
            .json(serde_json::json!({"error": "File size exceeds the 200MB limit."})));
    }

    let lower_filename = base_filename.to_lowercase();
    let detected_file_type = if lower_filename.ends_with(".pdf") {
        if let Some(ref ct) = file_content_type {
            if ct != "application/pdf" {
                if !ct.starts_with("application/octet-stream") {
                    log::warn!(
                        "PDF upload for '{}': Mismatch Content-Type: {:?}, expected application/pdf or application/octet-stream",
                        user_filename,
                        file_content_type
                    );
                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid Content-Type for PDF file. Expected 'application/pdf'."
                    })));
                }
            }
        }
        if !bytes_data.starts_with(PDF_MAGIC_BYTES) {
            log::warn!("Invalid PDF magic bytes for file '{}'", user_filename);
            return Err(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid PDF file format (magic bytes mismatch)."})));
        }
        "pdf"
    } else if lower_filename.ends_with(".md") {
        if file_content_type.as_deref().map_or(false, |ct| {
            ct != "text/markdown"
                && ct != "text/plain"
                && !ct.starts_with("application/octet-stream")
        }) {
            log::warn!(
                "MD upload for '{}': Suspicious Content-Type: {:?}. Allowing.",
                user_filename,
                file_content_type
            );
        }
        "md"
    } else if lower_filename.ends_with(".txt") {
        if file_content_type.as_deref().map_or(false, |ct| {
            ct != "text/plain" && !ct.starts_with("application/octet-stream")
        }) {
            log::warn!(
                "TXT upload for '{}': Suspicious Content-Type: {:?}. Allowing.",
                user_filename,
                file_content_type
            );
        }
        "txt"
    } else {
        return Err(HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Unsupported file type. Only .pdf, .md, .txt are allowed."})));
    };

    let pages = if detected_file_type == "pdf" {
        match PdfDoc::load_mem(bytes_data).map(|d| d.get_pages().len() as i32) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to parse PDF pages for {}: {:?}", user_filename, e);
                return Err(HttpResponse::BadRequest().json(serde_json::json!({"error": "Corrupt or invalid PDF file. Could not count pages."})));
            }
        }
    } else {
        0
    };

    Ok((base_filename, pages))
}

#[post("/upload")]
pub async fn upload(
    mut payload: Multipart,
    params: web::Query<UploadParams>,
    user: AuthUser,
    pool: web::Data<sqlx::PgPool>,
    s3: web::Data<Client>,
) -> HttpResponse {
    let mut user_provided_filename = String::new();
    let mut file_content_type: Option<String> = None;
    let mut bytes_data = Vec::new();

    // Read file
    while let Some(Ok(mut field)) = payload.next().await {
        if let Some(name) = field.content_disposition().get_filename() {
            user_provided_filename = name.to_string();
        }
        if let Some(content_type) = field.content_type() {
            file_content_type = Some(content_type.to_string());
        }

        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => bytes_data.extend_from_slice(&data),
                Err(e) => {
                    log::error!("Error reading chunk from multipart field: {:?}", e);
                    return HttpResponse::BadRequest()
                        .json(serde_json::json!({"error": "Error reading uploaded file."}));
                }
            }
        }
    }

    // Validate file and get PDF page count
    let (base_filename, pages) = match validate_document(
        &user_provided_filename,
        &file_content_type,
        &bytes_data,
    ).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    // Authz check
    if params.org_id != user.org_id && user.role != "admin" {
        log::warn!(
            "User {} (org_id {}) attempted to upload to org_id {} without admin rights.",
            user.user_id,
            user.org_id,
            params.org_id
        );
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "You are not authorized to upload to this organization."}));
    }

    // Quota check (target docs)
    if params.is_target.unwrap_or(false) {
        if let Err(resp) = check_upload_quota(&pool, params.org_id).await {
            return resp;
        }
    }

    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
    let sanitized_filename_part = sanitize_filename::sanitize(&base_filename);
    let s3_key_name = format!("{}-{}", Uuid::new_v4(), sanitized_filename_part);

    if let Err(resp) = upload_to_s3(s3.get_ref(), &bucket, &s3_key_name, bytes_data.clone()).await {
        return resp;
    }

    let doc_to_create = NewDocument {
        org_id: params.org_id,
        owner_id: user.user_id,
        filename: s3_key_name.clone(),
        display_name: user_provided_filename,
        pages,
        is_target: params.is_target.unwrap_or(false),
        expires_at: None,
    };

    let created_document = match Document::create(&pool, doc_to_create).await {
        Ok(d) => d,
        Err(DocumentError::SanitizationFailed) => {
            log::warn!("Rejected unsafe filename during document creation: {}", s3_key_name);
            cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid filename."}));
        }
        Err(DocumentError::Sqlx(e)) => {
            log::error!("Failed to create document record for S3 key {}: {:?}", s3_key_name, e);
            cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to save document information."}));
        }
    };

    log_action(&pool, user.org_id, user.user_id, &format!("upload:{}", created_document.id)).await;

    // Optional: Queue for analysis
    if let Some(pipeline_id) = params.pipeline_id {
        if let Err(resp) = check_analysis_quota(&pool, params.org_id).await {
            cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
            return resp;
        }
        let job_to_create = NewAnalysisJob {
            org_id: params.org_id,
            document_id: created_document.id,
            pipeline_id,
            status: "pending".into(),
        };
        match AnalysisJob::create(&pool, job_to_create).await {
            Ok(j) => {
                log_action(&pool, user.org_id, user.user_id, &format!("job_created:{}", j.id)).await;
                if let Ok(redis_url) = std::env::var("REDIS_URL") {
                    if let Ok(client) = redis::Client::open(redis_url) {
                        if let Ok(mut conn) = client.get_async_connection().await {
                            let _: Result<(), _> = conn.rpush("jobs", j.id.to_string()).await;
                        } else {
                            log::error!("Failed to connect to Redis to queue job {}.", j.id);
                        }
                    } else {
                        log::error!("Failed to open Redis client to queue job {}.", j.id);
                    }
                } else {
                    log::warn!("REDIS_URL not set, job {} not queued via Redis.", j.id);
                }
            }
            Err(e) => {
                log::error!("Failed to create analysis job for document {}: {:?}", created_document.id, e);
                cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Failed to queue analysis job."}));
            }
        }
    }

    HttpResponse::Ok().json(created_document)
}

/// Configure Actix routes for document-related endpoints.
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(upload);
}