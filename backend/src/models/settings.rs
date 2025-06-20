use serde::{Serialize, Deserialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct OrgSettings {
    pub org_id: Uuid,
    pub monthly_upload_quota: i32,
    pub monthly_analysis_quota: i32,
    pub accent_color: String,
}

pub struct NewOrgSettings {
    pub org_id: Uuid,
}

impl OrgSettings {
    pub async fn create_default(pool: &PgPool, org_id: Uuid) -> sqlx::Result<OrgSettings> {
        sqlx::query_as::<_, OrgSettings>(
            "INSERT INTO org_settings (org_id) VALUES ($1) RETURNING *",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await
    }

    pub async fn find(pool: &PgPool, org_id: Uuid) -> sqlx::Result<OrgSettings> {
        sqlx::query_as::<_, OrgSettings>("SELECT * FROM org_settings WHERE org_id=$1")
            .bind(org_id)
            .fetch_one(pool)
            .await
    }

    pub async fn update(pool: &PgPool, settings: OrgSettings) -> sqlx::Result<OrgSettings> {
        sqlx::query_as::<_, OrgSettings>(
            "UPDATE org_settings SET monthly_upload_quota=$1, monthly_analysis_quota=$2, accent_color=$3 WHERE org_id=$4 RETURNING *",
        )
        .bind(settings.monthly_upload_quota)
        .bind(settings.monthly_analysis_quota)
        .bind(settings.accent_color)
        .bind(settings.org_id)
        .fetch_one(pool)
        .await
    }
}
