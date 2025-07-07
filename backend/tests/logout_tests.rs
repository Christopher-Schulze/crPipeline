use actix_web::{http::StatusCode, test};

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user};

#[actix_rt::test]
async fn logout_clears_cookie() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Logout Org").await;
    create_user(&pool, org_id, "logout@example.com", "user").await;

    let payload = serde_json::json!({"email": "logout@example.com", "password": "password"});
    let login_req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(&payload)
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    assert_eq!(login_resp.status(), StatusCode::OK);
    assert!(login_resp.response().cookies().any(|c| c.name() == "token"));

    let req = test::TestRequest::post()
        .uri("/api/logout")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let token_cookie = resp.response().cookies().find(|c| c.name() == "token");
    assert!(token_cookie.is_some());
    let cookie = token_cookie.unwrap();
    assert!(cookie.value().is_empty());
    assert_eq!(cookie.max_age(), Some(actix_web::cookie::time::Duration::ZERO));
}
