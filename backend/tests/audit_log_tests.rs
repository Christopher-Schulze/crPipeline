use actix_web::{test, web, App, http::header};
use backend::handlers;
mod test_utils;
use test_utils::{create_org, create_user, generate_jwt_token};
use sqlx::{PgPool, postgres::PgPoolOptions};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::method;

async fn setup_app(s3: &MockServer) -> (impl actix_web::dev::Service<actix_http::Request, Response = actix_web::dev::ServiceResponse, Error = actix_web::Error>, PgPool) {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL_TEST").unwrap_or_else(|_| std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"));
    let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await.expect("db");
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    std::env::set_var("AWS_ACCESS_KEY_ID", "test");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
    let shared = aws_config::from_env().region(aws_config::meta::region::RegionProviderChain::default_provider().or_else("us-east-1")).load().await;
    let cfg = aws_sdk_s3::config::Builder::from(&shared).endpoint_url(s3.uri()).force_path_style(true).build();
    let s3_client = aws_sdk_s3::Client::from_conf(cfg);

    let app = test::init_service(App::new().app_data(web::Data::new(pool.clone())).app_data(web::Data::new(s3_client)).configure(handlers::init)).await;
    (app, pool)
}

#[actix_rt::test]
async fn upload_creates_audit_log() {
    let s3_server = MockServer::start().await;
    let _mock_guard = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_app(&s3_server).await;
    let org_id = create_org(&pool, "Audit Org").await;
    let user_id = create_user(&pool, org_id, "audit@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let pdf = "%PDF-1.4\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF";
    let boundary = "BOUNDARY";
    let body = format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"t.pdf\"\r\nContent-Type: application/pdf\r\n\r\n{pdf}\r\n--{b}--\r\n", b=boundary, pdf=pdf);

    let req = test::TestRequest::post()
        .uri(&format!("/api/upload?org_id={}&is_target=true", org_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .insert_header((header::CONTENT_TYPE, format!("multipart/form-data; boundary={}", boundary)))
        .set_payload(body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(count.0 > 0);
}
