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
    sqlx::migrate!(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"))
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
async fn test_get_settings_success() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Settings Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let req = test::TestRequest::get()
        .uri(&format!("/api/settings/{}", org_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["org_id"], org_id.to_string());
}

#[actix_rt::test]
async fn test_update_settings_success() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Update Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "monthly_upload_quota": 150,
        "monthly_analysis_quota": 200,
        "accent_color": "#FF00FF"
    });
    let req = test::TestRequest::post()
        .uri("/api/settings")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let updated: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(updated["monthly_upload_quota"], 150);
    let row: (i32,) =
        sqlx::query_as("SELECT monthly_upload_quota FROM org_settings WHERE org_id=$1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, 150);
}

#[actix_rt::test]
async fn test_update_settings_invalid_quota() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Bad Quota Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "monthly_upload_quota": -1,
        "monthly_analysis_quota": 10,
        "accent_color": "#123456"
    });
    let req = test::TestRequest::post()
        .uri("/api/settings")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}
