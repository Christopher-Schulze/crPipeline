use actix_web::{get, post, web, HttpResponse};
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::OrgSettings;
use sqlx::PgPool;


#[get("/settings/{org_id}")]
async fn get_settings(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    let org_id_from_path = *path;

    // Authorization: Global admin can view any org's settings.
    // Other roles (e.g. "user", or a future "org_admin") can only view their own org's settings.
    if user.role != "admin" && org_id_from_path != user.org_id {
        log::warn!(
            "Unauthorized attempt to access settings for org {} by user {} (role: {}, user_org: {})",
            org_id_from_path, user.user_id, user.role, user.org_id
        );
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "You are not authorized to view these settings."}));
    }

    match OrgSettings::find(&pool, org_id_from_path).await {
        Ok(mut settings) => { // Make settings mutable for masking
            // Mask sensitive API keys before sending to frontend
            if settings.ai_api_key.as_ref().map_or(false, |k| !k.is_empty()) {
                settings.ai_api_key = Some("********".to_string());
            }
            if settings.ocr_api_key.as_ref().map_or(false, |k| !k.is_empty()) {
                settings.ocr_api_key = Some("********".to_string());
            }
            // Other sensitive fields could be masked here in the future

            HttpResponse::Ok().json(settings)
        },
        Err(sqlx::Error::RowNotFound) => {
            log::info!("Settings not found for org {}. A new default might be created on next update or implicitly.", org_id_from_path);
            // Depending on product requirements, this could return default settings instead of 404.
            // For now, if no settings row exists, it's treated as "not found".
            // Org creation should ideally create default settings record.
            HttpResponse::NotFound().json(serde_json::json!({"error": "Settings not found for this organization."}))
        }
        Err(e) => {
            log::error!("Failed to retrieve settings for org {}: {:?}", org_id_from_path, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to retrieve settings."}))
        }
    }
}

#[post("/settings")]
async fn update_settings(
    payload: web::Json<OrgSettings>, // The incoming settings from frontend
    user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let mut incoming_settings = payload.into_inner();

    // Authorization: Global admin can update any org's settings by specifying org_id in payload.
    // Other users can only update their own org's settings.
    if user.role != "admin" && incoming_settings.org_id != user.org_id {
        log::warn!(
            "Unauthorized attempt to update settings for org {} by user {} (role: {}, user_org: {})",
            incoming_settings.org_id, user.user_id, user.role, user.org_id
        );
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "You are not authorized to update these settings."}));
    }
    // Also, a non-admin user should not be able to change their org_id via the payload to something else.
    // The incoming_settings.org_id must match user.org_id if not admin.
    if user.role != "admin" && incoming_settings.org_id != user.org_id {
         // This check is somewhat redundant due to the one above, but emphasizes that org_id in payload must be theirs.
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid organization ID in request."}));
    }


    // Fetch current settings from DB to preserve keys if "********" is sent
    let current_settings = match OrgSettings::find(&pool, incoming_settings.org_id).await {
        Ok(cs) => cs,
        Err(sqlx::Error::RowNotFound) => {
            // This case implies an attempt to update settings for an org that might not exist or has no settings row.
            // If OrgSettings::update is purely UPDATE, this will fail.
            // For this scenario, we should prevent updating if the target settings don't exist,
            // as creating default settings here might be unexpected side-effect for an update endpoint.
            log::error!("Settings not found for org_id {} during update attempt. Org might not exist or setup is incomplete.", incoming_settings.org_id);
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Settings for the specified organization not found. Cannot update."}));
        }
        Err(e) => {
            log::error!("Failed to fetch current settings for org {}: {:?}", incoming_settings.org_id, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Could not retrieve current settings to safely update API keys."}));
        }
    };

    // Preserve AI API Key if "********" is sent (means user didn't change it from masked value)
    if incoming_settings.ai_api_key.as_deref() == Some("********") {
        incoming_settings.ai_api_key = current_settings.ai_api_key;
    }
    // If incoming_settings.ai_api_key is Some(""), it means user wants to clear it.
    // If it's Some("new_key"), it will be updated.
    // If it's None (e.g. frontend omits field if not changed), model's Option takes care.

    // Preserve OCR API Key if "********" is sent
    if incoming_settings.ocr_api_key.as_deref() == Some("********") {
        incoming_settings.ocr_api_key = current_settings.ocr_api_key;
    }

    // Now, incoming_settings contains the new values, or original values for keys if "********" was passed.
    match OrgSettings::update(&pool, incoming_settings).await {
        Ok(updated_settings_from_db) => {
            // Mask API keys again before sending back the updated settings in the response
            let mut response_settings = updated_settings_from_db;
            if response_settings.ai_api_key.as_ref().map_or(false, |k| !k.is_empty()) {
                response_settings.ai_api_key = Some("********".to_string());
            }
            if response_settings.ocr_api_key.as_ref().map_or(false, |k| !k.is_empty()) {
                response_settings.ocr_api_key = Some("********".to_string());
            }
            HttpResponse::Ok().json(response_settings)
        }
        Err(e) => {
            // Log the specific org_id for which the update failed.
            // The `incoming_settings` was moved into OrgSettings::update, so we use `user.org_id`
            // or more accurately, the org_id that was attempted to be updated.
            // Since `incoming_settings` is moved, we should ideally log `current_settings.org_id`
            // or ensure `incoming_settings.org_id` is cloned before move for logging.
            // For now, let's use `current_settings.org_id` for this log message.
            log::error!("Failed to update settings for org {}: {:?}", current_settings.org_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update settings."}))
        }
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_settings).service(update_settings);
}
