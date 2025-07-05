use actix_web::{http::header, test, web, App};
use backend::handlers;
use sqlx::{postgres::PgPoolOptions, PgPool};

mod test_utils;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use backend::models::{Document, NewDocument, NewPipeline, Pipeline};
use mini_redis::server;
use test_utils::{create_org, create_user, generate_jwt_token};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_test_app(
    s3_server: &MockServer,
) -> (
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
            .configure(handlers::init),
    )
    .await;
    (app, pool)
}

async fn start_redis() -> (oneshot::Sender<()>, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let _ = server::run(listener, async {
            let _ = rx.await;
        })
        .await;
    });
    (tx, port)
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
        .insert_header((
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        ))
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
        .insert_header((
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
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
        .insert_header((
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
    assert_eq!(put_mock.received_requests().await.len(), 1);
    assert_eq!(delete_mock.received_requests().await.len(), 1);
}

#[actix_rt::test]
async fn reject_dangerous_filename() {
    let s3_server = MockServer::start().await;
    let (_app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Sanitize Org").await;
    let user_id = create_user(&pool, org_id, "san@example.com", "org_admin").await;

    let result = Document::create(
        &pool,
        NewDocument {
            org_id,
            owner_id: user_id,
            filename: "../evil.pdf".into(),
            pages: 1,
            is_target: false,
            expires_at: None,
            display_name: "evil.pdf".into(),
        },
    )
    .await;

    assert!(result.is_err());
}

#[actix_rt::test]
async fn reject_when_upload_quota_exceeded() {
    let s3_server = MockServer::start().await;
    let put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Quota Org").await;
    let user_id = create_user(&pool, org_id, "quota@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    sqlx::query("UPDATE org_settings SET monthly_upload_quota=1 WHERE org_id=$1")
        .bind(org_id)
        .execute(&pool)
        .await
        .unwrap();

    Document::create(
        &pool,
        NewDocument {
            org_id,
            owner_id: user_id,
            filename: "first.pdf".into(),
            pages: 1,
            is_target: true,
            expires_at: None,
            display_name: "first.pdf".into(),
        },
    )
    .await
    .unwrap();

    let pdf = "%PDF-1.5\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF";
    let boundary = "BOUNDARY";
    let body = multipart_body(boundary, "exceed.pdf", "application/pdf", pdf);
    let req = test::TestRequest::post()
        .uri(&format!("/api/upload?org_id={}&is_target=true", org_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .insert_header((
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::TOO_MANY_REQUESTS
    );

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
    assert_eq!(put_mock.received_requests().await.len(), 0);
}

#[actix_rt::test]
async fn reject_when_analysis_quota_exceeded() {
    let (shutdown, port) = start_redis().await;
    std::env::set_var("REDIS_URL", format!("redis://127.0.0.1:{}/", port));

    let s3_server = MockServer::start().await;
    let put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Anal Org").await;
    let user_id = create_user(&pool, org_id, "anal@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    sqlx::query("UPDATE org_settings SET monthly_analysis_quota=0 WHERE org_id=$1")
        .bind(org_id)
        .execute(&pool)
        .await
        .unwrap();

    let stages = serde_json::json!([{"type":"ocr"}]);
    let pipeline = Pipeline::create(
        &pool,
        NewPipeline {
            org_id,
            name: "P".into(),
            stages,
        },
    )
    .await
    .unwrap();

    let pdf = "%PDF-1.5\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF";
    let boundary = "BOUNDARY";
    let body = multipart_body(boundary, "test.pdf", "application/pdf", pdf);
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/upload?org_id={}&pipeline_id={}&is_target=true",
            org_id, pipeline.id
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .insert_header((
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::TOO_MANY_REQUESTS
    );

    let docs: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(docs.0, 1);
    let jobs: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM analysis_jobs WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs.0, 0);

    assert_eq!(put_mock.received_requests().await.len(), 1);
    let _ = shutdown.send(());
}

#[actix_rt::test]
async fn cleanup_on_s3_upload_failure() {
    let s3_server = MockServer::start().await;
    let put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(500))
        .mount_as_scoped(&s3_server)
        .await;
    let delete_mock = Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Fail Org").await;
    let user_id = create_user(&pool, org_id, "fail@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let pdf = "%PDF-1.5\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF";
    let boundary = "BOUNDARY";
    let body = multipart_body(boundary, "test.pdf", "application/pdf", pdf);
    let req = test::TestRequest::post()
        .uri(&format!("/api/upload?org_id={}&is_target=true", org_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .insert_header((
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
    );

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
    assert_eq!(put_mock.received_requests().await.len(), 1);
    assert_eq!(delete_mock.received_requests().await.len(), 0);
}

#[actix_rt::test]
async fn download_returns_presigned_url() {
    let s3_server = MockServer::start().await;
    let _put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Down Org").await;
    let user_id = create_user(&pool, org_id, "down@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let doc = Document::create(
        &pool,
        NewDocument {
            org_id,
            owner_id: user_id,
            filename: "file.pdf".into(),
            pages: 1,
            is_target: false,
            expires_at: None,
            display_name: "file.pdf".into(),
        },
    )
    .await
    .unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/download/{}", doc.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("url").is_some());
}

#[actix_rt::test]
async fn download_streams_local_file() {
    let s3_server = MockServer::start().await;
    let _put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200))
        .mount_as_scoped(&s3_server)
        .await;

    let (app, pool) = setup_test_app(&s3_server).await;
    let org_id = create_org(&pool, "Local Org").await;
    let user_id = create_user(&pool, org_id, "local@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let tempdir = tempfile::tempdir().unwrap();
    std::env::set_var("LOCAL_S3_DIR", tempdir.path());
    let path = tempdir.path().join("local.pdf");
    tokio::fs::write(&path, b"%PDF-1.4\n1 0 obj<<>>endobj\nstartxref\n0\n%%EOF")
        .await
        .unwrap();

    let doc = Document::create(
        &pool,
        NewDocument {
            org_id,
            owner_id: user_id,
            filename: "local.pdf".into(),
            pages: 1,
            is_target: false,
            expires_at: None,
            display_name: "local.pdf".into(),
        },
    )
    .await
    .unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/download/{}", doc.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert!(!body.is_empty());
}
