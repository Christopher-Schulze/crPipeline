// backend/tests/pipeline_tests.rs
use actix_web::{test, http::header};
use backend::handlers;
use uuid::Uuid;
use serde_json::json;

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user, generate_jwt_token};

#[actix_rt::test]
async fn test_create_pipeline_success() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Pipeline Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": "My Pipeline",
        "stages": [{"type": "ocr", "command": "echo hi"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let created: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(created["name"], "My Pipeline");
}

#[actix_rt::test]
async fn test_reject_missing_stage_type() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Bad Pipeline Org").await;
    let user_id = create_user(&pool, org_id, "admin2@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": "Bad Pipeline",
        "stages": [{"command": "echo"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn test_reject_empty_pipeline_name() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Empty Name Org").await;
    let user_id = create_user(&pool, org_id, "admin3@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": " ",
        "stages": [{"type": "ocr"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn test_update_pipeline_success_as_org_admin() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Update Org").await;
    let user_id = create_user(&pool, org_id, "upd@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "Orig",
        "stages": [{"type": "ocr"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let created: serde_json::Value = test::read_body_json(resp).await;
    let pipeline_id = created["id"].as_str().unwrap();

    let update_payload = json!({
        "org_id": org_id,
        "name": "Updated",
        "stages": [{"type": "ocr"}]
    });
    let req = test::TestRequest::put()
        .uri(&format!("/api/pipelines/{}", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&update_payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let updated: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(updated["name"], "Updated");
}

#[actix_rt::test]
async fn test_update_pipeline_other_org_unauthorized() {
    let (app, pool) = setup_test_app().await;
    let org1 = create_org(&pool, "Org1").await;
    let org2 = create_org(&pool, "Org2").await;
    let admin1 = create_user(&pool, org1, "a1@example.com", "org_admin").await;
    let admin2 = create_user(&pool, org2, "a2@example.com", "org_admin").await;
    let token1 = generate_jwt_token(admin1, org1, "org_admin");
    let token2 = generate_jwt_token(admin2, org2, "org_admin");

    let payload = json!({
        "org_id": org1,
        "name": "Pipe",
        "stages": [{"type": "ocr"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token1)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let created: serde_json::Value = test::read_body_json(resp).await;
    let pipeline_id = created["id"].as_str().unwrap();

    let update_payload = json!({
        "org_id": org1,
        "name": "FailUpdate",
        "stages": [{"type": "ocr"}]
    });
    let req = test::TestRequest::put()
        .uri(&format!("/api/pipelines/{}", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token2)))
        .set_json(&update_payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn test_delete_pipeline_success() {
    let (app, pool) = setup_test_app().await;
    let org_id = create_org(&pool, "Delete Org").await;
    let user_id = create_user(&pool, org_id, "del@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": "DelPipe",
        "stages": [{"type": "ocr"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let created: serde_json::Value = test::read_body_json(resp).await;
    let pipeline_id = created["id"].as_str().unwrap();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/pipelines/{}", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pipelines WHERE id = $1")
        .bind(Uuid::parse_str(pipeline_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
}

#[actix_rt::test]
async fn test_delete_pipeline_other_org_unauthorized() {
    let (app, pool) = setup_test_app().await;
    let org1 = create_org(&pool, "DelOrg1").await;
    let org2 = create_org(&pool, "DelOrg2").await;
    let admin1 = create_user(&pool, org1, "del1@example.com", "org_admin").await;
    let admin2 = create_user(&pool, org2, "del2@example.com", "org_admin").await;
    let token1 = generate_jwt_token(admin1, org1, "org_admin");
    let token2 = generate_jwt_token(admin2, org2, "org_admin");

    let payload = json!({
        "org_id": org1,
        "name": "DelPipeOther",
        "stages": [{"type": "ocr"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token1)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let created: serde_json::Value = test::read_body_json(resp).await;
    let pipeline_id = created["id"].as_str().unwrap();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/pipelines/{}", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token2)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
}


