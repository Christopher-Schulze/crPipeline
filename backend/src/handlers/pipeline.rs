use actix_web::{delete, get, post, put, web, HttpResponse, ResponseError, http::StatusCode};
use crate::error::ApiError;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use crate::models::{NewPipeline, Pipeline};
use crate::middleware::auth::AuthUser;
use crate::pipeline_validation::validate_stages;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use redis::AsyncCommands;

static PIPELINE_CACHE: Lazy<DashMap<Uuid, Vec<Pipeline>>> = Lazy::new(DashMap::new);

async fn cache_get(org_id: Uuid) -> Option<Vec<Pipeline>> {
    let key = format!("pipelines:{}", org_id);
    if let Ok(redis_url) = std::env::var("REDIS_URL") {
        if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_async_connection().await {
                if let Ok(data) = conn.get::<_, String>(&key).await {
                    if let Ok(pipes) = serde_json::from_str::<Vec<Pipeline>>(&data) {
                        return Some(pipes);
                    }
                }
            }
        }
    }
    PIPELINE_CACHE.get(&org_id).map(|v| v.clone())
}

async fn cache_set(org_id: Uuid, pipelines: &[Pipeline]) {
    PIPELINE_CACHE.insert(org_id, pipelines.to_vec());
    if let Ok(redis_url) = std::env::var("REDIS_URL") {
        if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_async_connection().await {
                if let Ok(data) = serde_json::to_string(pipelines) {
                    let _ : redis::RedisResult<()> = conn.set::<_, _, ()>(&format!("pipelines:{}", org_id), data).await;
                }
            }
        }
    }
}

async fn cache_invalidate(org_id: Uuid) {
    PIPELINE_CACHE.remove(&org_id);
    if let Ok(redis_url) = std::env::var("REDIS_URL") {
        if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_async_connection().await {
                let _ : redis::RedisResult<()> = conn.del::<_, ()>(&format!("pipelines:{}", org_id)).await;
            }
        }
    }
}

#[derive(Deserialize)]
pub struct PipelineInput {
    pub org_id: Uuid,
    pub name: String,
    pub stages: serde_json::Value,
}


#[post("/pipelines")]
async fn create_pipeline(data: web::Json<PipelineInput>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    // Authorization: Global admin can create for any org_id specified in data.
    // Other users can only create for their own org_id, which must match data.org_id.
    if user.role != "admin" {
        if data.org_id != user.org_id {
            return HttpResponse::Unauthorized().json(serde_json::json!({"error": "You can only create pipelines for your own organization."}));
        }
    }
    // If user is admin, they can create for the data.org_id provided in the payload.

    // Validate pipeline name
    if data.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Pipeline name cannot be empty."}));
    }

    if let Err(resp) = validate_stages(&data.stages) {
        return resp;
    }

    // If validation passes, proceed to create the pipeline
    let new_pipeline_data = NewPipeline { // Renamed variable to avoid conflict with 'new' keyword if it were one
        org_id: data.org_id,
        name: data.name.clone(),
        stages: data.stages.clone(), // Clone the validated Value
    };

    match Pipeline::create(&pool, new_pipeline_data).await {
        Ok(p) => {
            cache_invalidate(data.org_id).await;
            HttpResponse::Ok().json(p)
        }
        Err(e) => {
            log::error!("Failed to create pipeline for org_id {}: {:?}", data.org_id, e);
            ApiError::new("Failed to create pipeline", StatusCode::INTERNAL_SERVER_ERROR)
                .error_response()
        }
    }
}

#[get("/pipelines/{org_id}")]
async fn list_pipelines(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }
    if let Some(cached) = cache_get(*path).await {
        return HttpResponse::Ok().json(cached);
    }
    match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE org_id=$1")
        .bind(*path)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(list) => {
            cache_set(*path, &list).await;
            HttpResponse::Ok().json(list)
        }
        Err(_) => ApiError::new("Failed to list pipelines", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    }
}

#[put("/pipelines/{id}")]
async fn update_pipeline(
    path: web::Path<Uuid>,
    data: web::Json<PipelineInput>,
    user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let pipeline_id = path.into_inner();

    let existing = match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE id=$1")
        .bind(pipeline_id)
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(p) => p,
        Err(sqlx::Error::RowNotFound) => {
            return ApiError::new("Pipeline not found", StatusCode::NOT_FOUND)
                .error_response();
        }
        Err(_) => {
            return ApiError::new("Failed to fetch pipeline", StatusCode::INTERNAL_SERVER_ERROR)
                .error_response();
        }
    };

    if user.role != "admin" && existing.org_id != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }

    if data.org_id != existing.org_id {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Organization ID cannot be changed"}));
    }

    if data.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Pipeline name cannot be empty."}));
    }
    if let Err(resp) = validate_stages(&data.stages) {
        return resp;
    }

    match Pipeline::update(&pool, pipeline_id, &data.name, data.stages.clone()).await {
        Ok(p) => {
            cache_invalidate(existing.org_id).await;
            HttpResponse::Ok().json(p)
        }
        Err(_) => ApiError::new("Failed to update pipeline", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    }
}

#[delete("/pipelines/{id}")]
async fn delete_pipeline(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    let pipeline_id = path.into_inner();

    let existing = match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE id=$1")
        .bind(pipeline_id)
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(p) => p,
        Err(sqlx::Error::RowNotFound) => {
            return ApiError::new("Pipeline not found", StatusCode::NOT_FOUND)
                .error_response();
        }
        Err(_) => {
            return ApiError::new("Failed to fetch pipeline", StatusCode::INTERNAL_SERVER_ERROR)
                .error_response();
        }
    };

    if user.role != "admin" && existing.org_id != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }

    match Pipeline::delete(&pool, pipeline_id).await {
        Ok(_) => {
            cache_invalidate(existing.org_id).await;
            HttpResponse::Ok().finish()
        }
        Err(_) => ApiError::new("Failed to delete pipeline", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    }
}

#[post("/pipelines/{id}/clone")]
async fn clone_pipeline(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    let pipeline_id = path.into_inner();
    let existing = match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE id=$1")
        .bind(pipeline_id)
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(p) => p,
        Err(sqlx::Error::RowNotFound) => {
            return ApiError::new("Pipeline not found", StatusCode::NOT_FOUND)
                .error_response();
        }
        Err(_) => {
            return ApiError::new("Failed to fetch pipeline", StatusCode::INTERNAL_SERVER_ERROR)
                .error_response();
        }
    };

    if user.role != "admin" && existing.org_id != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }

    let new_data = NewPipeline {
        org_id: existing.org_id,
        name: format!("{} (copy)", existing.name),
        stages: existing.stages.clone(),
    };

    match Pipeline::create(&pool, new_data).await {
        Ok(p) => {
            cache_invalidate(existing.org_id).await;
            HttpResponse::Ok().json(p)
        }
        Err(_) => ApiError::new("Failed to clone pipeline", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(create_pipeline)
        .service(list_pipelines)
        .service(update_pipeline)
        .service(delete_pipeline)
        .service(clone_pipeline);
}

