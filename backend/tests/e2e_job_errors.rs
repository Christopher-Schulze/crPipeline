use actix_rt::time::sleep;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::process::Command;
use mini_redis::server;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user};
use backend::models::{Pipeline, NewPipeline, Document, NewDocument, NewAnalysisJob, AnalysisJob};

async fn start_redis() -> (oneshot::Sender<()>, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let _ = server::run(listener, async { let _ = rx.await; }).await;
    });
    (tx, port)
}

#[actix_rt::test]
async fn ocr_error_marks_failed() {
    dotenvy::from_filename(".env.test").ok();
    let (shutdown, port) = start_redis().await;
    let redis_url = format!("redis://127.0.0.1:{}/", port);
    std::env::set_var("REDIS_URL", &redis_url);
    std::env::set_var("S3_BUCKET", "uploads");

    let db_url = std::env::var("DATABASE_URL_TEST")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL for tests");
    std::env::set_var("DATABASE_URL", &db_url);

    let tempdir = tempfile::tempdir().unwrap();

    let ocr_server = MockServer::start().await;
    let _ocr_mock = Mock::given(method("POST")).and(path("/ocr"))
        .respond_with(ResponseTemplate::new(500))
        .mount_as_scoped(&ocr_server)
        .await;

    let Ok((_app, pool)) = setup_test_app().await else { let _=shutdown.send(()); return; };

    let org_id = create_org(&pool, "Err Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;

    let stages = serde_json::json!([{
        "type":"ocr",
        "ocr_engine":"external",
        "ocr_stage_endpoint": format!("{}/ocr", ocr_server.uri()),
        "ocr_stage_key":"k"
    }]);
    let pipeline = Pipeline::create(&pool, NewPipeline { org_id, name: "Flow".into(), stages }).await.unwrap();

    let doc = Document::create(&pool, NewDocument {
        org_id,
        owner_id: user_id,
        filename: "input.pdf".into(),
        pages: 1,
        is_target: true,
        expires_at: None,
        display_name: "input.pdf".into(),
    }).await.unwrap();
    let local_path = tempdir.path().join("uploads").join("input.pdf");
    tokio::fs::create_dir_all(local_path.parent().unwrap()).await.unwrap();
    tokio::fs::write(&local_path, b"dummy").await.unwrap();

    let job = AnalysisJob::create(&pool, NewAnalysisJob {
        org_id,
        document_id: doc.id,
        pipeline_id: pipeline.id,
        status: "pending".into(),
    }).await.unwrap();

    let client = redis::Client::open(redis_url.clone()).unwrap();
    let mut conn = client.get_async_connection().await.unwrap();
    redis::cmd("LPUSH").arg("jobs").arg(job.id.to_string()).query_async::<_, ()>(&mut conn).await.unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_worker"))
        .env("DATABASE_URL", &db_url)
        .env("REDIS_URL", &redis_url)
        .env("S3_BUCKET", "uploads")
        .env("PROCESS_ONE_JOB", "1")
        .env("LOCAL_S3_DIR", tempdir.path())
        .spawn()
        .expect("worker binary run");

    sleep(Duration::from_secs(2)).await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    let _ = shutdown.send(());

    let job = AnalysisJob::find(&pool, job.id).await.unwrap();
    let pdf_tmp = std::env::temp_dir().join(format!("{}-input.pdf", job.id));
    let txt_tmp = pdf_tmp.with_extension("txt");
    assert!(!pdf_tmp.exists());
    assert!(!txt_tmp.exists());
    assert_eq!(job.status, "failed");
    let log_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(log_count.0 > 0);
}

#[actix_rt::test]
async fn ai_invalid_json_marks_failed() {
    dotenvy::from_filename(".env.test").ok();
    let (shutdown, port) = start_redis().await;
    let redis_url = format!("redis://127.0.0.1:{}/", port);
    std::env::set_var("REDIS_URL", &redis_url);
    std::env::set_var("S3_BUCKET", "uploads");

    let db_url = std::env::var("DATABASE_URL_TEST")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL for tests");
    std::env::set_var("DATABASE_URL", &db_url);

    let tempdir = tempfile::tempdir().unwrap();

    let ai_server = MockServer::start().await;
    let _ai_mock = Mock::given(method("POST")).and(path("/ai"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount_as_scoped(&ai_server)
        .await;

    let Ok((_app, pool)) = setup_test_app().await else { let _=shutdown.send(()); return; };

    let org_id = create_org(&pool, "AI Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;

    let stages = serde_json::json!([{"type":"ai"}]);
    let pipeline = Pipeline::create(&pool, NewPipeline { org_id, name: "AI".into(), stages }).await.unwrap();

    let doc = Document::create(&pool, NewDocument {
        org_id,
        owner_id: user_id,
        filename: "input.pdf".into(),
        pages: 1,
        is_target: true,
        expires_at: None,
        display_name: "input.pdf".into(),
    }).await.unwrap();
    let local_path = tempdir.path().join("uploads").join("input.pdf");
    tokio::fs::create_dir_all(local_path.parent().unwrap()).await.unwrap();
    tokio::fs::write(&local_path, b"dummy").await.unwrap();

    let job = AnalysisJob::create(&pool, NewAnalysisJob {
        org_id,
        document_id: doc.id,
        pipeline_id: pipeline.id,
        status: "pending".into(),
    }).await.unwrap();

    let client = redis::Client::open(redis_url.clone()).unwrap();
    let mut conn = client.get_async_connection().await.unwrap();
    redis::cmd("LPUSH").arg("jobs").arg(job.id.to_string()).query_async::<_, ()>(&mut conn).await.unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_worker"))
        .env("DATABASE_URL", &db_url)
        .env("REDIS_URL", &redis_url)
        .env("S3_BUCKET", "uploads")
        .env("PROCESS_ONE_JOB", "1")
        .env("LOCAL_S3_DIR", tempdir.path())
        .env("AI_API_URL", format!("{}/ai", ai_server.uri()))
        .env("AI_API_KEY", "k")
        .spawn()
        .expect("worker binary run");

    sleep(Duration::from_secs(2)).await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    let _ = shutdown.send(());

    let job = AnalysisJob::find(&pool, job.id).await.unwrap();
    let pdf_tmp = std::env::temp_dir().join(format!("{}-input.pdf", job.id));
    let txt_tmp = pdf_tmp.with_extension("txt");
    assert!(!pdf_tmp.exists());
    assert!(!txt_tmp.exists());
    assert_eq!(job.status, "failed");
    let log_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE org_id=$1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(log_count.0 > 0);
}
