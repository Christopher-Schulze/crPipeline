use actix_web::{http::header, test};
use serde_json::json;
use tracing_test::traced_test;
use uuid::Uuid;

mod test_utils;
use test_utils::{create_org, create_user, generate_jwt_token, setup_test_app};

#[actix_rt::test]
async fn test_assign_org_admin_missing_org_id() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Assign Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let payload = json!({"role": "org_admin"});
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/assign_role", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn test_admin_deactivate_and_reactivate_user() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
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
#[traced_test]
async fn test_admin_update_user_profile_email_change_logs() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Profile Org").await;
    let admin_id = create_user(&pool, org_id, "admin@example.com", "admin").await;
    let user_id = create_user(&pool, org_id, "member@example.com", "user").await;
    let token = generate_jwt_token(admin_id, org_id, "admin");
    let new_email = format!("new_{}@example.com", Uuid::new_v4());
    let payload = json!({"email": new_email});
    let req = test::TestRequest::put()
        .uri(&format!("/api/admin/users/{}/profile", user_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let row: (String, bool) = sqlx::query_as("SELECT email, confirmed FROM users WHERE id=$1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, new_email);
    assert!(!row.1);
    assert!(logs_contain("email body"));
}
