use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::middleware::auth::AuthUser;
use crate::models::User as UserModel;
use crate::email::send_email;

#[derive(Serialize, FromRow, Debug)]
pub struct OrgUserView {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub confirmed: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct UserIdPath {
    user_id: Uuid,
}

pub async fn get_organization_users(user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if user.role != "org_admin" && user.role != "admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "You do not have permission to view users for this organization."}));
    }

    if user.role == "org_admin" && user.org_id == Uuid::nil() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Organization admin is not associated with an organization."}));
    }

    let query = r#"
        SELECT id, email, role, confirmed, is_active, created_at, deactivated_at
        FROM users
        WHERE org_id = $1
        ORDER BY email ASC
    "#;

    match sqlx::query_as::<_, OrgUserView>(query)
        .bind(user.org_id)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(e) => {
            log::error!("Failed to fetch users for organization {}: {:?}", user.org_id, e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to retrieve users for the organization."}))
        }
    }
}

async fn get_and_authorize_target_user_for_org_action(
    pool: &PgPool,
    org_admin_user_id: Uuid,
    org_admin_org_id: Uuid,
    target_user_id: Uuid,
) -> Result<UserModel, HttpResponse> {
    if target_user_id == org_admin_user_id {
        return Err(HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Organization administrators cannot manage their own account using this function."})));
    }

    match UserModel::find_by_id_for_admin(pool, target_user_id).await {
        Ok(Some(target_user)) => {
            if target_user.org_id != org_admin_org_id {
                Err(HttpResponse::Forbidden().json(serde_json::json!({"error": "This user does not belong to your organization."})))
            } else if target_user.role == "admin" || target_user.role == "org_admin" {
                Err(HttpResponse::Forbidden().json(serde_json::json!({"error": "Organization administrators cannot manage other administrators using this function."})))
            } else {
                Ok(target_user)
            }
        }
        Ok(None) => Err(HttpResponse::NotFound().json(serde_json::json!({"error": "Target user not found."}))),
        Err(e) => {
            log::error!("DB error fetching target user {} for org action by org_admin {}: {:?}", target_user_id, org_admin_user_id, e);
            Err(HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error fetching user information."})))
        }
    }
}

pub async fn remove_user_from_organization(
    org_admin: AuthUser,
    path: web::Path<UserIdPath>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if org_admin.role != "org_admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Only organization administrators can perform this action."}));
    }
    let target_user_id = path.user_id;

    match get_and_authorize_target_user_for_org_action(&pool, org_admin.user_id, org_admin.org_id, target_user_id).await {
        Ok(_) => {
            match sqlx::query("UPDATE users SET is_active = false, deactivated_at = NOW() WHERE id = $1 AND org_id = $2")
                .bind(target_user_id)
                .bind(org_admin.org_id)
                .execute(pool.as_ref())
                .await
            {
                Ok(result) if result.rows_affected() > 0 => {
                    log::info!("User {} removed (deactivated) from organization {} by org_admin {}", target_user_id, org_admin.org_id, org_admin.user_id);
                    HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User has been removed (deactivated) from the organization."}))
                }
                Ok(_) => {
                    log::warn!("User {} removal from org {} by org_admin {} had no effect (already deactivated or not found).", target_user_id, org_admin.org_id, org_admin.user_id);
                    HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User was already in the requested state or not found during update."}))
                }
                Err(e) => {
                    log::error!("Failed to deactivate user {} in org {} by org_admin {}: {:?}", target_user_id, org_admin.org_id, org_admin.user_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update user status for removal."}))
                }
            }
        }
        Err(resp) => resp,
    }
}

pub async fn deactivate_user_in_organization(
    org_admin: AuthUser,
    path: web::Path<UserIdPath>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if org_admin.role != "org_admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only organization administrators can perform this action."}));
    }
    let target_user_id = path.user_id;
    match get_and_authorize_target_user_for_org_action(&pool, org_admin.user_id, org_admin.org_id, target_user_id).await {
        Ok(target_user) => {
            if !target_user.is_active {
                return HttpResponse::BadRequest().json(serde_json::json!({"error": "User is already deactivated."}));
            }
            match sqlx::query("UPDATE users SET is_active = false, deactivated_at = NOW() WHERE id = $1 AND org_id = $2")
                .bind(target_user_id)
                .bind(org_admin.org_id)
                .execute(pool.as_ref())
                .await
            {
                Ok(result) if result.rows_affected() > 0 => {
                    log::info!("User {} deactivated in organization {} by org_admin {}", target_user_id, org_admin.org_id, org_admin.user_id);
                    HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User deactivated successfully."}))
                }
                Ok(_) => HttpResponse::InternalServerError().json(serde_json::json!({"error": "Deactivation failed, user not found or no change."})),
                Err(e) => {
                    log::error!("Failed to deactivate user {} in org {} by org_admin {}: {:?}", target_user_id, org_admin.org_id, org_admin.user_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to deactivate user."}))
                }
            }
        }
        Err(resp) => resp,
    }
}

