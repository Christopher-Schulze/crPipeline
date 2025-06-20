use actix_web::{get, web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::AuditLog;

#[get("/audit/{org_id}")]
async fn list_logs(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }
    match AuditLog::list_by_org(pool.as_ref(), *path).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_logs);
}
