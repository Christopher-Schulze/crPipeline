use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Serialize, FromRow, Debug)]
pub struct AuditLog {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub action: String,
}

pub struct NewAuditLog {
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub action: String,
}

impl AuditLog {
    pub async fn create(pool: &PgPool, new: NewAuditLog) -> sqlx::Result<AuditLog> {
        sqlx::query_as::<_, AuditLog>(
            "INSERT INTO audit_logs (id, org_id, user_id, action) VALUES ($1,$2,$3,$4) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(new.org_id)
        .bind(new.user_id)
        .bind(new.action)
        .fetch_one(pool)
        .await
    }

    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<Vec<AuditLog>> {
        sqlx::query_as::<_, AuditLog>(
            "SELECT * FROM audit_logs WHERE org_id=$1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }
}
