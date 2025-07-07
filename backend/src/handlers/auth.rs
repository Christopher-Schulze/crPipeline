use crate::email::enqueue_email;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser; // Keep for 'me' handler
use crate::middleware::jwt::create_jwt; // Keep for login, not for register response
use crate::models::{NewUser, User};
use actix_web::{get, http::StatusCode, post, web, HttpResponse, ResponseError};
use argon2::{Argon2, PasswordHasher};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
// log_action removed from here, as it's not suitable for public registration without an actor AuthUser
// For admin actions, log_action would be used in those specific admin handlers.
use crate::metrics::AUTH_FAILURE_COUNTER;
use actix_web::cookie::time::Duration as ActixDuration; // For cookie Max-Age
use actix_web::cookie::SameSite; // For cookie SameSite attribute
use argon2::password_hash::SaltString;
use chrono::Utc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct RegisterInput {
    pub org_id: uuid::Uuid,
    pub email: String,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub success: bool,
}

#[derive(Deserialize)]
pub struct ResetRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetInput {
    pub token: Uuid,
    pub password: String,
}

#[post("/register")]
#[tracing::instrument(skip(data, pool))]
pub async fn register(data: web::Json<RegisterInput>, pool: web::Data<PgPool>) -> HttpResponse {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = match Argon2::default().hash_password(data.password.as_bytes(), &salt) {
        Ok(ph) => ph.to_string(),
        Err(e) => {
            log::error!("Password hash error: {:?}", e);
            return ApiError::new(
                "Failed to register user.",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .error_response();
        }
    };
    let new_user_role = data.role.clone().unwrap_or_else(|| "user".to_string());
    // Further validation for `new_user_role` could be added here if needed,
    // e.g., ensuring only specific roles can be set, or that "admin" cannot be self-assigned.
    // For now, it directly uses the provided role or defaults to "user".

    let user_to_create = NewUser {
        org_id: data.org_id,
        email: data.email.clone(),
        password_hash,
        role: new_user_role,
    };

    match User::create(&pool, user_to_create).await {
        Ok(u) => {
            // log_action removed for public registration endpoint
            let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
            // u.confirmation_token should always be Some due to User::create logic
            let link = format!(
                "{}/api/confirm/{}",
                base,
                u.confirmation_token.unwrap_or_else(Uuid::new_v4)
            );

            if let Err(e) = enqueue_email(&u.email, "Confirm your account", &link).await {
                log::warn!("Failed to send confirmation email to {}: {:?}", u.email, e);
                // Still return Ok for registration, email is auxiliary. User can request another confirmation.
            }

            // Do not auto-login. Client should redirect to login page or prompt to check email.
            HttpResponse::Ok().json(AuthResponse { success: true })
        }
        Err(e) => {
            log::error!("Failed to create user {}: {:?}", data.email, e);
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return HttpResponse::Conflict()
                        .json(serde_json::json!({"error": "Email address already in use."}));
                }
            }
            ApiError::from_db("Failed to register user.", e).error_response()
        }
    }
}

