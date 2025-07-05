use tracing::info;
use tracing_test::traced_test;
use backend::config::AppConfig;
use backend::middleware::jwt::init_jwt_secret;

#[tokio::test]
#[traced_test]
async fn secrets_are_not_logged() {
    std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost/db");
    std::env::set_var("JWT_SECRET", "very_secret_value_1234567890very_secret_value");
    std::env::set_var("S3_BUCKET", "uploads");
    std::env::set_var("FRONTEND_ORIGIN", "*");

    let _ = AppConfig::from_env().unwrap();
    init_jwt_secret();

    info!("startup complete");

    assert!(!logs_contain("very_secret_value_1234567890very_secret_value"));
}
