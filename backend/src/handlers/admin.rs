use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Serialize, Deserialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::{User as UserModel, NewUser}; // Added NewUser
use chrono::{DateTime, Utc};
use crate::email::send_email;
use argon2::{Argon2, PasswordHasher}; // For placeholder password in invite
use argon2::password_hash::SaltString; // For placeholder password in invite
use log;
use serde_json;

#[derive(Deserialize, Debug)] // For request payload
struct AssignRolePayload {
    role: String,
    org_id: Option<Uuid>,
}

#[derive(Serialize, FromRow, Debug)] // For list_all_users response
struct AdminUserView {
    id: Uuid,
    email: String,
    role: String,
    org_id: Uuid,
    organization_name: Option<String>,
    confirmed: bool,
    is_active: bool,
    deactivated_at: Option<DateTime<Utc>>,
}

#[get("/admin/users")]
async fn list_all_users(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    // 1. Authorization: Only global admins
    if user.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "You do not have permission to access this resource."}));
    }

    // 2. Fetch all users with their organization names
    let query = "
        SELECT
            u.id,
            u.email,
            u.role,
            u.org_id,
            o.name as organization_name,
            u.confirmed,
            u.is_active,
            u.deactivated_at
        FROM
            users u
        LEFT JOIN
            organizations o ON u.org_id = o.id
        ORDER BY
            u.email ASC
    ";

    match sqlx::query_as::<_, AdminUserView>(query)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(e) => {
            log::error!("Failed to fetch all users for admin view: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to retrieve user list."}))
        }
    }
}

#[post("/admin/users/{user_id}/assign_role")]
async fn assign_user_role(
    path_params: web::Path<Uuid>,    // user_id of the target user
    payload: web::Json<AssignRolePayload>,
    current_admin_user: AuthUser, // The authenticated global admin performing the action
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let target_user_id = path_params.into_inner();

    // 1. Authorization: Only global admins can perform this
    if current_admin_user.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only global administrators can assign roles."}));
    }

    // 2. Validation:
    // 2a. Admin cannot change their own role via this endpoint
    if target_user_id == current_admin_user.user_id {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Administrators cannot change their own role using this endpoint."}));
    }

    // 2b. Validate the target role string
    let new_role = payload.role.trim().to_lowercase();
    if new_role != "user" && new_role != "org_admin" {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid role specified. Allowed roles are 'user' or 'org_admin'."}));
    }

    // 2c. If assigning "org_admin", org_id must be provided and valid
    let mut target_org_id_for_update: Option<Uuid> = None;
    if new_role == "org_admin" {
        match payload.org_id {
            Some(org_uuid) => {
                match sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)")
                    .bind(org_uuid)
                    .fetch_one(pool.as_ref())
                    .await
                {
                    Ok(exists) if exists => target_org_id_for_update = Some(org_uuid),
                    Ok(_) => return HttpResponse::BadRequest().json(serde_json::json!({"error": format!("Organization with ID {} not found.", org_uuid)})),
                    Err(e) => {
                        log::error!("Failed to check existence of org_id {}: {:?}", org_uuid, e);
                        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error validating organization."}));
                    }
                }
            }
            None => return HttpResponse::BadRequest().json(serde_json::json!({"error": "An organization ID is required when assigning the 'org_admin' role."})),
        }
    }

    // 3. Fetch the target user
    let target_user = match sqlx::query_as::<_, UserModel>("SELECT * FROM users WHERE id = $1") // Use aliased UserModel
        .bind(target_user_id)
        .fetch_optional(pool.as_ref()) // Use fetch_optional for better error handling if user not found
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Target user not found."})),
        Err(e) => {
            log::error!("Failed to fetch target user {}: {:?}", target_user_id, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error fetching user."}));
        }
    };

    // 4. Safety Check: Prevent removing the last global admin's "admin" role
    if target_user.role == "admin" && new_role != "admin" {
        match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE role = 'admin'")
            .fetch_one(pool.as_ref())
            .await
        {
            Ok(admin_count) if admin_count <= 1 => {
                return HttpResponse::Forbidden().json(serde_json::json!({"error": "Cannot remove the last global administrator's 'admin' role."}));
            }
            Ok(_) => {} // Proceed if other admins exist
            Err(e) => {
                log::error!("Failed to count admin users: {:?}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error during safety check."}));
            }
        }
    }

    // 5. Update user's role and org_id
    let final_org_id_to_set = match payload.org_id {
        Some(new_org_uuid) => {
            if new_role != "org_admin" {
                 match sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)")
                     .bind(new_org_uuid)
                     .fetch_one(pool.as_ref())
                     .await
                 {
                     Ok(exists) if exists => new_org_uuid,
                     Ok(_) => {
                         log::warn!("Invalid org_id {} provided for user role update when role is 'user'.", new_org_uuid);
                         return HttpResponse::BadRequest().json(serde_json::json!({"error": format!("Organization with ID {} not found.", new_org_uuid)}));
                     }
                     Err(e) => {
                         log::error!("Failed to check existence of org_id {} for user role update: {:?}", new_org_uuid, e);
                         return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error validating organization."}));
                     }
                 }
            } else {
                target_org_id_for_update.unwrap()
            }
        }
        None => target_user.org_id,
    };

    let update_query = "UPDATE users SET role = $1, org_id = $2 WHERE id = $3";
    match sqlx::query(update_query)
        .bind(new_role.clone())
        .bind(final_org_id_to_set)
        .bind(target_user_id)
        .execute(pool.as_ref())
        .await
    {
        Ok(result) if result.rows_affected() > 0 => {
            log::info!("User {} role updated to {} for org_id {}. Action by admin: {}",
                target_user_id, new_role, final_org_id_to_set, current_admin_user.user_id);
            HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User role updated successfully."}))
        }
        Ok(_) => {
            log::warn!("User {} role update to {} for org_id {} attempted by admin {}, but no rows affected (user might not exist or data was the same).",
                target_user_id, new_role, final_org_id_to_set, current_admin_user.user_id);
            // User was fetched successfully earlier, so this likely means data was the same or concurrent modification.
            // Or, if role and org_id were already set to these values.
            HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User role and org assignment are already up to date or user not found during update."}))
        }
        Err(e) => {
            log::error!("Failed to update user {} role: {:?}", target_user_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error updating user role."}))
        }
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_all_users)
       .service(assign_user_role)
       .service(resend_confirmation_email)
       .service(deactivate_user)
       .service(reactivate_user)
       .service(invite_user) // Keep one invite_user service
       .service(update_user_profile);
}