#[derive(Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[post("/login")]
#[tracing::instrument(skip(data, pool))]
pub async fn login(
    data: web::Json<LoginInput>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
    match User::find_by_email(&pool, &data.email).await {
        Ok(user) => {
            if !user.is_active {
                AUTH_FAILURE_COUNTER.with_label_values(&["inactive"]).inc();
                log::warn!("Login attempt for deactivated user: {}", data.email);
                return Err(ApiError::new(
                    "Your account has been deactivated. Please contact an administrator.",
                    actix_web::http::StatusCode::UNAUTHORIZED,
                ));
            }
            if user.verify_password(&data.password) {
                if !user.confirmed {
                    AUTH_FAILURE_COUNTER
                        .with_label_values(&["unconfirmed"])
                        .inc();
                    log::warn!("Login attempt for unconfirmed user: {}", data.email);
                    return Err(ApiError::new(
                        "Account not confirmed. Please check your email or contact an administrator to resend confirmation.",
                        actix_web::http::StatusCode::UNAUTHORIZED,
                    ));
                }

                let token = match create_jwt(user.id, user.org_id, &user.role) {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("Failed to create JWT for {}: {:?}", user.email, e);
                        return Err(ApiError::new(
                            "Login failed",
                            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                        ));
                    }
                }; // JWT has 24h expiry

                // Determine if 'Secure' flag should be set based on BASE_URL
                let base_url_str =
                    std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
                let secure_cookie = base_url_str.starts_with("https://");

                let mut cookie_builder = actix_web::cookie::Cookie::build("token", token)
                    .path("/")
                    .http_only(true) // Already set, good
                    .same_site(SameSite::Lax) // Add SameSite=Lax
                    .max_age(ActixDuration::hours(24)); // Align with JWT expiry

                if secure_cookie {
                    cookie_builder = cookie_builder.secure(true); // Set Secure flag if served over HTTPS
                }

                let cookie = cookie_builder.finish();

                log::info!(
                    "User {} logged in successfully. Secure cookie: {}",
                    user.email,
                    secure_cookie
                );
                return Ok(HttpResponse::Ok()
                    .cookie(cookie)
                    .json(AuthResponse { success: true }));
            } else {
                AUTH_FAILURE_COUNTER
                    .with_label_values(&["invalid_password"])
                    .inc();
                log::warn!(
                    "Failed login attempt for user: {} (invalid password)",
                    data.email
                );
                return Err(ApiError::new(
                    "Invalid email or password.",
                    actix_web::http::StatusCode::UNAUTHORIZED,
                ));
            }
        }
        Err(sqlx::Error::RowNotFound) => {
            AUTH_FAILURE_COUNTER
                .with_label_values(&["user_not_found"])
                .inc();
            log::warn!("Failed login attempt: User {} not found.", data.email);
            Err(ApiError::new(
                "Invalid email or password.",
                actix_web::http::StatusCode::UNAUTHORIZED,
            ))
        }
        Err(e) => {
            AUTH_FAILURE_COUNTER.with_label_values(&["db_error"]).inc();
            log::error!(
                "Database error during login for user {}: {:?}",
                data.email,
                e
            );
            Err(ApiError::new(
                "Login failed due to a server error.",
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

#[post("/logout")]
#[tracing::instrument]
async fn logout() -> HttpResponse {
    let cookie = actix_web::cookie::Cookie::build("token", "")
        .path("/")
        .max_age(ActixDuration::ZERO)
        .finish();
    HttpResponse::Ok().cookie(cookie).finish()
}

#[get("/confirm/{token}")]
#[tracing::instrument(skip(pool))]
async fn confirm(path: web::Path<Uuid>, pool: web::Data<PgPool>) -> HttpResponse {
    match User::confirm(&pool, *path).await {
        Ok(Some(_)) => HttpResponse::Ok().finish(),
        Ok(None) => ApiError::new("Invalid token", StatusCode::BAD_REQUEST).error_response(),
        Err(_) => {
            ApiError::new("Failed to confirm", StatusCode::INTERNAL_SERVER_ERROR).error_response()
        }
    }
}

#[post("/request_reset")]
#[tracing::instrument(skip(data, pool))]
async fn request_reset(data: web::Json<ResetRequest>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Ok(user) = User::find_by_email(&pool, &data.email).await {
        let token = Uuid::new_v4();
        let expires = Utc::now() + chrono::Duration::hours(24);
        if User::set_reset_token(&pool, user.id, token, expires)
            .await
            .is_ok()
        {
            let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
            let link = format!("{}/reset?token={}", base, token);
            let _ = enqueue_email(&user.email, "Password reset", &link).await;
            return HttpResponse::Ok().finish();
        }
    }
    HttpResponse::Ok().finish()
}

#[post("/reset_password")]
#[tracing::instrument(skip(data, pool))]
async fn reset_password(data: web::Json<ResetInput>, pool: web::Data<PgPool>) -> HttpResponse {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = match Argon2::default().hash_password(data.password.as_bytes(), &salt) {
        Ok(ph) => ph.to_string(),
        Err(_) => {
            return ApiError::new("Password hashing failed", StatusCode::INTERNAL_SERVER_ERROR)
                .error_response()
        }
    };
    match User::reset_with_token(&pool, data.token, password_hash).await {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => ApiError::new("Invalid token", StatusCode::BAD_REQUEST).error_response(),
        Err(_) => ApiError::new(
            "Failed to reset password",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
        .error_response(),
    }
}

#[post("/logout")]
async fn logout() -> HttpResponse {
    let cookie = actix_web::cookie::Cookie::build("token", "")
        .max_age(ActixDuration::ZERO)
        .finish();
    HttpResponse::Ok().cookie(cookie).finish()
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register)
        .service(login)
        .service(logout)
        .service(me)
        .service(confirm)
        .service(request_reset)
        .service(reset_password);
}

#[get("/me")]
#[tracing::instrument(skip(user))]
async fn me(user: AuthUser) -> HttpResponse {
    HttpResponse::Ok().json(
        serde_json::json!({"user_id": user.user_id, "org_id": user.org_id, "role": user.role}),
    )
}
