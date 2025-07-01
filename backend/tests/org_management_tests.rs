// backend/tests/org_management_tests.rs

use actix_web::{test, http::header};
use uuid::Uuid;
use serde_json::json;

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user, generate_jwt_token};



#[actix_rt::test]
async fn test_get_organization_users_as_org_admin() {
    let Ok((app, pool)) = setup_test_app().await else { return; };

    let org_id = create_org(&pool, "Test Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let user_email = "user1@example.com";
    let _user_id = create_user(&pool, org_id, user_email, "user").await;

    let token = generate_jwt_token(admin_id, org_id, "org_admin");

    let req = test::TestRequest::get()
        .uri("/api/organizations/me/users")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let users: Vec<serde_json::Value> = test::read_body_json(resp).await;
    assert_eq!(users.len(), 2);
    assert!(users.iter().any(|u| u["email"] == user_email));
}

#[actix_rt::test]
async fn test_invite_user_to_organization_as_org_admin() {
    let Ok((app, pool)) = setup_test_app().await else { return; };

    let org_id = create_org(&pool, "Invite Test Org").await;
    let admin_id = create_user(&pool, org_id, "orgadmin@example.com", "org_admin").await;

    let token = generate_jwt_token(admin_id, org_id, "org_admin");

    let new_user_email = format!("invited_user_{}@example.com", Uuid::new_v4());
    let payload = json!({ "email": new_user_email });

    let req = test::TestRequest::post()
        .uri("/api/organizations/me/invite")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = $1")
        .bind(&new_user_email)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[actix_rt::test]
async fn test_get_organization_users_as_unauthorized_user() {
    let Ok((app, pool)) = setup_test_app().await else { return; };

    let org_id = create_org(&pool, "Unauthorized Org").await;
    let user_id = create_user(&pool, org_id, "user@example.com", "user").await;

    let token = generate_jwt_token(user_id, org_id, "user");

    let req = test::TestRequest::get()
        .uri("/api/organizations/me/users")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn test_deactivate_and_reactivate_user() {
    let Ok((app, pool)) = setup_test_app().await else { return; };

    let org_id = create_org(&pool, "Deactivate Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;

    let token = generate_jwt_token(admin_id, org_id, "org_admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/organizations/me/users/{}/deactivate", user_id))
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
        .uri(&format!("/api/organizations/me/users/{}/reactivate", user_id))
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

// These tests require a PostgreSQL instance pointed to by `DATABASE_URL_TEST`.
// Migrations are applied automatically during setup.

#[actix_rt::test]
async fn test_remove_user_from_organization() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Remove Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let user_id = create_user(&pool, org_id, "user_to_remove@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "org_admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/organizations/me/users/{}/remove", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let active: bool = sqlx::query_scalar("SELECT is_active FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!active);
}

#[actix_rt::test]
async fn test_remove_user_from_organization_unauthorized() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Remove Unauthorized Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(user_id, org_id, "user");

    let req = test::TestRequest::post()
        .uri(&format!("/api/organizations/me/users/{}/remove", admin_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn test_resend_confirmation_email_org_user() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Resend Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let user_id = create_user(&pool, org_id, "unconfirmed@example.com", "user").await;

    sqlx::query("UPDATE users SET confirmed=false WHERE id=$1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let token = generate_jwt_token(admin_id, org_id, "org_admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/organizations/me/users/{}/resend_confirmation", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let confirmed: bool = sqlx::query_scalar("SELECT confirmed FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!confirmed);
}

#[actix_rt::test]
async fn test_resend_confirmation_email_org_user_unauthorized() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Resend Unauthorized Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let user_id = create_user(&pool, org_id, "unconfirmed@example.com", "user").await;

    sqlx::query("UPDATE users SET confirmed=false WHERE id=$1")
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();

    let token = generate_jwt_token(user_id, org_id, "user");
    let req = test::TestRequest::post()
        .uri(&format!("/api/organizations/me/users/{}/resend_confirmation", admin_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
}
