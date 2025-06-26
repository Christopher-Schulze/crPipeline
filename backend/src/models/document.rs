use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use chrono::DateTime;
use chrono::Utc;

#[derive(Serialize, FromRow, Debug, Clone)]
pub struct Document {
    pub id: Uuid,
    pub org_id: Uuid,
    pub owner_id: Uuid,
    pub filename: String, // This is the S3 key (sanitized name)
    pub pages: i32,
    pub is_target: bool,
    pub upload_date: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub display_name: String, // New field
}

pub struct NewDocument {
    pub org_id: Uuid,
    pub owner_id: Uuid,
    pub filename: String, // S3 key
    pub pages: i32,
    pub is_target: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub display_name: String, // New field
}

impl Document {
    pub async fn create(pool: &PgPool, new: NewDocument) -> sqlx::Result<Document> {
        sqlx::query_as::<_, Document>(
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
        .await
    }
}
