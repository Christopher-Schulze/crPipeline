use actix_web::{web, get, post, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Pipeline, NewPipeline};
use crate::middleware::auth::AuthUser;

#[derive(Deserialize)]
pub struct PipelineInput {
    pub org_id: Uuid,
    pub name: String,
    pub stages: serde_json::Value,
}

#[post("/pipelines")]
async fn create_pipeline(data: web::Json<PipelineInput>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if data.org_id != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }
    let new = NewPipeline {
        org_id: data.org_id,
        name: data.name.clone(),
        stages: data.stages.clone(),
    };
    match Pipeline::create(&pool, new).await {
        Ok(p) => HttpResponse::Ok().json(p),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/pipelines/{org_id}")]
async fn list_pipelines(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }
    match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE org_id=$1")
        .bind(*path)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_pipeline).service(list_pipelines);
}

