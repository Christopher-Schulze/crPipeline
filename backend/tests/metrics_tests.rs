use actix_web::{test, App};
use actix_web_prom::PrometheusMetricsBuilder;
use backend::handlers;

#[actix_rt::test]
async fn metrics_endpoint_returns_ok() {
    let prometheus = PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics")
        .build()
        .unwrap();
    let app = test::init_service(
        App::new()
            .wrap(prometheus)
            .configure(handlers::health::routes)
    ).await;

    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("jobs_total") || body_str.contains("api_http_requests_total"));
}
