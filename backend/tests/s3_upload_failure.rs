use actix_rt::time::sleep;
use mini_redis::server;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::oneshot;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

mod test_utils;
use backend::models::{AnalysisJob, Document, NewAnalysisJob, NewDocument, NewPipeline, Pipeline};
use sqlx::postgres::PgPoolOptions;
use test_utils::{create_org, create_user};

async fn start_redis() -> (oneshot::Sender<()>, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let _ = server::run(listener, async {
            let _ = rx.await;
        })
        .await;
    });
    (tx, port)
}

#[actix_rt::test]
async fn cleanup_called_on_stage_failure() {
    dotenvy::from_filename(".env.test").ok();
    let (shutdown, port) = start_redis().await;
    let redis_url = format!("redis://127.0.0.1:{}/", port);
    std::env::set_var("REDIS_URL", &redis_url);
    std::env::set_var("S3_BUCKET", "uploads");

    let db_url = std::env::var("DATABASE_URL_TEST")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL for tests");
    std::env::set_var("DATABASE_URL", &db_url);

    let s3_server = MockServer::start().await;
    let _get_mock = Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"pdf"))
        .mount_as_scoped(&s3_server)
        .await;
    let _put_mock = Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(500))
        .mount_as_scoped(&s3_server)
        .await;
    let delete_mock = Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204))
        .mount_as_scoped(&s3_server)
        .await;

    std::env::set_var("AWS_ENDPOINT", s3_server.uri());
    std::env::set_var("AWS_ACCESS_KEY", "k");
    std::env::set_var("AWS_SECRET_KEY", "k");
    std::env::set_var("PROCESS_ONE_JOB", "1");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let org_id = create_org(&pool, "Cleanup Org").await;
    let user_id = create_user(&pool, org_id, "admin@example.com", "org_admin").await;

    let stages = serde_json::json!([{"type":"report"}]);
    let pipeline = Pipeline::create(
        &pool,
        NewPipeline {
            org_id,
            name: "T".into(),
            stages,
        },
    )
    .await
    .unwrap();

    let doc = Document::create(
        &pool,
        NewDocument {
            org_id,
            owner_id: user_id,
            filename: "input.pdf".into(),
            pages: 1,
            is_target: true,
            expires_at: None,
            display_name: "input.pdf".into(),
        },
    )
    .await
    .unwrap();

    let job = AnalysisJob::create(
        &pool,
        NewAnalysisJob {
            org_id,
            document_id: doc.id,
            pipeline_id: pipeline.id,
            status: "pending".into(),
        },
    )
    .await
    .unwrap();

    let client = redis::Client::open(redis_url.clone()).unwrap();
    let mut conn = client.get_async_connection().await.unwrap();
    redis::cmd("LPUSH")
        .arg("jobs")
        .arg(job.id.to_string())
        .query_async::<_, ()>(&mut conn)
        .await
        .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_worker"))
        .env("DATABASE_URL", &db_url)
        .env("REDIS_URL", &redis_url)
        .env("S3_BUCKET", "uploads")
        .env("AWS_ENDPOINT", s3_server.uri())
        .env("AWS_ACCESS_KEY", "k")
        .env("AWS_SECRET_KEY", "k")
        .env("PROCESS_ONE_JOB", "1")
        .spawn()
        .expect("worker binary run");

    sleep(Duration::from_secs(2)).await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    let _ = shutdown.send(());

    let job = AnalysisJob::find(&pool, job.id).await.unwrap();
    assert_eq!(job.status, "failed");
    assert_eq!(delete_mock.received_requests().await.len(), 1);
}
