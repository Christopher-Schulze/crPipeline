use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_cors::Cors;
use dotenvy::dotenv;
use std::env;
use sqlx::postgres::PgPoolOptions;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;


use backend::handlers;
use backend::middleware::{jwt::init_jwt_secret, rate_limit::RateLimit};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    init_jwt_secret();
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

    HttpServer::new(move || {
        let allowed_origin = env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "*".into());
        let cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .wrap(RateLimit)
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(s3_client.clone()))
            .configure(handlers::init)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
