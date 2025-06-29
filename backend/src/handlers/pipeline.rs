use actix_web::{web, get, post, put, delete, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use crate::models::{Pipeline, NewPipeline};
use crate::middleware::auth::AuthUser;
use serde_json; // For json! macro and potentially Value if not inferred
use log; // For logging

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

    // Validate stages
    if let Some(stages_array) = data.stages.as_array() {
        if stages_array.is_empty() {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Pipeline must have at least one stage."}));
        }
        for (index, stage_val) in stages_array.iter().enumerate() {
            if let Some(stage_obj) = stage_val.as_object() {
                // Check for 'type' field
                let stage_type_str: String;
                match stage_obj.get("type") {
                    Some(type_val) => {
                        if let Some(s) = type_val.as_str() {
                            if s.trim().is_empty() {
                                return HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} 'type' cannot be empty.", index)
                                }));
                            }
                            stage_type_str = s.trim().to_lowercase();
                        } else {
                            return HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} 'type' must be a string.", index)
                            }));
                        }
                    }
                    None => {
                        return HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("Stage {} must have a 'type' field.", index)
                        }));
                    }
                }

                // Check for 'command' field (if present)
                if let Some(command_val) = stage_obj.get("command") {
                    if command_val.is_null() {
                        // Explicit null is allowed for command (means use default behavior for stage type)
                    } else if let Some(command_str) = command_val.as_str() {
                        if command_str.trim().is_empty() {
                            return HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} 'command', if present and not null, cannot be empty.", index)
                            }));
                        }
                    } else {
                        // 'command' is present, not null, but not a string
                        return HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("Stage {} 'command' must be a string or null.", index)
                        }));
                    }
                }

                // --- New Type-Specific Validations ---
                match stage_type_str.as_str() {
                    "ai" => {
                        if let Some(prompt_name_val) = stage_obj.get("prompt_name") {
                            if !prompt_name_val.is_string() && !prompt_name_val.is_null() {
                                return HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} (AI): 'prompt_name' must be a string or null.", index)
                                }));
                            }
                            if let Some(s) = prompt_name_val.as_str() {
                                if s.trim().is_empty() {
                                    return HttpResponse::BadRequest().json(serde_json::json!({
                                         "error": format!("Stage {} (AI): 'prompt_name', if a string, cannot be empty.", index)
                                    }));
                                }
                            }
                        }
                    }
                    "ocr" => {
                        if let Some(engine_val) = stage_obj.get("ocr_engine") {
                            if !engine_val.is_null() {
                                if let Some(engine_str) = engine_val.as_str() {
                                    if engine_str != "default" && engine_str != "external" {
                                        return HttpResponse::BadRequest().json(serde_json::json!({
                                            "error": format!("Stage {} (OCR): 'ocr_engine' must be 'default', 'external', or null.", index)
                                        }));
                                    }
                                    if engine_str == "external" {
                                        match stage_obj.get("ocr_stage_endpoint") {
                                            Some(endpoint_val) => {
                                                if let Some(endpoint_str) = endpoint_val.as_str() {
                                                    if endpoint_str.trim().is_empty() {
                                                        return HttpResponse::BadRequest().json(serde_json::json!({
                                                            "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' must be a non-empty string when ocr_engine is 'external'.", index)
                                                        }));
                                                    }
                                                } else { // Not a string
                                                    return HttpResponse::BadRequest().json(serde_json::json!({
                                                        "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' must be a non-empty string when ocr_engine is 'external'.", index)
                                                    }));
                                                }
                                            }
                                            None => { // Not present
                                                 return HttpResponse::BadRequest().json(serde_json::json!({
                                                    "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' is required when ocr_engine is 'external'.", index)
                                                }));
                                            }
                                        }
                                        // Validate ocr_stage_key if present
                                        if let Some(key_val) = stage_obj.get("ocr_stage_key") {
                                            if !key_val.is_string() && !key_val.is_null() {
                                                return HttpResponse::BadRequest().json(serde_json::json!({
                                                    "error": format!("Stage {} (OCR): 'ocr_stage_key' for external engine must be a string or null.", index)
                                                }));
                                            }
                                        }
                                    }
                                } else { // ocr_engine is not a string (and not null)
                                    return HttpResponse::BadRequest().json(serde_json::json!({
                                        "error": format!("Stage {} (OCR): 'ocr_engine' must be a string or null.", index)
                                    }));
                                }
                            }
                        }
                        // General check for ocr_stage_key if ocr_engine is not 'external' but key is provided
                        // or if ocr_engine is not provided at all but key is.
                        if stage_obj.get("ocr_engine").as_ref().map_or(true, |v| v.as_str() != Some("external")) {
                             if stage_obj.contains_key("ocr_stage_endpoint") || stage_obj.contains_key("ocr_stage_key") {
                                if stage_obj.get("ocr_engine").is_none() || stage_obj.get("ocr_engine").unwrap().as_str() == Some("default") {
                                     // Warn or error if endpoint/key are provided with default/missing engine?
                                     // For now, let's be permissive: if engine isn't 'external', these fields are ignored by worker.
                                     // However, if they are present and malformed, it's good to catch.
                                }
                             }
                        }
                         if let Some(key_val) = stage_obj.get("ocr_stage_key") {
                            if !key_val.is_string() && !key_val.is_null() {
                                 return HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} (OCR): 'ocr_stage_key' must be a string or null.", index)
                                }));
                            }
                        }
                    }
                    "parse" | "report" => {
                        // No specific fields to validate for these types yet beyond 'type' and 'command'
                    }
                    _ => { // Unknown stage types are currently permitted. Worker will ignore/fail.
                    }
                }
            } else {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": format!("Stage {} must be an object.", index)
                }));
            }
        }
    } else {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "'stages' must be an array."}));
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
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create pipeline."}))
        }
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
        Err(sqlx::Error::RowNotFound) => return HttpResponse::NotFound().finish(),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if user.role != "admin" && existing.org_id != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }

    if data.org_id != existing.org_id {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Organization ID cannot be changed"}));
    }

    if data.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Pipeline name cannot be empty."}));
    }
    if !data.stages.is_array() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "'stages' must be an array."}));
    }

    match Pipeline::update(&pool, pipeline_id, &data.name, data.stages.clone()).await {
        Ok(p) => HttpResponse::Ok().json(p),
        Err(_) => HttpResponse::InternalServerError().finish(),
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
        Err(sqlx::Error::RowNotFound) => return HttpResponse::NotFound().finish(),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if user.role != "admin" && existing.org_id != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }

    match Pipeline::delete(&pool, pipeline_id).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(create_pipeline)
        .service(list_pipelines)
        .service(update_pipeline)
        .service(delete_pipeline);
}

