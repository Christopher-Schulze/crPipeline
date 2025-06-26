use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_cors::Cors;
use dotenvy::dotenv;
use std::env;
use sqlx::postgres::PgPoolOptions;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use actix_csrf::CsrfMiddleware; // Added for CSRF
use actix_web::cookie::Key;    // For CSRF secret key
use base64::{engine::general_purpose::STANDARD as base64_standard, Engine as _}; // For decoding

use backend::handlers;
use backend::middleware::rate_limit::RateLimit;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init(); // Or your existing logger

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let s3_client = S3Client::new(&shared_config);

    // Load CSRF Secret Key
    let csrf_secret_key_b64 = env::var("CSRF_SECRET_KEY_B64")
        .expect("CSRF_SECRET_KEY_B64 must be set in .env for CSRF protection. Generate with 'openssl rand -base64 32'.");

    let csrf_key_bytes = base64_standard.decode(&csrf_secret_key_b64)
        .expect("CSRF_SECRET_KEY_B64 is not valid base64.");
    if csrf_key_bytes.len() != 32 {
        panic!("CSRF_SECRET_KEY_B64 must decode to a 32-byte key. Current length: {} bytes", csrf_key_bytes.len());
    }
    let csrf_protection_key = Key::from(&csrf_key_bytes);


    HttpServer::new(move || {
        let allowed_origin = env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "*".into());
        let cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        let csrf_middleware = CsrfMiddleware::new(csrf_protection_key.clone());

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .wrap(RateLimit)
            .wrap(csrf_middleware) // Add CSRF middleware
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(s3_client.clone()))
            .configure(handlers::init)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
