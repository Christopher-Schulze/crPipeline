use actix_web::{get, web, HttpResponse, http::StatusCode, ResponseError};
use crate::error::ApiError;
use sqlx::PgPool;
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::AuditLog;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct PaginationParams {
    page: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct PaginatedAuditLogs {
    items: Vec<AuditLog>,
    total_items: i64,
    page: i64,
    per_page: i64,
    total_pages: i64,
}

#[get("/audit/{org_id}")]
async fn list_logs(
    path: web::Path<Uuid>,
    query: web::Query<PaginationParams>,
    user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if *path != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1).min(100);
    let offset = (page - 1) * limit;

    let total_items: i64 = match sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE org_id=$1",
    )
    .bind(*path)
    .fetch_one(pool.as_ref())
    .await
    {
        Ok(count) => count,
        Err(e) => {
            log::error!("Failed to count audit logs for org {}: {:?}", *path, e);
            return ApiError::new(
                "Failed to retrieve logs",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .error_response();
        }
    };

    match sqlx::query_as::<_, AuditLog>(
        "SELECT * FROM audit_logs WHERE org_id=$1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(*path)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(logs) => {
            let total_pages = (total_items as f64 / limit as f64).ceil() as i64;
            HttpResponse::Ok().json(PaginatedAuditLogs {
                items: logs,
                total_items,
                page,
                per_page: limit,
                total_pages,
            })
        }
        Err(e) => {
            log::error!("Failed to fetch audit logs for org {}: {:?}", *path, e);
            ApiError::new("Failed to retrieve logs", StatusCode::INTERNAL_SERVER_ERROR)
                .error_response()
        }
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_logs);
}
