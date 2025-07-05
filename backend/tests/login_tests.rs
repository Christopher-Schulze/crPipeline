use actix_web::{http::StatusCode, test};
use serde_json::json;

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user, create_unconfirmed_user};

#[actix_rt::test]
async fn login_success() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Login Org").await;
    create_user(&pool, org_id, "login@example.com", "user").await;

    let payload = json!({"email": "login@example.com", "password": "password"});
    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let has_cookie = resp.response().cookies().any(|c| c.name() == "token");
    assert!(has_cookie);
}

#[actix_rt::test]
async fn login_unconfirmed_user() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Login Unconfirmed Org").await;
    create_unconfirmed_user(&pool, org_id, "new@example.com", "user").await;

    let payload = json!({"email": "new@example.com", "password": "password"});
    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap_or("").contains("Account not confirmed"));
}
