use actix_web::{web, get, post, HttpResponse, Scope};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use crate::models::{Organization, NewOrganization, OrgSettings, User as UserModel, NewUser};
use crate::middleware::auth::AuthUser;
use crate::email::send_email; // For sending invitation emails
use argon2::{Argon2, PasswordHasher}; // For placeholder password in invite
use argon2::password_hash::SaltString; // For placeholder password in invite
use rand::Rng; // For generating random passwords
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct OrgInput { pub name: String }

// Response struct for users within an organization, tailored for org admins
#[derive(Serialize, FromRow, Debug)]
pub struct OrgUserView {
    id: Uuid,
    email: String,
    role: String, // Should typically be 'user' for users managed by an org_admin
    confirmed: bool,
    is_active: bool,
    created_at: DateTime<Utc>,
    deactivated_at: Option<DateTime<Utc>>,
}

// Payload for inviting a user to the organization
#[derive(Deserialize, Debug)]
pub struct OrgInviteUserPayload {
    email: String,
}

#[post("/orgs")]
async fn create_org(data: web::Json<OrgInput>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    // Only global admins can create organizations
    if user.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "You do not have permission to create organizations."}));
    }

    let org_name = data.name.trim();
    if org_name.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Organization name cannot be empty."}));
    }

    let new_org = NewOrganization { name: org_name.to_string() };
    match Organization::create(&pool, new_org).await {
        Ok(created_org) => {
            match OrgSettings::create_default(&pool, created_org.id).await {
                Ok(_) => HttpResponse::Ok().json(created_org),
                Err(e) => {
                    log::error!("Failed to create default settings for new org {}: {:?}", created_org.id, e);
                    // Organization was created, but settings failed. This is a partial success.
                    // Depending on requirements, might want to roll back org creation or just log and return org.
                    // For now, returning the org but logging the error.
                    HttpResponse::Ok().json(serde_json::json!({
                        "warning": "Organization created, but default settings failed to initialize.",
                        "organization": created_org
                    }))
                }
            }
        },
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("organizations_name_key") => {
            HttpResponse::Conflict().json(serde_json::json!({"error": "An organization with this name already exists."}))
        }
        Err(e) => {
            log::error!("Failed to create organization '{}': {:?}", org_name, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create organization."}))
        }
    }
}

#[get("/orgs")]
async fn list_orgs(user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    let result = if user.role == "admin" {
        Organization::all(pool.as_ref()).await
    } else {
        // Non-global admins (e.g., "user", "org_admin") get only their own organization
        sqlx::query_as::<_, Organization>("SELECT id, name, api_key FROM organizations WHERE id = $1")
            .bind(user.org_id)
            .fetch_optional(pool.as_ref())
            .await
            .map(|opt_org| opt_org.map_or_else(Vec::new, |org| vec![org]))
            .map_err(|e| {
                log::error!("Database error fetching single organization for user {}: {:?}", user.user_id, e);
                e
            })
    };

    match result {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(_) => {
            // Specific error already logged
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to retrieve organizations."}))
        }
    }
}

async fn get_organization_users(user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if user.role != "org_admin" && user.role != "admin" { // Allow global admin to use this too
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "You do not have permission to view users for this organization."}));
    }

    // For org_admin, org_id is from their session. For global admin, they'd need to specify.
    // This endpoint is /me/users, so it's implicitly the authenticated user's org.
    // If a global admin needs to see users for *any* org, they should use /admin/users or a new specific endpoint.
    // So, we strictly use user.org_id.
    if user.role == "org_admin" && user.org_id == Uuid::nil() { // Nil UUID check
         return HttpResponse::BadRequest().json(serde_json::json!({"error": "Organization admin is not associated with an organization."}));
    }


    let query = "
        SELECT id, email, role, confirmed, is_active, created_at, deactivated_at
        FROM users
        WHERE org_id = $1
        ORDER BY email ASC
    ";

    match sqlx::query_as::<_, OrgUserView>(query)
        .bind(user.org_id)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(e) => {
            log::error!("Failed to fetch users for organization {}: {:?}", user.org_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to retrieve users for the organization."}))
        }
    }
}

