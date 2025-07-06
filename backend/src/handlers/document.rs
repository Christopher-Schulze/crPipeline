use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::{
    AnalysisJob, Document, DocumentError, NewAnalysisJob, NewDocument, OrgSettings,
};
use crate::utils::log_action;
use actix_multipart::Multipart;
use actix_web::{delete, get, post, web, HttpResponse, ResponseError};
use anyhow::Error;
use async_trait::async_trait;
use aws_sdk_s3::{presigning::PresigningConfig, Client};
use futures_util::StreamExt as _;
use lopdf::Document as PdfDoc;
use redis::AsyncCommands;
use sanitize_filename; // Added for sanitizing filenames
use sqlx::PgPool;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

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
        log::error!(
            "Failed to delete {} from S3 bucket {} during cleanup: {:?}",
            key,
            bucket,
            e
        );
    }
}

#[derive(serde::Deserialize)]
pub struct UploadParams {
    pub org_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub is_target: Option<bool>,
}

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
            log::error!(
                "Could not verify organization settings for quota (org_id {}): {:?}",
                org_id,
                e
            );
            Err(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Could not verify organization settings for quota."}),
            ))
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
            log::error!(
                "Could not verify organization settings for analysis quota (org_id {}): {:?}",
                org_id,
                e
            );
            Err(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Could not verify organization settings for analysis quota."})))
        }
    }
}

#[tracing::instrument(skip(s3, bytes))]
async fn upload_to_s3(
    s3: &Client,
    bucket: &str,
    key: &str,
    bytes: Vec<u8>,
) -> Result<(), ApiError> {
    match s3
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(bytes.into())
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!(error=?e, bucket, key, "upload failed");
            Err(ApiError::from_s3("File upload to storage failed.", e))
        }
    }
}

#[post("/upload")]
#[tracing::instrument(skip(payload, params, pool, s3, user))]
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
    let (base_filename, pages) =
        match validate_document(&user_provided_filename, &file_content_type, &bytes_data).await {
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
        return HttpResponse::Unauthorized().json(
            serde_json::json!({"error": "You are not authorized to upload to this organization."}),
        );
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

    if let Err(err) = upload_to_s3(s3.get_ref(), &bucket, &s3_key_name, bytes_data.clone()).await {
        return err.error_response();
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
            log::warn!(
                "Rejected unsafe filename during document creation: {}",
                s3_key_name
            );
            cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "Invalid filename."}));
        }
        Err(DocumentError::Sqlx(e)) => {
            cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
            return ApiError::from_db("Failed to save document information.", e).error_response();
        }
    };

    log_action(
        &pool,
        user.org_id,
        user.user_id,
        &format!("upload:{}", created_document.id),
    )
    .await;

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
                log_action(
                    &pool,
                    user.org_id,
                    user.user_id,
                    &format!("job_created:{}", j.id),
                )
                .await;
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
                cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
                return ApiError::from_db("Failed to queue analysis job.", e).error_response();
            }
        }
    }

    HttpResponse::Ok().json(created_document)
}

/// Download a document either by streaming locally when `LOCAL_S3_DIR` is set
/// or by returning a presigned S3 URL.
#[get("/download/{document_id}")]
#[tracing::instrument(skip(pool, s3, user))]
pub async fn download(
    path: web::Path<Uuid>,
    user: AuthUser,
    pool: web::Data<PgPool>,
    s3: web::Data<Client>,
) -> HttpResponse {
    let document_id = path.into_inner();

    let doc = match sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id=$1")
        .bind(document_id)
        .fetch_optional(pool.as_ref())
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return HttpResponse::NotFound().finish(),
        Err(e) => {
            return ApiError::from_db("Failed to fetch document.", e).error_response();
        }
    };

    if doc.org_id != user.org_id && user.role != "admin" {
        log::warn!(
            "Unauthorized download attempt {} by user {} (org {})",
            document_id,
            user.user_id,
            user.org_id
        );
        return HttpResponse::Unauthorized().finish();
    }

    if let Ok(local_dir) = std::env::var("LOCAL_S3_DIR") {
        let path = Path::new(&local_dir).join(&doc.filename);
        match tokio::fs::read(path).await {
            Ok(bytes) => HttpResponse::Ok()
                .append_header(("Content-Type", "application/pdf"))
                .append_header((
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", doc.display_name),
                ))
                .body(bytes),
            Err(e) => ApiError::from_s3("Failed to read local file", e).error_response(),
        }
    } else {
        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
        let presign_cfg = match PresigningConfig::expires_in(Duration::from_secs(3600)) {
            Ok(cfg) => cfg,
            Err(e) => {
                return ApiError::from_s3("Failed to create presign config", e).error_response();
            }
        };
        match s3
            .get_object()
            .bucket(&bucket)
            .key(&doc.filename)
            .presigned(presign_cfg)
            .await
        {
            Ok(req) => HttpResponse::Ok().json(serde_json::json!({"url": req.uri().to_string()})),
            Err(e) => ApiError::from_s3("Failed to presign document", e).error_response(),
        }
    }
}

#[delete("/documents/{id}")]
#[tracing::instrument(skip(pool, s3, user))]
pub async fn delete_document(
    path: web::Path<Uuid>,
    user: AuthUser,
    pool: web::Data<PgPool>,
    s3: web::Data<Client>,
) -> HttpResponse {
    let doc_id = path.into_inner();
    let doc = match sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id=$1")
        .bind(doc_id)
        .fetch_optional(pool.as_ref())
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return HttpResponse::NotFound().finish(),
        Err(e) => return ApiError::from_db("Failed to fetch document.", e).error_response(),
    };

    if doc.org_id != user.org_id && user.role != "admin" {
        log::warn!(
            "Unauthorized delete attempt {} by user {} (org {})",
            doc_id,
            user.user_id,
            user.org_id
        );
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"}));
    }

    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
    cleanup_s3_object(s3.get_ref(), &bucket, &doc.filename).await;

    match Document::delete(&pool, doc_id).await {
        Ok(_) => {
            log_action(
                &pool,
                user.org_id,
                user.user_id,
                &format!("delete_document:{}", doc_id),
            )
            .await;
            HttpResponse::Ok().finish()
        }
        Err(e) => ApiError::from_db("Failed to delete document", e).error_response(),
    }
}

/// Configure Actix routes for document-related endpoints.
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(upload)
        .service(download)
        .service(delete_document);
}

async fn validate_document(
    user_filename: &str,
    file_content_type: &Option<String>,
    bytes_data: &[u8],
) -> Result<(String, i32), HttpResponse> {
    let (base_filename, file_type) =
        crate::utils::validate_filename_and_type(user_filename, file_content_type, bytes_data)?;

    let pages = if file_type == "pdf" {
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
