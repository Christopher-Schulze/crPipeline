use actix_web::{post, web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::{User as UserModel, NewUser};
use crate::email::send_email;
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use rand::Rng;

#[derive(Deserialize, Debug)]
pub struct InviteUserPayload {
    pub email: String,
    pub org_id: Uuid,
    pub role: Option<String>,
}

#[post("/admin/invite")]
pub async fn invite_user(
    admin: AuthUser,
    payload: web::Json<InviteUserPayload>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if admin.role != "admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Only global administrators can invite users."}));
    }
    let email = payload.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid email address."}));
    }

    match UserModel::find_by_email(&pool, &email).await {
        Ok(_) => {
            return HttpResponse::Conflict()
                .json(serde_json::json!({"error": "A user with this email already exists."}));
        }
        Err(sqlx::Error::RowNotFound) => {}
        Err(e) => {
            log::error!("DB error checking for existing user {}: {:?}", email, e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Database error checking email."}));
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
                .json(serde_json::json!({"error": "Failed to process invitation."}));
        }
    };

    let role = payload.role.clone().unwrap_or_else(|| "user".into());
    let new_user_data = NewUser {
        org_id: payload.org_id,
        email: email.clone(),
        password_hash,
        role,
    };

    match UserModel::create(&pool, new_user_data).await {
        Ok(created_user) => {
            let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
            let confirmation_link = format!("{}/api/confirm/{}", base_url, created_user.confirmation_token.unwrap_or_default());

            let email_subject = "You're invited to crPipeline";
            let email_body = format!(
                "Hello,\n\nYou have been invited to join crPipeline. Click the link below to confirm your account:\n{}\n",
                confirmation_link
            );
            if let Err(e) = send_email(&email, email_subject, &email_body).await {
                log::error!("Failed to send invite email to {}: {:?}", email, e);
                HttpResponse::Accepted().json(serde_json::json!({
                    "success": true,
                    "message": "User created but email could not be sent.",
                    "user_id": created_user.id
                }))
            } else {
                HttpResponse::Ok().json(serde_json::json!({"success": true, "user_id": created_user.id}))
            }
        }
        Err(e) => {
            log::error!("Failed to create invited user {}: {:?}", email, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create user."}))
        }
    }
}

