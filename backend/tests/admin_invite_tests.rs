use actix_web::{http::header, test};
use serde_json::json;
use uuid::Uuid;

mod test_utils;
use test_utils::{create_org, create_user, generate_jwt_token, setup_test_app};

#[actix_rt::test]
async fn test_admin_invite_success() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Admin Invite Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");

    let invite_email = format!("new_admin_user_{}@example.com", Uuid::new_v4());
    let payload = json!({"email": invite_email, "org_id": org_id});
    let req = test::TestRequest::post()
        .uri("/api/admin/invite")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email=$1")
        .bind(&invite_email)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[actix_rt::test]
async fn test_admin_invite_invalid_email() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Bad Email Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");

    let payload = json!({"email": "not-an-email", "org_id": org_id});
    let req = test::TestRequest::post()
        .uri("/api/admin/invite")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_admin_invite_unauthorized() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Unauthorized Admin Invite Org").await;
    let user_id = create_user(&pool, org_id, "user@example.com", "user").await;
    let token = generate_jwt_token(user_id, org_id, "user");

    let payload = json!({"email": "someone@example.com", "org_id": org_id});
    let req = test::TestRequest::post()
        .uri("/api/admin/invite")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_admin_invite_duplicate_email() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Duplicate Invite Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let existing_email = "dup@example.com";
    let _existing_id = create_user(&pool, org_id, existing_email, "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");

    let payload = json!({"email": existing_email, "org_id": org_id});
    let req = test::TestRequest::post()
        .uri("/api/admin/invite")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::CONFLICT);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_admin_invite_email_failure() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    std::env::set_var("MOCK_EMAIL_FAIL", "1");
    let org_id = create_org(&pool, "Email Fail Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let invite_email = format!("fail_email_user_{}@example.com", Uuid::new_v4());
    let payload = json!({"email": invite_email, "org_id": org_id});
    let req = test::TestRequest::post()
        .uri("/api/admin/invite")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::ACCEPTED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.get("success"), Some(&serde_json::Value::Bool(true)));
    assert!(body.get("message").is_some());
    std::env::remove_var("MOCK_EMAIL_FAIL");
}
