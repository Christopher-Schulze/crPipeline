use actix_web::{test, App};
use backend::handlers;
use backend::middleware::csrf_check::CsrfCheck;

#[actix_rt::test]
async fn rejects_missing_token() {
    std::env::set_var("CSRF_TOKEN", "secret");
    let app = test::init_service(
        App::new()
            .wrap(CsrfCheck)
            .configure(handlers::health::routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::try_call_service(&app, req).await;
    let err = resp.expect_err("expected csrf error");
    assert_eq!(err.as_response_error().status_code(), actix_web::http::StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn rejects_invalid_token() {
    std::env::set_var("CSRF_TOKEN", "secret");
    let app = test::init_service(
        App::new()
            .wrap(CsrfCheck)
            .configure(handlers::health::routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/health")
        .insert_header(("X-CSRF-Token", "wrong"))
        .to_request();
    let resp = test::try_call_service(&app, req).await;
    let err = resp.expect_err("expected csrf error");
    assert_eq!(err.as_response_error().status_code(), actix_web::http::StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn accepts_valid_token() {
    std::env::set_var("CSRF_TOKEN", "secret");
    let app = test::init_service(
        App::new()
            .wrap(CsrfCheck)
            .configure(handlers::health::routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/health")
        .insert_header(("X-CSRF-Token", "secret"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}
