use actix_web::{test, web, App};
use backend::handlers;
use backend::middleware::rate_limit::RateLimit;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use mini_redis::server;

mod test_utils;
use test_utils::setup_test_app;

const MAX_REQUESTS: usize = 100; // must match middleware constant

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

#[actix_rt::test]
async fn redis_enforces_rate_limit() {
    let (shutdown, port) = start_redis().await;
    std::env::set_var("REDIS_URL", format!("redis://127.0.0.1:{}/", port));
    std::env::remove_var("REDIS_RATE_LIMIT_FALLBACK");

    let Ok((_, pool)) = setup_test_app().await else { return; };
    let app = test::init_service(
        App::new()
            .wrap(RateLimit)
            .app_data(web::Data::new(pool))
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

    let _ = shutdown.send(());
}

#[actix_rt::test]
async fn memory_fallback_enforces_limit() {
    std::env::remove_var("REDIS_URL");
    std::env::set_var("REDIS_RATE_LIMIT_FALLBACK", "memory");

    let Ok((_, pool)) = setup_test_app().await else { return; };
    let app = test::init_service(
        App::new()
            .wrap(RateLimit)
            .app_data(web::Data::new(pool))
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
