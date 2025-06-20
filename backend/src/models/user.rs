use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordVerifier};
use chrono::{DateTime, Utc};

#[derive(Serialize, FromRow, Debug)]
pub struct User {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub confirmed: bool,
    pub confirmation_token: Option<Uuid>,
    pub reset_token: Option<Uuid>,
    pub reset_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct NewUser {
    pub org_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: String,
}

impl User {
    pub async fn create(pool: &PgPool, new: NewUser) -> sqlx::Result<User> {
        let confirmation_token = Uuid::new_v4();
        let rec = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, org_id, email, password_hash, role, confirmed, confirmation_token) \
             VALUES ($1, $2, $3, $4, $5, false, $6) RETURNING *"
        )
            .bind(Uuid::new_v4())
            .bind(new.org_id)
            .bind(new.email)
            .bind(new.password_hash)
            .bind(new.role)
            .bind(confirmation_token)
            .fetch_one(pool)
            .await?;
        Ok(rec)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> sqlx::Result<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email=$1")
            .bind(email)
            .fetch_one(pool)
            .await
    }

    pub fn verify_password(&self, password: &str) -> bool {
        let parsed = PasswordHash::new(&self.password_hash).unwrap();
        Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
    }

    pub async fn confirm(pool: &PgPool, token: Uuid) -> sqlx::Result<Option<User>> {
        if let Some(user) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE confirmation_token=$1")
            .bind(token)
            .fetch_optional(pool)
            .await? {
                sqlx::query("UPDATE users SET confirmed=true, confirmation_token=NULL WHERE id=$1")
                    .bind(user.id)
                    .execute(pool)
                    .await?;
                return Ok(Some(user));
        }
        Ok(None)
    }

    pub async fn set_reset_token(pool: &PgPool, user_id: Uuid, token: Uuid, expires: DateTime<Utc>) -> sqlx::Result<()> {
        sqlx::query("UPDATE users SET reset_token=$1, reset_expires_at=$2 WHERE id=$3")
            .bind(token)
            .bind(expires)
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn reset_with_token(pool: &PgPool, token: Uuid, new_hash: String) -> sqlx::Result<bool> {
        if let Some(user) = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE reset_token=$1 AND reset_expires_at > NOW()"
        )
        .bind(token)
        .fetch_optional(pool)
        .await? {
            sqlx::query("UPDATE users SET password_hash=$1, reset_token=NULL, reset_expires_at=NULL WHERE id=$2")
                .bind(new_hash)
                .bind(user.id)
                .execute(pool)
                .await?;
            return Ok(true);
        }
        Ok(false)
    }
}
