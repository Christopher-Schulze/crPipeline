use actix_web::{test, web, App, http::header};
use backend::handlers;
use sqlx::{PgPool, postgres::PgPoolOptions};

mod test_utils;
use test_utils::{create_org, create_user, generate_jwt_token};
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::method;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;

async fn setup_test_app(s3_server: &MockServer) -> (impl actix_web::dev::Service<actix_http::Request, Response=actix_web::dev::ServiceResponse, Error=actix_web::Error>, PgPool) {
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

    std::env::set_var("AWS_ACCESS_KEY_ID", "test");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
        .endpoint_url(s3_server.uri())
        .force_path_style(true)
        .build();
    let s3_client = S3Client::from_conf(s3_config);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(s3_client.clone()))
            .configure(handlers::init)
    ).await;
    (app, pool)
}


fn multipart_body(boundary: &str, filename: &str, content_type: &str, content: &str) -> String {
    format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n{content}\r\n--{boundary}--\r\n",
        boundary = boundary,
        filename = filename,
        content_type = content_type,
        content = content
    )
}

#[actix_rt::test]
async fn test_pdf_upload_success() {
    let s3_server = MockServer::start().await;
    let put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Doc Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let pdf = "%PDF-1.5\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF";
    let boundary = "BOUNDARY";
    let body = multipart_body(boundary, "test.pdf", "application/pdf", pdf);
    let req = test::TestRequest::post()
        .uri(&format!("/api/upload?org_id={}&is_target=true", org_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .insert_header((header::CONTENT_TYPE, format!("multipart/form-data; boundary={}", boundary)))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
    assert_eq!(put_mock.received_requests().await.len(), 1);
}

#[actix_rt::test]
async fn test_pdf_upload_bad_content_type() {
    let s3_server = MockServer::start().await;
    let put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Doc Org2").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let pdf = "%PDF-1.5\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF";
    let boundary = "BOUNDARY";
    let body = multipart_body(boundary, "test.pdf", "text/plain", pdf);
    let req = test::TestRequest::post()
        .uri(&format!("/api/upload?org_id={}&is_target=true", org_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .insert_header((header::CONTENT_TYPE, format!("multipart/form-data; boundary={}", boundary)))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
    assert_eq!(put_mock.received_requests().await.len(), 0);
}

#[actix_rt::test]
async fn test_cleanup_on_failed_upload() {
    let s3_server = MockServer::start().await;
    let put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&s3_server)
        .await;
    let delete_mock = Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Doc Org3").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    // remove user to trigger FK violation when inserting document
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let pdf = "%PDF-1.5\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF";
    let boundary = "BOUNDARY";
    let body = multipart_body(boundary, "test.pdf", "application/pdf", pdf);
    let req = test::TestRequest::post()
        .uri(&format!("/api/upload?org_id={}&is_target=true", org_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .insert_header((header::CONTENT_TYPE, format!("multipart/form-data; boundary={}", boundary)))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(put_mock.received_requests().await.len(), 1);
    assert_eq!(delete_mock.received_requests().await.len(), 1);
}
