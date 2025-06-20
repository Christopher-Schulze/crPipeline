use actix_web::{get, post, web, HttpResponse};
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::OrgSettings;
use sqlx::PgPool;

#[get("/settings/{org_id}")]
async fn get_settings(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }
    match OrgSettings::find(&pool, *path).await {
        Ok(settings) => HttpResponse::Ok().json(settings),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/settings")]
async fn update_settings(data: web::Json<OrgSettings>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if data.org_id != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }
    match OrgSettings::update(&pool, data.into_inner()).await {
        Ok(updated) => HttpResponse::Ok().json(updated),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_settings).service(update_settings);
}
