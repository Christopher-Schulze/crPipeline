use actix_web::{post, get, web, HttpResponse};
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use crate::models::{User, NewUser};
use crate::middleware::jwt::create_jwt; // Keep for login, not for register response
use crate::middleware::auth::AuthUser; // Keep for 'me' handler
use crate::email::send_email;
use argon2::{Argon2, PasswordHasher};
// log_action removed from here, as it's not suitable for public registration without an actor AuthUser
// For admin actions, log_action would be used in those specific admin handlers.
use argon2::password_hash::SaltString;
use actix_web::cookie::SameSite; // For cookie SameSite attribute
use actix_web::cookie::time::Duration as ActixDuration; // For cookie Max-Age
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
pub struct ResetRequest { pub email: String }

#[derive(Deserialize)]
pub struct ResetInput { pub token: Uuid, pub password: String }

#[post("/register")]
pub async fn register(data: web::Json<RegisterInput>, pool: web::Data<PgPool>) -> HttpResponse {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = match Argon2::default().hash_password(data.password.as_bytes(), &salt) {
        Ok(ph) => ph.to_string(),
        Err(e) => {
            log::error!("Password hash error: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to register user."}));
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
            let link = format!("{}/api/confirm/{}", base, u.confirmation_token.unwrap_or_else(Uuid::new_v4));

            if let Err(e) = send_email(&u.email, "Confirm your account", &link).await {
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
                    return HttpResponse::Conflict().json(serde_json::json!({"error": "Email address already in use."}));
                }
            }
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to register user."}))
        }
    }
}

#[derive(Deserialize)]
pub struct LoginInput { pub email: String, pub password: String }

#[post("/login")]
pub async fn login(
    data: web::Json<LoginInput>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
    match User::find_by_email(&pool, &data.email).await {
        Ok(user) => {
            if !user.is_active {
                log::warn!("Login attempt for deactivated user: {}", data.email);
                return Err(ApiError::new(
                    "Your account has been deactivated. Please contact an administrator.",
                    actix_web::http::StatusCode::UNAUTHORIZED,
                ));
            }
            if user.verify_password(&data.password) {
                if !user.confirmed {
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
                let base_url_str = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
                let secure_cookie = base_url_str.starts_with("https://");

                let mut cookie_builder = actix_web::cookie::Cookie::build("token", token)
                    .path("/")
                    .http_only(true)      // Already set, good
                    .same_site(SameSite::Lax) // Add SameSite=Lax
                    .max_age(ActixDuration::hours(24)); // Align with JWT expiry

                if secure_cookie {
                    cookie_builder = cookie_builder.secure(true); // Set Secure flag if served over HTTPS
                }

                let cookie = cookie_builder.finish();

                log::info!("User {} logged in successfully. Secure cookie: {}", user.email, secure_cookie);
                return Ok(HttpResponse::Ok().cookie(cookie).json(AuthResponse { success: true }));
            } else {
                log::warn!("Failed login attempt for user: {} (invalid password)", data.email);
                return Err(ApiError::new(
                    "Invalid email or password.",
                    actix_web::http::StatusCode::UNAUTHORIZED,
                ));
            }
        }
        Err(sqlx::Error::RowNotFound) => {
            log::warn!("Failed login attempt: User {} not found.", data.email);
            Err(ApiError::new(
                "Invalid email or password.",
                actix_web::http::StatusCode::UNAUTHORIZED,
            ))
        }
        Err(e) => {
            log::error!("Database error during login for user {}: {:?}", data.email, e);
            Err(ApiError::new(
                "Login failed due to a server error.",
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

#[get("/confirm/{token}")]
async fn confirm(path: web::Path<Uuid>, pool: web::Data<PgPool>) -> HttpResponse {
    match User::confirm(&pool, *path).await {
        Ok(Some(_)) => HttpResponse::Ok().finish(),
        Ok(None) => HttpResponse::BadRequest().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/request_reset")]
async fn request_reset(data: web::Json<ResetRequest>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Ok(user) = User::find_by_email(&pool, &data.email).await {
        let token = Uuid::new_v4();
        let expires = Utc::now() + chrono::Duration::hours(24);
        if User::set_reset_token(&pool, user.id, token, expires).await.is_ok() {
            let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
            let link = format!("{}/reset?token={}", base, token);
            let _ = send_email(&user.email, "Password reset", &link).await;
            return HttpResponse::Ok().finish();
        }
    }
    HttpResponse::Ok().finish()
}

#[post("/reset_password")]
async fn reset_password(data: web::Json<ResetInput>, pool: web::Data<PgPool>) -> HttpResponse {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = match Argon2::default().hash_password(data.password.as_bytes(), &salt) {
        Ok(ph) => ph.to_string(),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match User::reset_with_token(&pool, data.token, password_hash).await {
        Ok(true) => HttpResponse::Ok().finish(),
        Ok(false) => HttpResponse::BadRequest().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(register)
        .service(login)
        .service(me)
        .service(confirm)
        .service(request_reset)
        .service(reset_password);
}

#[get("/me")]
async fn me(user: AuthUser) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"user_id": user.user_id, "org_id": user.org_id, "role": user.role}))
}
