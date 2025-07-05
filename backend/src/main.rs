use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use actix_web_prom::PrometheusMetricsBuilder;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use sqlx::postgres::PgPoolOptions;

use backend::config::AppConfig;

use backend::handlers;
use backend::middleware::{
    jwt::init_jwt_secret,
    rate_limit::RateLimit,
    csrf_check::{CsrfCheck, init_csrf_token},
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = match AppConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    std::env::set_var("JWT_SECRET", &config.jwt_secret);
    init_jwt_secret();
    init_csrf_token();
    tracing_subscriber::fmt::init();

    let database_url = config.database_url;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let s3_client = S3Client::new(&shared_config);

    let prometheus = PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics")
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("metrics init: {e}")))?;
    let allowed_origin = config.frontend_origin.clone();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .wrap(RateLimit)
            .wrap(CsrfCheck)
            .wrap(prometheus.clone())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(s3_client.clone()))
            .configure(handlers::init)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
