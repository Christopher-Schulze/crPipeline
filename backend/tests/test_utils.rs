use actix_web::{test, web, App};
use backend::handlers;
use backend::middleware::jwt::create_jwt;
use sqlx::{PgPool, postgres::PgPoolOptions};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use uuid::Uuid;

pub async fn setup_test_app() -> (impl actix_web::dev::Service<actix_http::Request, Response = actix_web::dev::ServiceResponse, Error = actix_web::Error>, PgPool) {
    if let Err(_) = std::env::var("DATABASE_URL_TEST") {
        todo!("skip");
    }
    dotenvy::from_filename(".env.test").ok();
    let database_url = std::env::var("DATABASE_URL_TEST").expect("DATABASE_URL_TEST must be set");
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
            .configure(handlers::init)
    ).await;
    (app, pool)
}

pub fn generate_jwt_token(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    std::env::set_var("JWT_SECRET", "testsecret");
    create_jwt(user_id, org_id, role)
}

pub async fn create_org(pool: &PgPool, name: &str) -> Uuid {
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

pub async fn create_user(pool: &PgPool, org_id: Uuid, email: &str, role: &str) -> Uuid {
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
