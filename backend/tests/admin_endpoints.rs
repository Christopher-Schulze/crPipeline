use actix_web::{http::header, test};
use serde_json::json;
use uuid::Uuid;

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user, generate_jwt_token};

#[actix_rt::test]
async fn change_user_role_success() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Role Endpoint Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let payload = json!({"role": "org_admin", "org_id": org_id});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(role, "org_admin");
}

#[actix_rt::test]
async fn change_user_role_unauthorized() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Role Unauthorized Org").await;
    let user_id = create_user(&pool, org_id, "user@example.com", "user").await;
    let other_id = create_user(&pool, org_id, "other@example.com", "user").await;
    let token = generate_jwt_token(user_id, org_id, "user");
    let payload = json!({"role": "org_admin", "org_id": org_id});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", other_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn deactivate_reactivate_user_success() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Deactivate Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/deactivate", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token.clone())))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let active: bool = sqlx::query_scalar("SELECT is_active FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!active);

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/reactivate", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let active: bool = sqlx::query_scalar("SELECT is_active FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(active);
}

#[actix_rt::test]
async fn deactivate_user_unauthorized() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Deactivate Unauthorized").await;
    let user_id = create_user(&pool, org_id, "user@example.com", "user").await;
    let target_id = create_user(&pool, org_id, "target@example.com", "user").await;
    let token = generate_jwt_token(user_id, org_id, "user");

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/deactivate", target_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn invite_user_success() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Invite Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");

    let email = format!("invite_{}@example.com", Uuid::new_v4());
    let payload = json!({"email": email, "org_id": org_id});
    let req = test::TestRequest::post()
        .uri("/api/admin/invite")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email=$1")
        .bind(&email)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[actix_rt::test]
async fn invite_user_invalid_email() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Invite Bad Org").await;
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
