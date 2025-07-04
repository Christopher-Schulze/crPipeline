use actix_multipart::Multipart;
use actix_web::{post, get, web, HttpResponse};
use futures_util::StreamExt as _;
use aws_sdk_s3::Client;
use aws_sdk_s3::presigning::PresigningConfig;
use redis::AsyncCommands;
use uuid::Uuid;
use lopdf::Document as PdfDoc;
use crate::models::{Document, NewDocument, AnalysisJob, NewAnalysisJob, OrgSettings, DocumentError};
use crate::utils::log_action;
use crate::middleware::auth::AuthUser;
use sqlx::PgPool;
use std::time::Duration;
use sanitize_filename; // Added for sanitizing filenames
use anyhow::Error;
use async_trait::async_trait;

/// Abstraction over S3 deletion used for easier testing.
#[async_trait]
pub trait S3Deleter {
    /// Delete a single object from `bucket` under `key`.
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

/// Helper that attempts to delete the given S3 object, logging on failure.
pub async fn cleanup_s3_object<S: S3Deleter + Sync>(s3: &S, bucket: &str, key: &str) {
    if let Err(e) = s3.delete_object(bucket, key).await {
        log::error!("Failed to delete {} from S3 bucket {} during cleanup: {:?}", key, bucket, e);
    }
}

/// Query parameters accepted by the upload endpoint.
#[derive(serde::Deserialize)]
pub struct UploadParams {
    /// Organization receiving the document.
    pub org_id: Uuid,
    // owner_id is taken from [`AuthUser`] instead.
    /// Optional pipeline the document should immediately trigger.
    pub pipeline_id: Option<Uuid>,
    /// Mark document as a target document counting against quota.
    pub is_target: Option<bool>,
}

// Define PDF magic bytes
const PDF_MAGIC_BYTES: &[u8] = b"%PDF-";

/// Upload a document and optionally queue it for analysis.
///
/// Accepts a multipart file upload together with [`UploadParams`].
#[post("/upload")]
pub async fn upload(
    mut payload: Multipart,
    params: web::Query<UploadParams>,
    user: AuthUser,
    pool: web::Data<sqlx::PgPool>,
    s3: web::Data<Client>, // aws_sdk_s3::Client
) -> HttpResponse {
    let mut user_provided_filename = String::new(); // To store the original filename
    let mut file_content_type: Option<String> = None;
    let mut bytes_data = Vec::new();

    // 1. Iterate over multipart fields to get filename, content_type, and data
    while let Some(Ok(mut field)) = payload.next().await {
        if let Some(name) = field.content_disposition().get_filename() {
            // Store the original filename (potentially with path components, handle that)
            // For display_name, we want what the user provided.
            // For s3_key, we'll sanitize the base name part.
            user_provided_filename = name.to_string();
        }
        if let Some(content_type) = field.content_type() {
            file_content_type = Some(content_type.to_string());
            log::debug!("File content type from header: {:?}", file_content_type);
        }

        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => bytes_data.extend_from_slice(&data),
                Err(e) => {
                    log::error!("Error reading chunk from multipart field: {:?}", e);
                    return HttpResponse::BadRequest().json(serde_json::json!({"error": "Error reading uploaded file."}));
                }
            }
        }
    }

    // Extract the base filename from user_provided_filename for sanitization and validation
    let base_filename_for_validation = if let Some(f_name) = std::path::Path::new(&user_provided_filename).file_name().and_then(|s| s.to_str()) {
        f_name.to_string()
    } else {
        user_provided_filename.clone() // Fallback if it's already just a name or unusual
    };

    if base_filename_for_validation.is_empty() { // Check after potential path stripping
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Filename not provided or invalid."}));
    }
    if bytes_data.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "File content is empty."}));
    }

    // Max file size check (e.g., 200MB)
    if bytes_data.len() > 200 * 1024 * 1024 {
         return HttpResponse::PayloadTooLarge().json(serde_json::json!({"error": "File size exceeds the 200MB limit."}));
    }

    // 2. Validate based on filename extension and determined Content-Type / Magic Bytes
    // Use base_filename_for_validation for extension checks
    let lower_filename_for_validation = base_filename_for_validation.to_lowercase();
    let detected_file_type = if lower_filename_for_validation.ends_with(".pdf") {
        if let Some(ref ct) = file_content_type {
            if ct != "application/pdf" {
                if !ct.starts_with("application/octet-stream") {
                    log::warn!("PDF upload for '{}': Mismatch Content-Type: {:?}, expected application/pdf or application/octet-stream", user_provided_filename, file_content_type);
                    return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid Content-Type for PDF file. Expected 'application/pdf'."}));
                } else {
                    log::info!("PDF upload for '{}': Content-Type was application/octet-stream. Proceeding with magic byte check.", user_provided_filename);
                }
            }
        }

        if !bytes_data.starts_with(PDF_MAGIC_BYTES) {
            log::warn!("Invalid PDF magic bytes for file '{}'", user_provided_filename);
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid PDF file format (magic bytes mismatch)."}));
        }
        "pdf"
    } else if lower_filename_for_validation.ends_with(".md") {
        if file_content_type.as_deref().map_or(false, |ct| ct != "text/markdown" && ct != "text/plain" && !ct.starts_with("application/octet-stream")) {
            log::warn!("MD upload for '{}': Suspicious Content-Type: {:?}. Allowing.", user_provided_filename, file_content_type);
        }
        "md"
    } else if lower_filename_for_validation.ends_with(".txt") {
        if file_content_type.as_deref().map_or(false, |ct| ct != "text/plain" && !ct.starts_with("application/octet-stream")) {
            log::warn!("TXT upload for '{}': Suspicious Content-Type: {:?}. Allowing.", user_provided_filename, file_content_type);
        }
        "txt"
    } else {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Unsupported file type. Only .pdf, .md, .txt are allowed."}));
    };

    // 3. PDF Page Count (Only for PDFs)
    let pages = if detected_file_type == "pdf" {
        match PdfDoc::load_mem(&bytes_data).map(|d| d.get_pages().len() as i32) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to parse PDF pages for {}: {:?}", user_provided_filename, e);
                return HttpResponse::BadRequest().json(serde_json::json!({"error": "Corrupt or invalid PDF file. Could not count pages."}));
            }
        }
    } else {
        0 // Default to 0 pages for non-PDF files
    };

    // Authorization for org_id (user must belong to org or be admin)
    if params.org_id != user.org_id && user.role != "admin" {
        log::warn!("User {} (org_id {}) attempted to upload to org_id {} without admin rights.", user.user_id, user.org_id, params.org_id);
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "You are not authorized to upload to this organization."}));
    }

    // Quota checks (target documents and analysis jobs)
    if params.is_target.unwrap_or(false) {
        match OrgSettings::find(&pool, params.org_id).await {
            Ok(settings) => {
                let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE org_id=$1 AND is_target=true AND upload_date >= date_trunc('month', NOW())")
                    .bind(params.org_id).fetch_one(pool.as_ref()).await.unwrap_or((0,));
                if count >= settings.monthly_upload_quota as i64 {
                    return HttpResponse::TooManyRequests().json(serde_json::json!({"error": "Monthly upload quota for target documents exceeded."}));
                }
            }
            Err(e) => {
                log::error!("Could not verify organization settings for quota (org_id {}): {:?}", params.org_id, e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Could not verify organization settings for quota."}));
            }
        }
    }

    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());

    // Sanitize the base filename part for S3 key construction
    let sanitized_filename_part = sanitize_filename::sanitize(&base_filename_for_validation);
    let s3_key_name = format!("{}-{}", Uuid::new_v4(), sanitized_filename_part);

    if s3.put_object().bucket(&bucket).key(&s3_key_name).body(bytes_data.into()).send().await.is_err() {
        log::error!("Failed to upload {} to S3 bucket {}", s3_key_name, bucket);
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "File upload to storage failed."}));
    }

    let doc_to_create = NewDocument {
        org_id: params.org_id,
        owner_id: user.user_id,
        filename: s3_key_name.clone(),       // This is the S3 key
        display_name: user_provided_filename, // This is the original name from user
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
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to save document information."}));
        }
    };

    log_action(&pool, user.org_id, user.user_id, &format!("upload:{}", created_document.id)).await;

    if let Some(pipeline_id) = params.pipeline_id {
        match OrgSettings::find(&pool, params.org_id).await {
            Ok(settings) => {
                let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM analysis_jobs WHERE org_id=$1 AND created_at >= date_trunc('month', NOW())")
                    .bind(params.org_id).fetch_one(pool.as_ref()).await.unwrap_or((0,));
                if count >= settings.monthly_analysis_quota as i64 {
                    cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
                    return HttpResponse::TooManyRequests().json(serde_json::json!({"error": "Monthly analysis quota exceeded. Document uploaded but not queued for analysis."}));
                }
            }
            Err(e) => {
                log::error!("Could not verify organization settings for analysis quota (org_id {}): {:?}", params.org_id, e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Could not verify organization settings for analysis quota."}));
            }
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
                        } else { log::error!("Failed to connect to Redis to queue job {}.", j.id); }
                    } else { log::error!("Failed to open Redis client to queue job {}.", j.id); }
                } else { log::warn!("REDIS_URL not set, job {} not queued via Redis.", j.id); }
            }
            Err(e) => {
                log::error!("Failed to create analysis job for document {}: {:?}", created_document.id, e);
                cleanup_s3_object(s3.get_ref(), &bucket, &s3_key_name).await;
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to queue analysis job."}));
            }
        }
    }

    HttpResponse::Ok().json(created_document)
}

