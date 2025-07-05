use actix_rt::time::sleep;
use backend::config::AppConfig;
use backend::email::{enqueue_email, start_email_worker};
use std::time::Duration;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[actix_rt::test]
async fn queued_email_is_sent() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    std::env::set_var("EMAIL_HTTP_ENDPOINT", server.uri());

    let cfg = AppConfig {
        database_url: "postgres://".into(),
        jwt_secret: "0123456789abcdef0123456789abcdef".into(),
        s3_bucket: "uploads".into(),
        frontend_origin: "*".into(),
        email_queue_provider: "memory".into(),
        email_queue_size: 2,
    };
    start_email_worker(&cfg);
    enqueue_email("u@example.com", "subj", "body")
        .await
        .unwrap();
    sleep(Duration::from_millis(100)).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
