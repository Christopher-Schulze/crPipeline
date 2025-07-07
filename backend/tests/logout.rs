use actix_web::cookie::time::Duration;
use actix_web::{http::StatusCode, test, web, App};
use backend::handlers;

#[actix_rt::test]
async fn logout_clears_cookie() {
    let app = test::init_service(
        App::new().service(web::scope("/api").service(backend::handlers::auth::logout)),
    )
    .await;

    let req = test::TestRequest::post().uri("/api/logout").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let cookie = resp
        .response()
        .cookies()
        .find(|c| c.name() == "token")
        .expect("token cookie");
    assert_eq!(cookie.value(), "");
    assert_eq!(cookie.max_age(), Some(Duration::ZERO));
}
