use actix_web::{test, web, App, http::StatusCode};
use backend::handlers;
use sqlx::postgres::PgPoolOptions;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::method;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;

#[actix_rt::test]
async fn readiness_unavailable_without_db() {
    let s3_server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&s3_server)
        .await;

    std::env::set_var("AWS_ACCESS_KEY_ID", "test");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
        .endpoint_url(s3_server.uri())
        .force_path_style(true)
        .build();
    let s3_client = S3Client::from_conf(s3_config);

    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://user@localhost/db")
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .app_data(web::Data::new(s3_client))
            .configure(handlers::health::routes)
    ).await;

    let req = test::TestRequest::get().uri("/readiness").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
