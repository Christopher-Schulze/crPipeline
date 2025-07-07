use actix_rt::time::sleep;
use backend::models::{AnalysisJob, Document, NewAnalysisJob, NewDocument, Pipeline};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

#[actix_rt::test]
async fn worker_reloads_concurrency_via_sighup() {
    dotenvy::dotenv().ok();
    std::env::set_var("DATABASE_URL", "postgres://postgres@localhost/testdb");
    std::env::set_var("REDIS_URL", "redis://127.0.0.1/");
    std::env::set_var("S3_BUCKET", "uploads");

    let tempdir = TempDir::new().unwrap();
    std::env::set_var("LOCAL_S3_DIR", tempdir.path());

    let env_file = tempdir.path().join("worker.env");
    std::fs::write(&env_file, format!(
        "WORKER_CONCURRENCY=1\nDATABASE_URL=postgres://postgres@localhost/testdb\nREDIS_URL=redis://127.0.0.1/\nS3_BUCKET=uploads\nLOCAL_S3_DIR={}\n",
        tempdir.path().display()
    )).unwrap();
    std::env::set_var("ENV_FILE", &env_file);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres@localhost/testdb")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name, api_key) VALUES ($1,$2, uuid_generate_v4())")
        .bind(org_id)
        .bind("Test")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO org_settings (org_id) VALUES ($1)")
        .bind(org_id)
        .execute(&pool)
        .await
        .unwrap();
    let user_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, org_id, email, password_hash, role, confirmed) VALUES ($1,$2,$3,$4,$5,true)")
        .bind(user_id)
        .bind(org_id)
        .bind("user@example.com")
        .bind("hash")
        .bind("admin")
        .execute(&pool)
        .await
        .unwrap();

    let script = tempdir.path().join("ocr_sleep.sh");
    tokio::fs::write(&script, "#!/bin/sh\nsleep 4\necho sample > $2").await.unwrap();
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).await.unwrap();

    let stages = serde_json::json!([{"type":"ocr","command":script.to_string_lossy()}]);
    let pipeline = Pipeline::create(
        &pool,
        backend::models::NewPipeline {
            org_id,
            name: "Test".into(),
            stages: stages.clone(),
        },
    )
    .await
    .unwrap();

    let mut job_ids = Vec::new();
    for name in ["a.pdf", "b.pdf"] {
        let doc = Document::create(
            &pool,
            NewDocument {
                org_id,
                owner_id: user_id,
                filename: name.into(),
                pages: 1,
                is_target: true,
                expires_at: None,
                display_name: name.into(),
            },
        )
        .await
        .unwrap();
        let path = tempdir.path().join("uploads").join(name);
        tokio::fs::create_dir_all(path.parent().unwrap()).await.unwrap();
        tokio::fs::write(&path, b"dummy").await.unwrap();

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
        job_ids.push(job.id);
    }

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut conn = client.get_async_connection().await.unwrap();
    for id in &job_ids {
        redis::cmd("LPUSH")
            .arg("jobs")
            .arg(id.to_string())
            .query_async::<_, ()>(&mut conn)
            .await
            .unwrap();
    }

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_worker"))
        .env("ENV_FILE", &env_file)
        .spawn()
        .expect("worker binary run");

    sleep(Duration::from_secs(1)).await;
    std::fs::write(&env_file, format!(
        "WORKER_CONCURRENCY=2\nDATABASE_URL=postgres://postgres@localhost/testdb\nREDIS_URL=redis://127.0.0.1/\nS3_BUCKET=uploads\nLOCAL_S3_DIR={}\n",
        tempdir.path().display()
    )).unwrap();
    unsafe { libc::kill(child.id() as i32, libc::SIGHUP) };

    sleep(Duration::from_secs(6)).await;
    let _ = child.kill();
    let _ = child.wait();

    for id in job_ids {
        let outs = backend::models::job_stage_output::JobStageOutput::find_by_job_id(&pool, id)
            .await
            .unwrap();
        assert!(!outs.is_empty());
    }
}
