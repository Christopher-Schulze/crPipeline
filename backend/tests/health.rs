use actix_web::{test, App};
use backend::handlers;

#[actix_rt::test]
async fn health_ok() {
    let app = test::init_service(App::new().configure(handlers::health::routes)).await;
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}
