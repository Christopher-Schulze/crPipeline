use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use crate::models::{User as UserModel, NewUser};
use crate::middleware::auth::AuthUser;
use crate::email::send_email_retry;
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use rand::Rng;

#[derive(Deserialize, Debug)]
pub struct OrgInviteUserPayload {
    pub email: String,
}

pub async fn invite_user_to_organization(
    current_org_admin: AuthUser,
    payload: web::Json<OrgInviteUserPayload>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if current_org_admin.role != "org_admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Only organization administrators can invite users to their organization."}));
    }

    let target_email = payload.email.trim().to_lowercase();
    if target_email.is_empty() || !target_email.contains('@') {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Invalid email address provided."}));
    }

    match UserModel::find_by_email(&pool, &target_email).await {
        Ok(existing_user) => {
            if existing_user.org_id == current_org_admin.org_id {
                return HttpResponse::Conflict()
                    .json(serde_json::json!({"error": "This user is already a member of your organization."}));
            } else {
                return HttpResponse::Conflict().json(serde_json::json!({
                    "error": "This email address is already associated with an account in a different organization. Please contact a global administrator if you need to move this user."
                }));
            }
        }
        Err(sqlx::Error::RowNotFound) => {}
        Err(e) => {
            log::error!("Database error when checking for existing user by email '{}': {:?}", target_email, e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Error checking user existence."}));
        }
    }

    let placeholder_password: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = match Argon2::default().hash_password(placeholder_password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(e) => {
            log::error!("Failed to hash placeholder password for invite: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to process invitation due to a security configuration issue."}));
        }
    };

    let new_user_data = NewUser {
        org_id: current_org_admin.org_id,
        email: target_email.clone(),
        password_hash,
        role: "user".to_string(),
    };

    match UserModel::create(&pool, new_user_data).await {
        Ok(created_user) => {
            let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
            let confirmation_link = format!("{}/api/confirm/{}", base_url, created_user.confirmation_token.unwrap_or_default());

            let org_name = match sqlx::query_scalar::<_, String>("SELECT name FROM organizations WHERE id = $1")
                .bind(current_org_admin.org_id)
                .fetch_one(pool.as_ref())
                .await {
                    Ok(name) => name,
                    Err(_) => "Your Organization".to_string(),
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

            if let Err(e) = send_email_retry(&pool, current_org_admin.org_id, current_org_admin.user_id, &target_email, &email_subject, &email_body, 3).await {
                log::error!("User {} created by org_admin {}, but failed to send confirmation email to {}: {:?}", created_user.id, current_org_admin.user_id, target_email, e);
                return HttpResponse::Accepted().json(serde_json::json!({
                    "success": true,
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

