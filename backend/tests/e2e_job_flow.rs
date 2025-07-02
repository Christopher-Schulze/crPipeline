use actix_rt::time::sleep;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::process::Command;
use mini_redis::server;

mod test_utils;
use test_utils::{setup_test_app, create_org, create_user};
use backend::models::{Pipeline, NewPipeline, Document, NewDocument, NewAnalysisJob, AnalysisJob};
use uuid::Uuid;

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
async fn e2e_job_flow() {
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

    let Ok((_app, pool)) = setup_test_app().await else { let _=shutdown.send(()); return; };

    let org_id = create_org(&pool, "E2E Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;

    let script = tempdir.path().join("ocr.sh");
    tokio::fs::write(&script, "#!/bin/sh\necho sample > $2").await.unwrap();
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).await.unwrap();

    let stages = serde_json::json!([{"type":"ocr","command":script.to_string_lossy()}]);
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
    assert_eq!(job.status, "completed");
    let outs = backend::models::job_stage_output::JobStageOutput::find_by_job_id(&pool, job.id).await.unwrap();
    assert!(!outs.is_empty());
}

