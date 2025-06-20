use crate::models::{AuditLog, NewAuditLog};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn log_action(pool: &PgPool, org_id: Uuid, user_id: Uuid, action: &str) {
    let _ = AuditLog::create(pool, NewAuditLog { org_id, user_id, action: action.to_string() }).await;
}
