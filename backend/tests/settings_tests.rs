use actix_web::{http::header, test};
use serde_json::json;

mod test_utils;
use test_utils::{create_org, create_user, generate_jwt_token, setup_test_app};

#[actix_rt::test]
async fn test_get_settings_success() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Settings Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let req = test::TestRequest::get()
        .uri(&format!("/api/settings/{}", org_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["org_id"], org_id.to_string());
}

#[actix_rt::test]
async fn test_update_settings_success() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Update Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "monthly_upload_quota": 150,
        "monthly_analysis_quota": 200,
        "accent_color": "#FF00FF"
    });
    let req = test::TestRequest::post()
        .uri("/api/settings")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let updated: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(updated["monthly_upload_quota"], 150);
    let row: (i32,) =
        sqlx::query_as("SELECT monthly_upload_quota FROM org_settings WHERE org_id=$1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, 150);
}

#[actix_rt::test]
async fn test_update_settings_invalid_quota() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Bad Quota Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "monthly_upload_quota": -1,
        "monthly_analysis_quota": 10,
        "accent_color": "#123456"
    });
    let req = test::TestRequest::post()
        .uri("/api/settings")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_update_settings_invalid_ai_endpoint() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Bad AI Endpoint Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "ai_api_endpoint": "not-a-valid-url",
        "monthly_upload_quota": 10,
        "monthly_analysis_quota": 10,
        "accent_color": "#123456"
    });
    let req = test::TestRequest::post()
        .uri("/api/settings")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_update_settings_invalid_ocr_endpoint() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Bad OCR Endpoint Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "ocr_api_endpoint": "http:/missing-slashes",
        "monthly_upload_quota": 10,
        "monthly_analysis_quota": 10,
        "accent_color": "#123456"
    });
    let req = test::TestRequest::post()
        .uri("/api/settings")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}
