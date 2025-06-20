use actix_web::{web, get, post, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use crate::models::{Organization, NewOrganization, OrgSettings};
use crate::middleware::auth::AuthUser;

#[derive(Deserialize)]
pub struct OrgInput { pub name: String }

#[post("/orgs")]
async fn create_org(data: web::Json<OrgInput>, _user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    let org = NewOrganization { name: data.name.clone() };
    match Organization::create(&pool, org).await {
        Ok(o) => {
            let _ = OrgSettings::create_default(&pool, o.id).await;
            HttpResponse::Ok().json(o)
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/orgs")]
async fn list_orgs(_user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    match Organization::all(&pool).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_org).service(list_orgs);
}
