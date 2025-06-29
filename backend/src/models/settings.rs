use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Per-organization configuration and quota limits.
#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct OrgSettings {
    pub org_id: Uuid,
    pub monthly_upload_quota: i32,
    pub monthly_analysis_quota: i32,
    pub accent_color: String,
    // New fields
    pub ai_api_endpoint: Option<String>,
    pub ai_api_key: Option<String>,
    pub ocr_api_endpoint: Option<String>,
    pub ocr_api_key: Option<String>,
    pub prompt_templates: Option<serde_json::Value>,
    pub ai_custom_headers: Option<serde_json::Value>, // New field
}

/// Wrapper for creating default settings for an organization.
pub struct NewOrgSettings {
    pub org_id: Uuid,
}

impl OrgSettings {
    /// Insert default settings for a new organization.
    pub async fn create_default(pool: &PgPool, org_id: Uuid) -> sqlx::Result<OrgSettings> {
        sqlx::query_as::<_, OrgSettings>(
            "INSERT INTO org_settings (org_id) VALUES ($1) RETURNING *",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await
    }

    /// Retrieve settings for an organization.
    pub async fn find(pool: &PgPool, org_id: Uuid) -> sqlx::Result<OrgSettings> {
        sqlx::query_as::<_, OrgSettings>("SELECT * FROM org_settings WHERE org_id=$1")
            .bind(org_id)
            .fetch_one(pool)
            .await
    }

    /// Persist updated settings and return the saved row.
    pub async fn update(pool: &PgPool, settings: OrgSettings) -> sqlx::Result<OrgSettings> {
        sqlx::query_as::<_, OrgSettings>(
            "UPDATE org_settings SET \
             monthly_upload_quota=$1, \
             monthly_analysis_quota=$2, \
             accent_color=$3, \
             ai_api_endpoint=$4, \
             ai_api_key=$5, \
             ocr_api_endpoint=$6, \
             ocr_api_key=$7, \
             prompt_templates=$8, \
             ai_custom_headers=$9 \
             WHERE org_id=$10 RETURNING *",
        )
        .bind(settings.monthly_upload_quota)
        .bind(settings.monthly_analysis_quota)
        .bind(settings.accent_color)
        .bind(settings.ai_api_endpoint)
        .bind(settings.ai_api_key)
        .bind(settings.ocr_api_endpoint)
        .bind(settings.ocr_api_key)
        .bind(settings.prompt_templates)
        .bind(settings.ai_custom_headers) // New binding
        .bind(settings.org_id) // org_id is now $10
        .fetch_one(pool)
        .await
    }
}
