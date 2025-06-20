use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Serialize, FromRow, Debug)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub api_key: Uuid,
}

pub struct NewOrganization {
    pub name: String,
}

impl Organization {
    pub async fn create(pool: &PgPool, new: NewOrganization) -> sqlx::Result<Organization> {
        sqlx::query_as::<_, Organization>(
            "INSERT INTO organizations (id, name) VALUES ($1, $2) RETURNING id, name, api_key",
        )
        .bind(Uuid::new_v4())
        .bind(new.name)
        .fetch_one(pool)
        .await
    }

    pub async fn all(pool: &PgPool) -> sqlx::Result<Vec<Organization>> {
        sqlx::query_as::<_, Organization>("SELECT id, name, api_key FROM organizations")
            .fetch_all(pool)
            .await
    }
}