/// Configure Actix routes for document-related endpoints.
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(upload)
        .service(list_documents)
        .service(download);
}

// Structs for pagination
use serde::{Deserialize, Serialize}; // Ensure Deserialize is added for QueryParams

#[derive(Serialize)]
struct PaginatedDocumentsResponse {
    items: Vec<crate::models::Document>,
    total_items: i64,
    page: i64,
    per_page: i64,
    total_pages: i64,
    sort_by: String,      // New: Actual sort column used
    sort_order: String,   // New: Actual sort order used ("asc" or "desc")
}

#[derive(Deserialize, Debug)]
struct PaginationParams {
    page: Option<i64>,
    limit: Option<i64>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    // New filter fields
    display_name_ilike: Option<String>,
    is_target: Option<bool>,
}

/// List documents for an organization with optional pagination and filtering.
#[get("/documents/{org_id}")]
async fn list_documents(
    path: web::Path<Uuid>, // org_id
    query_params: web::Query<PaginationParams>,
    user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let org_id_from_path = path.into_inner();

    // Authorization
    if user.role != "admin" && org_id_from_path != user.org_id {
        log::warn!("Unauthorized attempt to list documents for org {} by user {}", org_id_from_path, user.user_id);
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "You are not authorized to list documents for this organization."}));
    }

    let page = query_params.page.unwrap_or(1).max(1);
    let limit = query_params.limit.unwrap_or(10).max(1).min(100);
    let offset = (page - 1) * limit;

    // Sorting parameters
    let mut sort_by_col = "upload_date".to_string();
    if let Some(sb) = &query_params.sort_by {
        match sb.trim().to_lowercase().as_str() {
            "filename" => sort_by_col = "filename".to_string(),
            "display_name" => sort_by_col = "display_name".to_string(),
            "upload_date" => sort_by_col = "upload_date".to_string(),
            "pages" => sort_by_col = "pages".to_string(),
            "is_target" => sort_by_col = "is_target".to_string(),
            _ => { log::warn!("Invalid sort_by parameter: '{}'. Defaulting to upload_date.", sb); }
        }
    }
    let sort_order_str = query_params.sort_order.as_ref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s == "asc" || s == "desc")
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| if sort_by_col == "upload_date" { "DESC".to_string() } else { "ASC".to_string() });

    // Build WHERE clauses and arguments for filtering
    let mut current_arg_idx = 1; // $1 will be org_id

    let display_name_filter_sql = if query_params.display_name_ilike.as_ref().filter(|s| !s.is_empty()).is_some() {
        current_arg_idx += 1;
        format!("AND display_name ILIKE ${}", current_arg_idx)
    } else {
        "".to_string()
    };

    let is_target_filter_sql = if query_params.is_target.is_some() {
        current_arg_idx += 1;
        format!("AND is_target = ${}", current_arg_idx)
    } else {
        "".to_string()
    };

    // --- Total Count Query ---
    let count_query_string = format!(
        "SELECT COUNT(*) FROM documents WHERE org_id = $1 {} {}", // org_id is always $1
        display_name_filter_sql,
        is_target_filter_sql
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_query_string);
    count_query = count_query.bind(org_id_from_path); // Bind $1
    if let Some(dni) = query_params.display_name_ilike.as_ref().filter(|s| !s.is_empty()) {
        count_query = count_query.bind(format!("%{}%", dni));
    }
    if let Some(it) = query_params.is_target {
        count_query = count_query.bind(it);
    }

    let total_items = match count_query.fetch_one(pool.as_ref()).await {
        Ok(count) => count,
        Err(e) => {
            log::error!("Failed to count documents for org {} (filters: name_ilike={:?}, is_target={:?} ): {:?}",
                org_id_from_path, query_params.display_name_ilike, query_params.is_target, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to count documents."}));
        }
    };

    if total_items == 0 {
        return HttpResponse::Ok().json(PaginatedDocumentsResponse {
            items: vec![], total_items: 0, page, per_page: limit, total_pages: 0,
            sort_by: sort_by_col, sort_order: sort_order_str,
        });
    }

    // --- Main Data Query ---
    let order_by_clause = format!("{} {}", sort_by_col, sort_order_str);
    // Recalculate current_arg_idx for main query's LIMIT and OFFSET placeholders
    let mut main_query_current_arg_idx = 1; // $1 for org_id
    if query_params.display_name_ilike.as_ref().filter(|s| !s.is_empty()).is_some() { main_query_current_arg_idx += 1; }
    if query_params.is_target.is_some() { main_query_current_arg_idx += 1; }

    let limit_placeholder = format!("${}", main_query_current_arg_idx + 1);
    let offset_placeholder = format!("${}", main_query_current_arg_idx + 2);

    let data_query_string = format!(
        "SELECT * FROM documents WHERE org_id = $1 {} {} ORDER BY {} LIMIT {} OFFSET {}",
        display_name_filter_sql,
        is_target_filter_sql,
        order_by_clause,
        limit_placeholder,
        offset_placeholder
    );

    let mut data_query = sqlx::query_as::<_, crate::models::Document>(&data_query_string);
    data_query = data_query.bind(org_id_from_path); // Bind $1
    if let Some(dni) = query_params.display_name_ilike.as_ref().filter(|s| !s.is_empty()) {
        data_query = data_query.bind(format!("%{}%", dni));
    }
    if let Some(it) = query_params.is_target {
        data_query = data_query.bind(it);
    }
    data_query = data_query.bind(limit);
    data_query = data_query.bind(offset);

    match data_query.fetch_all(pool.as_ref()).await {
        Ok(docs) => {
            let total_pages = (total_items as f64 / limit as f64).ceil() as i64;
            HttpResponse::Ok().json(PaginatedDocumentsResponse {
                items: docs, total_items, page, per_page: limit, total_pages,
                sort_by: sort_by_col, sort_order: sort_order_str,
            })
        }
        Err(e) => {
             log::error!("Failed to retrieve paginated documents for org {} (sorted by {} {}, filters: name_ilike={:?}, is_target={:?} ): {:?}",
                org_id_from_path, sort_by_col, sort_order_str, query_params.display_name_ilike, query_params.is_target, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to retrieve documents."}));
        }
    }
}

/// Generate a presigned download URL for a document owned by the user.
#[get("/download/{id}")]
async fn download(
    path: web::Path<Uuid>,
    user: AuthUser,
    pool: web::Data<PgPool>,
    s3: web::Data<Client>,
) -> HttpResponse {
    let document_id_from_path = *path;
    let doc = match sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id=$1")
        .bind(document_id_from_path)
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(d) => d,
        Err(sqlx::Error::RowNotFound) => {
            log::warn!("Document with id {} not found for download attempt.", document_id_from_path);
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Document not found."}));
        }
        Err(e) => {
            log::error!("Failed to fetch document {} for download: {:?}", document_id_from_path, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to retrieve document details."}));
        }
    };

    // Authorization: Allow global admin to download any document,
    // otherwise user must belong to the document's organization.
    if user.role != "admin" && doc.org_id != user.org_id {
        log::warn!(
            "Unauthorized attempt to download document {} (org {}) by user {} (org {})",
            doc.id, doc.org_id, user.user_id, user.org_id
        );
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "You are not authorized to download this document."}));
    }

    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());

    let presigning_config = match PresigningConfig::expires_in(Duration::from_secs(3600)) { // 1 hour expiry
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to create presigning config for document {}: {:?}", doc.id, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Could not generate download URL (config error)"}));
        }
    };

    match s3
        .get_object()
        .bucket(&bucket)
        .key(&doc.filename) // doc.filename stores the S3 key
        .presigned(presigning_config)
        .await
    {
        Ok(presigned_request) => {
            log_action(&pool, user.org_id, user.user_id, &format!("download_document_link_generated:{}", doc.id)).await;
            HttpResponse::Ok().json(serde_json::json!({ "url": presigned_request.uri().to_string() }))
        }
        Err(e) => {
            log::error!("Failed to generate presigned URL for document {}: {:?}", doc.id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Could not generate download URL (presign error)"}))
        }
    }
}
