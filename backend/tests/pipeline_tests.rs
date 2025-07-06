// backend/tests/pipeline_tests.rs
use actix_web::{test, http::header};
use uuid::Uuid;
use serde_json::json;

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user, generate_jwt_token};

#[actix_rt::test]
async fn test_create_pipeline_success() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
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
    let Ok((app, pool)) = setup_test_app().await else { return; };
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
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_reject_empty_pipeline_name() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
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
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_update_pipeline_success_as_org_admin() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
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
    let Ok((app, pool)) = setup_test_app().await else { return; };
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
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_delete_pipeline_success() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
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
async fn test_clone_pipeline_success() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Clone Org").await;
    let user_id = create_user(&pool, org_id, "clone@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "OrigPipe",
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

    let req = test::TestRequest::post()
        .uri(&format!("/api/pipelines/{}/clone", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let cloned: serde_json::Value = test::read_body_json(resp).await;
    assert_ne!(cloned["id"], created["id"]);
    assert_eq!(cloned["name"], "OrigPipe (copy)");

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pipelines WHERE org_id = $1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 2);
}

#[actix_rt::test]
async fn test_clone_pipeline_other_org_unauthorized() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org1 = create_org(&pool, "CloneOrg1").await;
    let org2 = create_org(&pool, "CloneOrg2").await;
    let admin1 = create_user(&pool, org1, "c1@example.com", "org_admin").await;
    let admin2 = create_user(&pool, org2, "c2@example.com", "org_admin").await;
    let token1 = generate_jwt_token(admin1, org1, "org_admin");
    let token2 = generate_jwt_token(admin2, org2, "org_admin");

    let payload = json!({
        "org_id": org1,
        "name": "CloneMe",
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

    let req = test::TestRequest::post()
        .uri(&format!("/api/pipelines/{}/clone", pipeline_id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token2)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pipelines WHERE org_id = $1")
        .bind(org1)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[actix_rt::test]
async fn test_delete_pipeline_cleans_up() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Cleanup Org").await;
    let user_id = create_user(&pool, org_id, "cleanup@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "ToRemove",
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
    let Ok((app, pool)) = setup_test_app().await else { return; };
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
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_reject_duplicate_stage_ids() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Dup Org").await;
    let user_id = create_user(&pool, org_id, "dup@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": "DupPipe",
        "stages": [
            {"id":"a", "type":"ocr"},
            {"id":"a", "type":"ai"}
        ]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_reject_invalid_command_type() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Cmd Org").await;
    let user_id = create_user(&pool, org_id, "cmd@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": "CmdPipe",
        "stages": [{"type": "ocr", "command": 123}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_reject_empty_stage_type() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "EmptyTypeOrg").await;
    let user_id = create_user(&pool, org_id, "etype@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": "PipeTypeEmpty",
        "stages": [{"type": "", "command": "echo hi"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_reject_empty_command_field() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "EmptyCmdOrg").await;
    let user_id = create_user(&pool, org_id, "ecmd@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": "PipeCmdEmpty",
        "stages": [{"type": "ocr", "command": ""}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}

#[actix_rt::test]
async fn test_reject_external_ocr_without_endpoint() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Ocr Org").await;
    let user_id = create_user(&pool, org_id, "ocr@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");
    let payload = json!({
        "org_id": org_id,
        "name": "OcrPipe",
        "stages": [{"type": "ocr", "ocr_engine": "external"}]
    });
    let req = test::TestRequest::post()
        .uri("/api/pipelines")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("error").is_some());
}


#[actix_rt::test]
async fn test_post_api_pipelines() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Post Org").await;
    let user_id = create_user(&pool, org_id, "post@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "PostPipeline",
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

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pipelines WHERE id = $1")
        .bind(Uuid::parse_str(pipeline_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[actix_rt::test]
async fn test_put_api_pipelines_id() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Put Org").await;
    let user_id = create_user(&pool, org_id, "put@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "InitPipe",
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
        "name": "UpdatedPipe",
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
    assert_eq!(updated["name"], "UpdatedPipe");
}

#[actix_rt::test]
async fn test_delete_api_pipelines_id() {
    let Ok((app, pool)) = setup_test_app().await else { return; };
    let org_id = create_org(&pool, "Delete Org2").await;
    let user_id = create_user(&pool, org_id, "delete@example.com", "org_admin").await;
    let token = generate_jwt_token(user_id, org_id, "org_admin");

    let payload = json!({
        "org_id": org_id,
        "name": "ToDelete",
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
