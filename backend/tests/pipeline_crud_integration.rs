use actix_web::{test, http::header};
use uuid::Uuid;
use serde_json::json;

mod test_utils;
use test_utils::{
    setup_test_app, create_org, create_user, generate_jwt_token, clear_database,
};

#[actix_rt::test]
async fn test_create_pipeline_integration() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Crud Org").await;
    let user_id = create_user(&pool, org_id, "create@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "CreatePipe",
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
    let pipeline_id = created["id"].as_str().unwrap();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pipelines WHERE id=$1")
        .bind(Uuid::parse_str(pipeline_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    clear_database(&pool).await;
}

#[actix_rt::test]
async fn test_update_pipeline_integration() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Crud Org2").await;
    let user_id = create_user(&pool, org_id, "update@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "PipeToUpdate",
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
    let pipeline_id = created["id"].as_str().unwrap();

    let update_payload = json!({
        "org_id": org_id,
        "name": "PipeUpdated",
        "stages": [{"type": "ocr", "command": "echo hi"}]
    });
    let req = test::TestRequest::put()
        .uri(&format!("/api/pipelines/{}", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&update_payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let updated: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(updated["name"], "PipeUpdated");

    clear_database(&pool).await;
}

#[actix_rt::test]
async fn test_delete_pipeline_integration() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Crud Org3").await;
    let user_id = create_user(&pool, org_id, "delete@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "PipeToDelete",
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
    let pipeline_id = created["id"].as_str().unwrap();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/pipelines/{}", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pipelines WHERE id=$1")
        .bind(Uuid::parse_str(pipeline_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    clear_database(&pool).await;
}