async fn invite_user_to_organization(
    current_org_admin: AuthUser,
    payload: web::Json<OrgInviteUserPayload>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if current_org_admin.role != "org_admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only organization administrators can invite users to their organization."}));
    }

    let target_email = payload.email.trim().to_lowercase();
    if target_email.is_empty() || !target_email.contains('@') {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid email address provided."}));
    }

    // Check if user already exists
    match UserModel::find_by_email(&pool, &target_email).await {
        Ok(existing_user) => {
            // User exists. Check if they are already in this org.
            if existing_user.org_id == current_org_admin.org_id {
                return HttpResponse::Conflict().json(serde_json::json!({"error": "This user is already a member of your organization."}));
            } else {
                // User exists but in a different organization.
                // Org admins cannot pull users from other orgs. Global admin task.
                return HttpResponse::Conflict().json(serde_json::json!({
                    "error": "This email address is already associated with an account in a different organization. Please contact a global administrator if you need to move this user."
                }));
            }
        }
        Err(sqlx::Error::RowNotFound) => {
            // User does not exist, proceed to create and invite.
        }
        Err(e) => {
            log::error!("Database error when checking for existing user by email '{}': {:?}", target_email, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Error checking user existence."}));
        }
    }

    // Generate a secure random password (it will be temporary, user confirms & sets new one)
    // Or, better: invite token that leads to password creation page.
    // For now, sticking to creating user directly with placeholder password and confirmation.
    let placeholder_password = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect::<String>();

    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = match Argon2::default().hash_password(placeholder_password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(e) => {
            log::error!("Failed to hash placeholder password for invite: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to process invitation due to a security configuration issue."}));
        }
    };

    let new_user_data = NewUser {
        org_id: current_org_admin.org_id,
        email: target_email.clone(),
        password_hash,
        role: "user".to_string(), // Org admins can only invite users with 'user' role to their org
    };

    match UserModel::create(&pool, new_user_data).await {
        Ok(created_user) => {
            let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
            let confirmation_link = format!("{}/api/confirm/{}", base_url, created_user.confirmation_token.unwrap_or_default());

            // Fetch organization name for email
            let org_name = match sqlx::query_scalar::<_, String>("SELECT name FROM organizations WHERE id = $1")
                .bind(current_org_admin.org_id)
                .fetch_one(pool.as_ref())
                .await {
                    Ok(name) => name,
                    Err(_) => "Your Organization".to_string(), // Fallback
                };

            let email_subject = format!("You're invited to join {} on crPipeline", org_name);
            let email_body = format!(
r#"Hello {},

You have been invited by an administrator to join the organization '{}' on crPipeline.
Please confirm your email address and set up your account by clicking the link below:
{}

If you were not expecting this invitation, you can safely ignore this email.

Thank you,
The crPipeline Team"#,
                target_email, org_name, confirmation_link
            );

            if let Err(e) = send_email(&target_email, &email_subject, &email_body).await {
                log::error!("User {} created by org_admin {}, but failed to send confirmation email to {}: {:?}", created_user.id, current_org_admin.user_id, target_email, e);
                // User is created, but email failed. This is a partial success.
                // The user will exist but won't be able to log in until confirmed.
                // Global admin might need to intervene or resend confirmation.
                return HttpResponse::Accepted().json(serde_json::json!({
                    "success": true, // Technically user was created
                    "message": "User account created, but the invitation email could not be sent. Please ask a global administrator to resend the confirmation or contact support.",
                    "user_id": created_user.id
                }));
            }

            log::info!("User {} invited to organization {} by org_admin {}. Email: {}", created_user.id, current_org_admin.org_id, current_org_admin.user_id, target_email);
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Invitation sent successfully. The user needs to confirm their email address.",
                "user_id": created_user.id
            }))
        }
        Err(sqlx::Error::Database(db_err)) => {
            // Check for unique constraint violation on email, though find_by_email should catch it.
            // This is a fallback.
            if db_err.constraint().map_or(false, |name| name.contains("users_email_key")) {
                 HttpResponse::Conflict().json(serde_json::json!({"error": "This email address is already registered."}))
            } else {
                log::error!("Database error creating user for invite by org_admin {}: {:?}", current_org_admin.user_id, db_err);
                HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create user account for invitation."}))
            }
        }
        Err(e) => {
            log::error!("Generic error creating user for invite by org_admin {}: {:?}", current_org_admin.user_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "An unexpected error occurred during user invitation."}))
        }
    }
}

