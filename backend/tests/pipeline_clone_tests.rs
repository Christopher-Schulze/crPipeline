use actix_web::{http::header, test};
use serde_json::json;
use uuid::Uuid;

mod test_utils;
use test_utils::{clear_database, create_org, create_user, generate_jwt_token, setup_test_app};

#[actix_rt::test]
async fn test_clone_pipeline_success_integration() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Clone Org").await;
    let user_id = create_user(&pool, org_id, "clone@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "OrigPipe",
        "stages": [{"type": "ocr", "command": "echo"}]
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

    let req = test::TestRequest::post()
        .uri(&format!("/api/pipelines/{}/clone", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let cloned: serde_json::Value = test::read_body_json(resp).await;
    assert_ne!(cloned["id"], created["id"]);
    assert_eq!(cloned["name"], "OrigPipe (copy)");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pipelines WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 2);

    clear_database(&pool).await;
}

#[actix_rt::test]
async fn test_clone_pipeline_other_org_unauthorized() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org1 = create_org(&pool, "Org1").await;
    let org2 = create_org(&pool, "Org2").await;
    let admin1 = create_user(&pool, org1, "a1@example.com", "org_admin").await;
    let admin2 = create_user(&pool, org2, "a2@example.com", "org_admin").await;
    let token1 = generate_jwt_token(admin1, org1, "org_admin");
    let token2 = generate_jwt_token(admin2, org2, "org_admin");

    let payload = json!({
        "org_id": org1,
        "name": "Pipe",
        "stages": [{"type": "ocr", "command": "echo"}]
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

    let req = test::TestRequest::post()
        .uri(&format!("/api/pipelines/{}/clone", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token2)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pipelines WHERE org_id=$1")
        .bind(org1)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    clear_database(&pool).await;
}

#[actix_rt::test]
async fn test_clone_pipeline_not_found() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "NF Org").await;
    let user_id = create_user(&pool, org_id, "nf@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let missing_id = Uuid::new_v4();
    let req = test::TestRequest::post()
        .uri(&format!("/api/pipelines/{}/clone", missing_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());

    clear_database(&pool).await;
}
