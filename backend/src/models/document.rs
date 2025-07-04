use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use chrono::DateTime;
use chrono::Utc;

/// Errors that can occur when creating a document.
#[derive(Debug)]
pub enum DocumentError {
    /// The provided filename failed sanitization.
    SanitizationFailed,
    /// Underlying database error.
    Sqlx(sqlx::Error),
}

impl From<sqlx::Error> for DocumentError {
    fn from(e: sqlx::Error) -> Self {
        DocumentError::Sqlx(e)
    }
}

/// Stored PDF document belonging to an organization.
/// `filename` is the sanitized S3 key and `display_name` keeps the original name.
#[derive(Serialize, FromRow, Debug, Clone)]
pub struct Document {
    pub id: Uuid,
    pub org_id: Uuid,
    pub owner_id: Uuid,
    /// Sanitized S3 key for the file
    pub filename: String,
    pub pages: i32,
    pub is_target: bool,
    pub upload_date: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    /// Original filename provided by the user
    pub display_name: String,
}

/// Data required to insert a new document record.
pub struct NewDocument {
    pub org_id: Uuid,
    pub owner_id: Uuid,
    /// S3 key for the uploaded file
    pub filename: String,
    pub pages: i32,
    pub is_target: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub display_name: String,
}

impl Document {
    /// Insert a new document and return the created row.
    pub async fn create(pool: &PgPool, new: NewDocument) -> Result<Document, DocumentError> {
        let sanitized = sanitize_filename::sanitize(&new.filename);
        if sanitized != new.filename {
            return Err(DocumentError::SanitizationFailed);
        }
        let doc = sqlx::query_as::<_, Document>(
            "INSERT INTO documents (id, org_id, owner_id, filename, pages, is_target, expires_at, display_name) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *"
        )
        .bind(Uuid::new_v4())
        .bind(new.org_id)
        .bind(new.owner_id)
        .bind(new.filename) // Sanitized S3 key
        .bind(new.pages)
        .bind(new.is_target)
        .bind(new.expires_at)
        .bind(new.display_name) // Original user-provided filename
        .fetch_one(pool)
        .await?;
        Ok(doc)
    }
}