// --- Org Admin User Management Actions ---

#[derive(Deserialize)]
struct UserIdPath {
    user_id: Uuid,
}

// Helper function to check if target user is in org_admin's org and is not the org_admin themselves
async fn get_and_authorize_target_user_for_org_action(
    pool: &PgPool,
    org_admin_user_id: Uuid,
    org_admin_org_id: Uuid,
    target_user_id: Uuid,
) -> Result<UserModel, HttpResponse> {
    if target_user_id == org_admin_user_id {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({"error": "Organization administrators cannot manage their own account using this function."})));
    }

    match UserModel::find_by_id_for_admin(pool, target_user_id).await {
        Ok(Some(target_user)) => {
            if target_user.org_id != org_admin_org_id {
                Err(HttpResponse::Forbidden().json(serde_json::json!({"error": "This user does not belong to your organization."})))
            } else if target_user.role == "admin" || target_user.role == "org_admin" {
                 Err(HttpResponse::Forbidden().json(serde_json::json!({"error": "Organization administrators cannot manage other administrators using this function."})))
            }
            else {
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

async fn remove_user_from_organization(
    org_admin: AuthUser,
    path: web::Path<UserIdPath>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if org_admin.role != "org_admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only organization administrators can perform this action."}));
    }
    let target_user_id = path.user_id;

    match get_and_authorize_target_user_for_org_action(&pool, org_admin.user_id, org_admin.org_id, target_user_id).await {
        Ok(_target_user) => { // User is authorized and fetched
            // Action: Deactivate the user
            match sqlx::query("UPDATE users SET is_active = false, deactivated_at = NOW() WHERE id = $1 AND org_id = $2")
                .bind(target_user_id)
                .bind(org_admin.org_id) // Ensure still scoped to org
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
        Err(resp) => resp, // Return the error HttpResponse from the helper
    }
}

async fn deactivate_user_in_organization(
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
                Ok(_) => HttpResponse::InternalServerError().json(serde_json::json!({"error": "Deactivation failed, user not found or no change."})), // Should be caught by get_and_authorize
                Err(e) => {
                    log::error!("Failed to deactivate user {} in org {} by org_admin {}: {:?}", target_user_id, org_admin.org_id, org_admin.user_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to deactivate user."}))
                }
            }
        }
        Err(resp) => resp,
    }
}

async fn reactivate_user_in_organization(
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

async fn resend_confirmation_email_org_user(
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
            // Update token and ensure user is marked as unconfirmed (should already be, but good practice)
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


pub fn org_routes() -> Scope {
    web::scope("/orgs")
        .service(create_org)
        .service(list_orgs)
}

pub fn org_me_routes() -> Scope {
    web::scope("/organizations/me")
        .route("/users", web::get().to(get_organization_users))
        .route("/invite", web::post().to(invite_user_to_organization))
        .route("/users/{user_id}/remove", web::post().to(remove_user_from_organization))
        .route("/users/{user_id}/deactivate", web::post().to(deactivate_user_in_organization))
        .route("/users/{user_id}/reactivate", web::post().to(reactivate_user_in_organization))
        .route("/users/{user_id}/resend_confirmation", web::post().to(resend_confirmation_email_org_user))
}

// Main routes function to be called in main.rs or lib.rs
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(org_routes())
       .service(org_me_routes());
}
