use actix_web::{get, web, HttpResponse, http::StatusCode, ResponseError};
use crate::error::ApiError;
use sqlx::PgPool;
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::AuditLog;

#[get("/audit/{org_id}")]
async fn list_logs(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }
    match AuditLog::list_by_org(pool.as_ref(), *path).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(_) => ApiError::new("Failed to retrieve logs", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_logs);
}
