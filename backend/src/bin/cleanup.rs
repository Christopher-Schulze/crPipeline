#![allow(warnings)]
#![allow(clippy::all)]
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use backend::models::Document;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing::{error, info};

/// Remove expired documents and their blobs from storage.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let shared = aws_config::from_env().region(region_provider).load().await;
    let s3 = S3Client::new(&shared);
    let bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());

    let expired: Vec<Document> = sqlx::query_as(
        "SELECT * FROM documents WHERE expires_at IS NOT NULL AND expires_at < NOW()",
    )
    .fetch_all(&pool)
    .await?;

    for doc in expired {
        if let Err(e) = s3
            .delete_object()
            .bucket(&bucket)
            .key(&doc.filename)
            .send()
            .await
        {
            error!("failed to delete {}: {:?}", doc.filename, e);
            continue;
        }
        sqlx::query("DELETE FROM documents WHERE id=$1")
            .bind(doc.id)
            .execute(&pool)
            .await?;
        info!("Deleted expired document {}", doc.filename);
    }

    Ok(())
}
