use actix_web::{http::header, test};
use serde_json::json;

mod test_utils;
use backend::models::{AnalysisJob, Document, NewAnalysisJob, NewDocument, NewPipeline, Pipeline};
use test_utils::{create_org, create_user, generate_jwt_token, setup_test_app};
use uuid::Uuid;

#[actix_rt::test]
async fn list_jobs_includes_names() {
    let Ok((app, pool)) = setup_test_app().await else {
        return;
    };
    let org_id = create_org(&pool, "Job Org").await;
    let user_id = create_user(&pool, org_id, "user@example.com", "org_admin").await;
    let _token = generate_jwt_token(user_id, org_id, "org_admin");

    let pipeline = Pipeline::create(
        &pool,
        NewPipeline {
            org_id,
            name: "Pipe".into(),
            stages: json!([]),
        },
    )
    .await
    .unwrap();
    let document = Document::create(
        &pool,
        NewDocument {
            org_id,
            owner_id: user_id,
            filename: "f.pdf".into(),
            pages: 1,
            is_target: true,
            expires_at: None,
            display_name: "File.pdf".into(),
        },
    )
    .await
    .unwrap();
    let _job = AnalysisJob::create(
        &pool,
        NewAnalysisJob {
            org_id,
            document_id: document.id,
            pipeline_id: pipeline.id,
            status: "pending".into(),
        },
    )
    .await
    .unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/jobs/{}", org_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let list: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    let first = &list[0];
    assert_eq!(first["document_name"], "File.pdf");
    assert_eq!(first["pipeline_name"], "Pipe");
}
