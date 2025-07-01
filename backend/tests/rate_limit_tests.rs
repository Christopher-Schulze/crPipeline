use actix_web::{test, App};
use backend::handlers;
use backend::middleware::rate_limit::RateLimit;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use tokio::time::{sleep, Duration};

const MAX_REQUESTS: usize = 100; // must match middleware constant

async fn start_redis() -> (Child, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let child = Command::new("redis-server")
        .arg("--port")
        .arg(port.to_string())
        .arg("--save")
        .arg("")
        .arg("--appendonly")
        .arg("no")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start redis-server");
    // give redis a moment to start
    sleep(Duration::from_millis(300)).await;
    (child, port)
}

#[actix_rt::test]
async fn redis_enforces_rate_limit() {
    let (mut redis_proc, port) = start_redis().await;
    std::env::set_var("REDIS_URL", format!("redis://127.0.0.1:{}/", port));
    std::env::remove_var("REDIS_RATE_LIMIT_FALLBACK");

    let app = test::init_service(
        App::new()
            .wrap(RateLimit)
            .configure(handlers::health::routes),
    )
    .await;

    for _ in 0..MAX_REQUESTS {
        let req = test::TestRequest::get()
            .uri("/health")
            .insert_header(("X-API-Key", "key1"))
            .to_request();
        let resp = test::try_call_service(&app, req).await.unwrap();
        assert!(resp.status().is_success());
    }
    let req = test::TestRequest::get()
        .uri("/health")
        .insert_header(("X-API-Key", "key1"))
        .to_request();
    let resp = test::try_call_service(&app, req).await;
    let err = resp.expect_err("expected rate limit error");
    assert_eq!(
        err.as_response_error().status_code(),
        actix_web::http::StatusCode::TOO_MANY_REQUESTS
    );

    let _ = redis_proc.kill();
    let _ = redis_proc.wait();
}

#[actix_rt::test]
async fn memory_fallback_enforces_limit() {
    std::env::remove_var("REDIS_URL");
    std::env::set_var("REDIS_RATE_LIMIT_FALLBACK", "memory");

    let app = test::init_service(
        App::new()
            .wrap(RateLimit)
            .configure(handlers::health::routes),
    )
    .await;

    for _ in 0..MAX_REQUESTS {
        let req = test::TestRequest::get()
            .uri("/health")
            .insert_header(("X-API-Key", "memkey"))
            .to_request();
        let resp = test::try_call_service(&app, req).await.unwrap();
        assert!(resp.status().is_success());
    }
    let req = test::TestRequest::get()
        .uri("/health")
        .insert_header(("X-API-Key", "memkey"))
        .to_request();
    let resp = test::try_call_service(&app, req).await;
    let err = resp.expect_err("expected rate limit error");
    assert_eq!(
        err.as_response_error().status_code(),
        actix_web::http::StatusCode::TOO_MANY_REQUESTS
    );
}
