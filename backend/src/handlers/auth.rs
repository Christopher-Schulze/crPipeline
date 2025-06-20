use actix_web::{post, get, web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use crate::models::{User, NewUser};
use crate::utils::log_action;
use crate::middleware::jwt::create_jwt;
use crate::middleware::auth::AuthUser;
use crate::email::send_email;
use argon2::{Argon2, PasswordHasher};
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
pub struct ResetRequest { pub email: String }

#[derive(Deserialize)]
pub struct ResetInput { pub token: Uuid, pub password: String }

#[post("/register")]
pub async fn register(data: web::Json<RegisterInput>, pool: web::Data<PgPool>) -> HttpResponse {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::default()
        .hash_password(data.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    let role = data.role.clone().unwrap_or_else(|| "user".to_string());
    let user = NewUser { org_id: data.org_id, email: data.email.clone(), password_hash, role };
    match User::create(&pool, user).await {
        Ok(u) => {
            log_action(&pool, u.org_id, u.id, "register").await;
            let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
            let link = format!("{}/api/confirm/{}", base, u.confirmation_token.unwrap());
            let _ = send_email(&u.email, "Confirm your account", &link).await;
            let token = create_jwt(u.id, u.org_id, &u.role);
            let cookie = actix_web::cookie::Cookie::build("token", token)
                .path("/")
                .http_only(true)
                .finish();
            HttpResponse::Ok().cookie(cookie).json(AuthResponse { success: true })
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[derive(Deserialize)]
pub struct LoginInput { pub email: String, pub password: String }

#[post("/login")]
pub async fn login(data: web::Json<LoginInput>, pool: web::Data<PgPool>) -> HttpResponse {
    if let Ok(user) = User::find_by_email(&pool, &data.email).await {
        if user.verify_password(&data.password) {
            let token = create_jwt(user.id, user.org_id, &user.role);
            let cookie = actix_web::cookie::Cookie::build("token", token)
                .path("/")
                .http_only(true)
                .finish();
            log_action(&pool, user.org_id, user.id, "login").await;
            return HttpResponse::Ok().cookie(cookie).json(AuthResponse { success: true });
        }
    }
    HttpResponse::Unauthorized().finish()
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
    let password_hash = Argon2::default()
        .hash_password(data.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
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
