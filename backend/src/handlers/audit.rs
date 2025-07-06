use actix_web::{get, web, HttpResponse, http::StatusCode, ResponseError};
use crate::error::ApiError;
use sqlx::PgPool;
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::AuditLog;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuditQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct PaginatedAuditLogs {
    pub items: Vec<AuditLog>,
    pub total_items: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[get("/audit/{org_id}")]
async fn list_logs(
    path: web::Path<Uuid>,
    query: web::Query<AuditQuery>,
    user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if *path != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1).min(100);

    match AuditLog::list_by_org_paginated(pool.as_ref(), *path, page, limit).await {
        Ok((items, total)) => {
            let total_pages = (total as f64 / limit as f64).ceil() as i64;
            HttpResponse::Ok().json(PaginatedAuditLogs {
                items,
                total_items: total,
                page,
                per_page: limit,
                total_pages,
            })
        }
        Err(_) => ApiError::new("Failed to retrieve logs", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_logs);
}
