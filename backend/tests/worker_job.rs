use actix_rt::time::sleep;
use backend::models::{AnalysisJob, Document, NewAnalysisJob, NewDocument, Pipeline};
use backend::worker::metrics::{JOB_COUNTER, STAGE_HISTOGRAM};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

#[actix_rt::test]
async fn worker_processes_job() {
    dotenvy::dotenv().ok();
    std::env::set_var("DATABASE_URL", "postgres://postgres@localhost/testdb");
    std::env::set_var("REDIS_URL", "redis://127.0.0.1/");
    std::env::set_var("S3_BUCKET", "uploads");
    std::env::set_var("PROCESS_ONE_JOB", "1");

    let before_jobs = JOB_COUNTER.with_label_values(&["success"]).get();
    let before_hist = STAGE_HISTOGRAM
        .with_label_values(&["ocr"])
        .get_sample_count();

    let tempdir = tempfile::tempdir().unwrap();
    std::env::set_var("LOCAL_S3_DIR", tempdir.path());

    // setup postgres
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres@localhost/testdb")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // insert org, settings, user
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

    // create dummy ocr script
    let script = tempdir.path().join("ocr.sh");
    tokio::fs::write(&script, "#!/bin/sh\necho sample > $2")
        .await
        .unwrap();
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .await
        .unwrap();

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

    // create document and place file in local storage
    let doc = Document::create(
        &pool,
        NewDocument {
            org_id,
            owner_id: user_id,
            filename: "test.pdf".into(),
            pages: 1,
            is_target: true,
            expires_at: None,
            display_name: "test.pdf".into(),
        },
    )
    .await
    .unwrap();
    let doc_path = tempdir.path().join("uploads").join("test.pdf");
    tokio::fs::create_dir_all(doc_path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&doc_path, b"dummy").await.unwrap();

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

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut conn = client.get_async_connection().await.unwrap();
    redis::cmd("LPUSH")
        .arg("jobs")
        .arg(job.id.to_string())
        .query_async::<_, ()>(&mut conn)
        .await
        .unwrap();

    // run worker binary
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_worker"))
        .spawn()
        .expect("worker binary run");

    sleep(Duration::from_secs(2)).await;
    let _ = child.kill();
    let _ = child.wait();

    let outs = backend::models::job_stage_output::JobStageOutput::find_by_job_id(&pool, job.id)
        .await
        .unwrap();
    assert!(!outs.is_empty());

    let after_jobs = JOB_COUNTER.with_label_values(&["success"]).get();
    let after_hist = STAGE_HISTOGRAM
        .with_label_values(&["ocr"])
        .get_sample_count();
    assert!(after_jobs > before_jobs);
    assert!(after_hist > before_hist);
}

#[actix_rt::test]
async fn worker_processes_jobs_concurrently() {
    dotenvy::dotenv().ok();
    std::env::set_var("DATABASE_URL", "postgres://postgres@localhost/testdb");
    std::env::set_var("REDIS_URL", "redis://127.0.0.1/");
    std::env::set_var("S3_BUCKET", "uploads");
    std::env::set_var("WORKER_CONCURRENCY", "2");

    let tempdir = tempfile::tempdir().unwrap();
    std::env::set_var("LOCAL_S3_DIR", tempdir.path());

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
    tokio::fs::write(&script, "#!/bin/sh\nsleep 2\necho sample > $2")
        .await
        .unwrap();
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .await
        .unwrap();

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
        tokio::fs::create_dir_all(path.parent().unwrap())
            .await
            .unwrap();
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
        .spawn()
        .expect("worker binary run");

    sleep(Duration::from_secs(3)).await;
    let _ = child.kill();
    let _ = child.wait();

    for id in job_ids {
        let outs = backend::models::job_stage_output::JobStageOutput::find_by_job_id(&pool, id)
            .await
            .unwrap();
        assert!(!outs.is_empty());
    }
}
