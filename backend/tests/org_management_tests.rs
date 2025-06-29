#![cfg(feature = "integration-tests")]
// backend/tests/org_management_tests.rs

use actix_web::{test, web, App, http::header};
use backend::handlers;
use backend::middleware::jwt::create_jwt;
use sqlx::{PgPool, postgres::PgPoolOptions};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use uuid::Uuid;
use serde_json::json;

// Placeholder for a function that would set up the application with a test DB pool
// and other necessary services, similar to main.rs.
// This would ideally also handle migrations.
use actix_web::dev::{ServiceRequest, ServiceResponse, Service};

async fn setup_test_app() -> (impl Service<ServiceRequest, Response = ServiceResponse, Error = actix_web::Error>, PgPool) {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL_TEST")
        .unwrap_or_else(|_| std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"));

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations on test DB");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            // .app_data(web::Data::new(s3_client.clone())) // Add other app_data if needed by these handlers
            .configure(handlers::init) // This should bring in your org routes
    ).await;
    (app, pool)
}

fn generate_jwt_token(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    std::env::set_var("JWT_SECRET", "testsecret");
    create_jwt(user_id, org_id, role)
}

async fn create_org(pool: &PgPool, name: &str) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name, api_key) VALUES ($1, $2, uuid_generate_v4())")
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
async fn test_get_organization_users_as_org_admin() {
    let (app, pool) = setup_test_app().await;

    let org_id = create_org(&pool, "Test Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let user_email = "user1@example.com";
    let _user_id = create_user(&pool, org_id, user_email, "user").await;

    let token = generate_jwt_token(admin_id, org_id, "org_admin");

    let req = test::TestRequest::get()
        .uri("/api/organizations/me/users")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let users: Vec<serde_json::Value> = test::read_body_json(resp).await;
    assert_eq!(users.len(), 2);
    assert!(users.iter().any(|u| u["email"] == user_email));
}

#[actix_rt::test]
async fn test_invite_user_to_organization_as_org_admin() {
    let (app, pool) = setup_test_app().await;

    let org_id = create_org(&pool, "Invite Test Org").await;
    let admin_id = create_user(&pool, org_id, "orgadmin@example.com", "org_admin").await;

    let token = generate_jwt_token(admin_id, org_id, "org_admin");

    let new_user_email = format!("invited_user_{}@example.com", Uuid::new_v4());
    let payload = json!({ "email": new_user_email });

    let req = test::TestRequest::post()
        .uri("/api/organizations/me/invite")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = $1")
        .bind(&new_user_email)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[actix_rt::test]
async fn test_get_organization_users_as_unauthorized_user() {
    let (app, pool) = setup_test_app().await;

    let org_id = create_org(&pool, "Unauthorized Org").await;
    let user_id = create_user(&pool, org_id, "user@example.com", "user").await;

    let token = generate_jwt_token(user_id, org_id, "user");

    let req = test::TestRequest::get()
        .uri("/api/organizations/me/users")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn test_deactivate_and_reactivate_user() {
    let (app, pool) = setup_test_app().await;

    let org_id = create_org(&pool, "Deactivate Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;

    let token = generate_jwt_token(admin_id, org_id, "org_admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/organizations/me/users/{}/deactivate", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token.clone())))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let active: bool = sqlx::query_scalar("SELECT is_active FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!active);

    let req = test::TestRequest::post()
        .uri(&format!("/api/organizations/me/users/{}/reactivate", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let active: bool = sqlx::query_scalar("SELECT is_active FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(active);
}

// These tests require a PostgreSQL instance pointed to by `DATABASE_URL_TEST`.
// Migrations are applied automatically during setup.