#[derive(Deserialize, Debug)]
struct UpdateUserProfilePayload {
    email: Option<String>,
}

#[actix_web::put("/admin/users/{user_id}/profile")]
async fn update_user_profile(
    path_params: web::Path<Uuid>,
    payload: web::Json<UpdateUserProfilePayload>,
    current_admin_user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let target_user_id = path_params.into_inner();

    if current_admin_user.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only global administrators can update user profiles."}));
    }

    let target_user = match UserModel::find_by_id_for_admin(&pool, target_user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Target user not found."})),
        Err(e) => {
            log::error!("Failed to fetch target user {} for profile update: {:?}", target_user_id, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error fetching user."}));
        }
    };

    if let Some(new_email_str) = &payload.email {
        let trimmed_new_email = new_email_str.trim();
        if trimmed_new_email.is_empty() {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Email cannot be empty."}));
        }
        if !trimmed_new_email.contains('@') {
             return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid email format."}));
        }

        if trimmed_new_email.to_lowercase() != target_user.email.to_lowercase() {
            // Email is changing. Check for uniqueness.
            match UserModel::find_by_email(&pool, trimmed_new_email).await {
                Ok(_existing_user) => return HttpResponse::Conflict().json(serde_json::json!({"error": "New email address is already in use."})),
                Err(sqlx::Error::RowNotFound) => { /* New email is available */ }
                Err(e) => {
                    log::error!("DB error checking new email existence for user {}: {:?}", target_user_id, e);
                    return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error checking email availability."}));
                }
            }

            let new_confirmation_token = Uuid::new_v4();
            match UserModel::update_email_and_set_unconfirmed(&pool, target_user_id, trimmed_new_email, new_confirmation_token).await {
                Ok(rows_affected) if rows_affected > 0 => {
                    log::info!("Admin {} updated email for user {} to {}. Marked unconfirmed.", current_admin_user.user_id, target_user_id, trimmed_new_email);

                    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
                    let confirmation_link = format!("{}/api/confirm/{}", base_url, new_confirmation_token);
                    let email_subject = "Your Email Address Was Changed - Confirm New Email";
                    let email_body = format!(
r#"Hello,

Your email address associated with crPipeline was changed by an administrator to {}.
Please confirm this new email address by clicking the link below:
{}

If you did not request this change, please contact support immediately."#,
trimmed_new_email, confirmation_link);

                    if let Err(e) = send_email(trimmed_new_email, email_subject, &email_body).await {
                        log::error!("Failed to send confirmation email to new address {}: {:?}", trimmed_new_email, e);
                        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Profile email updated, but failed to send confirmation to new email. User must confirm via other means or contact support."}));
                    }
                    HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User email updated. A confirmation email has been sent to the new address."}))
                }
                Ok(_) => {
                    log::error!("Failed to update email for user {} (0 rows affected). User ID might be wrong or race condition.", target_user_id);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update user email."}))
                }
                Err(e) => {
                    log::error!("Database error updating email for user {}: {:?}", target_user_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error updating email."}))
                }
            }
        } else {
            HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "No changes to email address."}))
        }
    } else {
        HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "No email provided for update."}))
    }
}

// Removed duplicated InviteUserPayload and other functions that were part of the incorrect SEARCH block
