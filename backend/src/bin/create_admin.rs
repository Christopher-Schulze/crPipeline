use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use backend::models::{NewUser, User, Organization, NewOrganization, OrgSettings};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use tracing::{info, error};
use anyhow::Result;

/// Convenience utility to create an initial admin user.
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        error!("Usage: create_admin <email> <password>");
        return Ok(());
    }
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new().connect(&database_url).await?;
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::default()
        .hash_password(args[2].as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("failed to hash password: {e}"))?
        .to_string();
    let org = sqlx::query_as::<_, Organization>("SELECT id, name, api_key FROM organizations LIMIT 1")
        .fetch_optional(&pool)
        .await?;
    let org_id = if let Some(o) = org { o.id } else {
        let o = Organization::create(&pool, NewOrganization { name: "Default".into() }).await?;
        let _ = OrgSettings::create_default(&pool, o.id).await?;
        o.id
    };
    let user = NewUser { org_id, email: args[1].clone(), password_hash, role: "admin".into() };
    User::create(&pool, user).await?;
    info!("Admin user created");
    Ok(())
}
