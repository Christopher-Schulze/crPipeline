use crate::models::{AuditLog, NewAuditLog};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn log_action(pool: &PgPool, org_id: Uuid, user_id: Uuid, action: &str) {
    let _ = AuditLog::create(pool, NewAuditLog { org_id, user_id, action: action.to_string() }).await;
}

const PDF_MAGIC_BYTES: &[u8] = b"%PDF-";
pub const MAX_FILE_SIZE: usize = 200 * 1024 * 1024; // 200MB

pub fn validate_filename_and_type(
    user_filename: &str,
    file_content_type: &Option<String>,
    bytes_data: &[u8],
) -> Result<(String, String), actix_web::HttpResponse> {
    use actix_web::HttpResponse;
    use std::path::Path;

    let base_filename = if let Some(f_name) = Path::new(user_filename)
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
        return Err(HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "File content is empty."})));
    }
    if bytes_data.len() > MAX_FILE_SIZE {
        return Err(HttpResponse::PayloadTooLarge().json(
            serde_json::json!({"error": "File size exceeds the 200MB limit."}),
        ));
    }

    let lower_filename = base_filename.to_lowercase();
    let detected_file_type = if lower_filename.ends_with(".pdf") {
        if let Some(ref ct) = file_content_type {
            if ct != "application/pdf" && !ct.starts_with("application/octet-stream") {
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
        if !bytes_data.starts_with(PDF_MAGIC_BYTES) {
            log::warn!("Invalid PDF magic bytes for file '{}'", user_filename);
            return Err(HttpResponse::BadRequest().json(
                serde_json::json!({"error": "Invalid PDF file format (magic bytes mismatch)."}),
            ));
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
        return Err(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Unsupported file type. Only .pdf, .md, .txt are allowed."
        })));
    };

    Ok((base_filename, detected_file_type.to_string()))
}

