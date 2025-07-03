use backend::config::AdminConfig;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = AdminConfig::from_env().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(())
}