pub async fn reactivate_user_in_organization(
    org_admin: AuthUser,
    path: web::Path<UserIdPath>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if org_admin.role != "org_admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only organization administrators can perform this action."}));
    }
    let target_user_id = path.user_id;
    match get_and_authorize_target_user_for_org_action(&pool, org_admin.user_id, org_admin.org_id, target_user_id).await {
        Ok(target_user) => {
            if target_user.is_active {
                return HttpResponse::BadRequest().json(serde_json::json!({"error": "User is already active."}));
            }
            match sqlx::query("UPDATE users SET is_active = true, deactivated_at = NULL WHERE id = $1 AND org_id = $2")
                .bind(target_user_id)
                .bind(org_admin.org_id)
                .execute(pool.as_ref())
                .await
            {
                Ok(result) if result.rows_affected() > 0 => {
                    log::info!("User {} reactivated in organization {} by org_admin {}", target_user_id, org_admin.org_id, org_admin.user_id);
                    HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User reactivated successfully."}))
                }
                Ok(_) => HttpResponse::InternalServerError().json(serde_json::json!({"error": "Reactivation failed, user not found or no change."})),
                Err(e) => {
                    log::error!("Failed to reactivate user {} in org {} by org_admin {}: {:?}", target_user_id, org_admin.org_id, org_admin.user_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to reactivate user."}))
                }
            }
        }
        Err(resp) => resp,
    }
}

pub async fn resend_confirmation_email_org_user(
    org_admin: AuthUser,
    path: web::Path<UserIdPath>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if org_admin.role != "org_admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only organization administrators can perform this action."}));
    }
    let target_user_id = path.user_id;

    match get_and_authorize_target_user_for_org_action(&pool, org_admin.user_id, org_admin.org_id, target_user_id).await {
        Ok(target_user) => {
            if target_user.confirmed {
                return HttpResponse::BadRequest().json(serde_json::json!({"error": "User's email is already confirmed."}));
            }

            let new_confirmation_token = Uuid::new_v4();
            match sqlx::query("UPDATE users SET confirmed = false, confirmation_token = $1 WHERE id = $2 AND org_id = $3")
                .bind(new_confirmation_token)
                .bind(target_user_id)
                .bind(org_admin.org_id)
                .execute(pool.as_ref())
                .await
            {
                Ok(result) if result.rows_affected() > 0 => {
                    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
                    let confirmation_link = format!("{}/api/confirm/{}", base_url, new_confirmation_token);

                    let org_name = match sqlx::query_scalar::<_, String>("SELECT name FROM organizations WHERE id = $1")
                        .bind(org_admin.org_id)
                        .fetch_one(pool.as_ref())
                        .await {
                            Ok(name) => name,
                            Err(_) => "Your Organization".to_string(),
                        };

                    let email_subject = format!("Confirm your email for {} on crPipeline", org_name);
                    let email_body = format!(
                        r#"Hello {},

Please confirm your email address for the organization '{}' on crPipeline by clicking the link below:
{}

If you did not request this, please ignore this email.

Thank you,
The crPipeline Team"#,
                        target_user.email, org_name, confirmation_link
                    );

                    if let Err(e) = send_email(&target_user.email, &email_subject, &email_body).await {
                        log::error!("Failed to resend confirmation email to {} (user {}) by org_admin {}: {:?}", target_user.email, target_user_id, org_admin.user_id, e);
                        HttpResponse::InternalServerError().json(serde_json::json!({"error": "Confirmation token updated, but failed to send email."}))
                    } else {
                        log::info!("Confirmation email resent to {} (user {}) by org_admin {}", target_user.email, target_user_id, org_admin.user_id);
                        HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "Confirmation email has been resent."}))
                    }
                }
                Ok(_) => {
                    log::warn!("Resend confirmation for user {} by org_admin {} had no effect (user not found or already confirmed).", target_user_id, org_admin.user_id);
                    HttpResponse::NotFound().json(serde_json::json!({"error": "Failed to update confirmation token for user, or user already confirmed."}))
                }
                Err(e) => {
                    log::error!("Database error updating confirmation token for user {} by org_admin {}: {:?}", target_user_id, org_admin.user_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error while trying to resend confirmation."}))
                }
            }
        }
        Err(resp) => resp,
    }
}

