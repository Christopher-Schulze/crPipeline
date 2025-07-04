use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use backend::models::Document;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use backend::config::CleanupConfig;
use tracing::{error, info};

async fn run_cleanup(
    pool: &sqlx::Pool<sqlx::Postgres>,
    s3: &S3Client,
    bucket: &str,
) -> anyhow::Result<()> {
    let expired: Vec<Document> = sqlx::query_as(
        "SELECT * FROM documents WHERE expires_at IS NOT NULL AND expires_at < NOW()",
    )
    .fetch_all(pool)
    .await?;

    for doc in expired {
        if let Err(e) = s3
            .delete_object()
            .bucket(bucket)
            .key(&doc.filename)
            .send()
            .await
        {
            error!("failed to delete {}: {:?}", doc.filename, e);
            continue;
        }
        sqlx::query("DELETE FROM documents WHERE id=$1")
            .bind(doc.id)
            .execute(pool)
            .await?;
        info!("Deleted expired document {}", doc.filename);
    }
    Ok(())
}

/// Remove expired documents and their blobs from storage.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = match CleanupConfig::from_env() {
        Ok(c) => c,
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    };
    tracing_subscriber::fmt::init();
    let database_url = cfg.database_url;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared = aws_config::from_env().region(region_provider).load().await;
    let s3 = S3Client::new(&shared);
    let bucket = cfg.s3_bucket;

    if let Some(interval) = cfg.interval_minutes
    {
        loop {
            run_cleanup(&pool, &s3, &bucket).await?;
            tokio::time::sleep(Duration::from_secs(interval * 60)).await;
        }
    } else {
        run_cleanup(&pool, &s3, &bucket).await?;
    }

    Ok(())
}
