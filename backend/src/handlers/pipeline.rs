use actix_web::{web, get, post, put, delete, HttpResponse, http::StatusCode, ResponseError};
use crate::error::ApiError;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashSet;
use crate::models::{Pipeline, NewPipeline};
use crate::middleware::auth::AuthUser;

#[derive(Deserialize)]
pub struct PipelineInput {
    pub org_id: Uuid,
    pub name: String,
    pub stages: serde_json::Value,
}

fn validate_stages(stages: &serde_json::Value) -> Result<(), HttpResponse> {
    if let Some(stages_array) = stages.as_array() {
        if stages_array.is_empty() {
            return Err(HttpResponse::BadRequest().json(serde_json::json!({"error": "Pipeline must have at least one stage."})));
        }
        let mut seen_ids = HashSet::new();
        for (index, stage_val) in stages_array.iter().enumerate() {
            if let Some(stage_obj) = stage_val.as_object() {
                if let Some(id_val) = stage_obj.get("id").and_then(|v| v.as_str()) {
                    if !seen_ids.insert(id_val.to_string()) {
                        return Err(HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("Duplicate stage id '{}'", id_val)
                        })));
                    }
                }
                let stage_type_str: String;
                if let Some(type_val) = stage_obj.get("type") {
                    if let Some(s) = type_val.as_str() {
                        if s.trim().is_empty() {
                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} 'type' cannot be empty.", index)
                            })));
                        }
                        stage_type_str = s.trim().to_lowercase();
                    } else {
                        return Err(HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("Stage {} 'type' must be a string.", index)
                        })));
                    }
                } else {
                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                        "error": format!("Stage {} must have a 'type' field.", index)
                    })));
                }

                if let Some(command_val) = stage_obj.get("command") {
                    if command_val.is_null() {
                    } else if let Some(command_str) = command_val.as_str() {
                        if command_str.trim().is_empty() {
                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} 'command', if present and not null, cannot be empty.", index)
                            })));
                        }
                    } else {
                        return Err(HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("Stage {} 'command' must be a string or null.", index)
                        })));
                    }
                }

                match stage_type_str.as_str() {
                    "ai" => {
                        if let Some(prompt_name_val) = stage_obj.get("prompt_name") {
                            if !prompt_name_val.is_string() && !prompt_name_val.is_null() {
                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} (AI): 'prompt_name' must be a string or null.", index)
                                })));
                            }
                            if let Some(s) = prompt_name_val.as_str() {
                                if s.trim().is_empty() {
                                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                        "error": format!("Stage {} (AI): 'prompt_name', if a string, cannot be empty.", index)
                                    })));
                                }
                            }
                        }
                    }
                    "ocr" => {
                        if let Some(engine_val) = stage_obj.get("ocr_engine") {
                            if !engine_val.is_null() {
                                if let Some(engine_str) = engine_val.as_str() {
                                    if engine_str != "default" && engine_str != "external" {
                                        return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                            "error": format!("Stage {} (OCR): 'ocr_engine' must be 'default', 'external', or null.", index)
                                        })));
                                    }
                                    if engine_str == "external" {
                                        if let Some(endpoint_val) = stage_obj.get("ocr_stage_endpoint") {
                                            if let Some(endpoint_str) = endpoint_val.as_str() {
                                                if endpoint_str.trim().is_empty() {
                                                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                                        "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' must be a non-empty string when ocr_engine is 'external'.", index)
                                                    })));
                                                }
                                            } else {
                                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                                    "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' must be a non-empty string when ocr_engine is 'external'.", index)
                                                })));
                                            }
                                        } else {
                                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                                "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' is required when ocr_engine is 'external'.", index)
                                            })));
                                        }
                                        if let Some(key_val) = stage_obj.get("ocr_stage_key") {
                                            if !key_val.is_string() && !key_val.is_null() {
                                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                                    "error": format!("Stage {} (OCR): 'ocr_stage_key' for external engine must be a string or null.", index)
                                                })));
                                            }
                                        }
                                    }
                                } else {
                                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                        "error": format!("Stage {} (OCR): 'ocr_engine' must be a string or null.", index)
                                    })));
                                }
                            }
                        }
                        if stage_obj.get("ocr_engine").as_ref().map_or(true, |v| v.as_str() != Some("external")) {
                            if stage_obj.contains_key("ocr_stage_endpoint") || stage_obj.contains_key("ocr_stage_key") {
                                if stage_obj.get("ocr_engine").is_none() || stage_obj.get("ocr_engine").and_then(|v| v.as_str()) == Some("default") {
                                }
                            }
                        }
                        if let Some(key_val) = stage_obj.get("ocr_stage_key") {
                            if !key_val.is_string() && !key_val.is_null() {
                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} (OCR): 'ocr_stage_key' must be a string or null.", index)
                                })));
                            }
                        }
                    }
                    "parse" | "report" => {}
                    _ => {}
                }
            } else {
                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": format!("Stage {} must be an object.", index)
                })));
            }
        }
    } else {
        return Err(HttpResponse::BadRequest().json(serde_json::json!({"error": "'stages' must be an array."})));
    }
    Ok(())
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
        Ok(p) => HttpResponse::Ok().json(p),
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
    match sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE org_id=$1")
        .bind(*path)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(list) => HttpResponse::Ok().json(list),
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
        Ok(p) => HttpResponse::Ok().json(p),
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
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => ApiError::new("Failed to delete pipeline", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(create_pipeline)
        .service(list_pipelines)
        .service(update_pipeline)
        .service(delete_pipeline);
}

