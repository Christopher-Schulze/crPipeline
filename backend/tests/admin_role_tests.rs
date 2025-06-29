use actix_web::{http::header, test, web, App};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use backend::handlers;
use backend::middleware::jwt::create_jwt;
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

async fn setup_test_app() -> (
    impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
    PgPool,
) {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL_TEST")
        .unwrap_or_else(|_| std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"));
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");
    sqlx::migrate!("migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations on test DB");
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .configure(handlers::init),
    )
    .await;
    (app, pool)
}

fn generate_jwt_token(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    std::env::set_var("JWT_SECRET", "testsecret");
    create_jwt(user_id, org_id, role)
}

async fn create_org(pool: &PgPool, name: &str) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO organizations (id, name, api_key) VALUES ($1, $2, uuid_generate_v4())",
    )
    .bind(org_id)
    .bind(name)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO org_settings (org_id) VALUES ($1)")
        .bind(org_id)
        .execute(pool)
        .await
        .unwrap();
    org_id
}

async fn create_user(pool: &PgPool, org_id: Uuid, email: &str, role: &str) -> Uuid {
    let user_id = Uuid::new_v4();
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::default()
        .hash_password(b"password", &salt)
        .unwrap()
        .to_string();
    sqlx::query("INSERT INTO users (id, org_id, email, password_hash, role, confirmed) VALUES ($1,$2,$3,$4,$5,true)")
        .bind(user_id)
        .bind(org_id)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
    user_id
}

#[actix_rt::test]
async fn test_global_admin_assign_role_success() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Role Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let payload = json!({"role": "user"});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(role, "user");
}

#[actix_rt::test]
async fn test_non_admin_cannot_assign_role() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "NonAdmin Org").await;
    let user_id = create_user(&pool, org_id, "user@example.com", "user").await;
    let other_user_id = create_user(&pool, org_id, "other@example.com", "user").await;
    let token = generate_jwt_token(user_id, org_id, "user");
    let payload = json!({"role": "user"});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", other_user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn test_assign_org_admin_with_valid_org() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Main Org").await;
    let target_org_id = create_org(&pool, "Target Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let payload = json!({"role": "org_admin", "org_id": target_org_id});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let row: (String, Uuid) = sqlx::query_as("SELECT role, org_id FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, "org_admin");
    assert_eq!(row.1, target_org_id);
}

#[actix_rt::test]
async fn test_assign_org_admin_with_invalid_org() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Invalid Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let payload = json!({"role": "org_admin", "org_id": Uuid::new_v4()});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

// These tests require a PostgreSQL instance pointed to by `DATABASE_URL_TEST`.
// Migrations are applied automatically during setup.
