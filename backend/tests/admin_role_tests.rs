use actix_web::{test, http::header};
use uuid::Uuid;
use serde_json::json;

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user, generate_jwt_token};

#[actix_rt::test]
async fn test_global_admin_assign_role_success() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Role Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let payload = json!({"role": "user"});
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
    assert_eq!(role, "user");
}

#[actix_rt::test]
async fn test_non_admin_cannot_assign_role() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "NonAdmin Org").await;
    let user_id = create_user(&pool, org_id, "user@example.com", "user").await;
    let other_user_id = create_user(&pool, org_id, "other@example.com", "user").await;
    let token = generate_jwt_token(user_id, org_id, "user");
    let payload = json!({"role": "user"});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", other_user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn test_assign_org_admin_with_valid_org() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Main Org").await;
    let target_org_id = create_org(&pool, "Target Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let payload = json!({"role": "org_admin", "org_id": target_org_id});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let row: (String, Uuid) = sqlx::query_as("SELECT role, org_id FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, "org_admin");
    assert_eq!(row.1, target_org_id);
}

#[actix_rt::test]
async fn test_assign_org_admin_with_invalid_org() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Invalid Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let payload = json!({"role": "org_admin", "org_id": Uuid::new_v4()});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

// These tests require a PostgreSQL instance pointed to by `DATABASE_URL_TEST`.
// Migrations are applied automatically during setup.

