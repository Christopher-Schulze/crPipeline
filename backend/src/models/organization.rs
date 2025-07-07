use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Organization that owns users and documents.
#[derive(Serialize, FromRow, Debug)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub api_key: Uuid,
}

/// Data to create a new organization record.
pub struct NewOrganization {
    pub name: String,
}

impl Organization {
    /// Insert a new organization and return it.
    pub async fn create(pool: &PgPool, new: NewOrganization) -> sqlx::Result<Organization> {
        sqlx::query_as::<_, Organization>(
            "INSERT INTO organizations (id, name) VALUES ($1, $2) RETURNING id, name, api_key",
        )
        .bind(Uuid::new_v4())
        .bind(new.name)
        .fetch_one(pool)
        .await
    }

    /// Retrieve all organizations.
    pub async fn all(pool: &PgPool) -> sqlx::Result<Vec<Organization>> {
        sqlx::query_as::<_, Organization>("SELECT id, name, api_key FROM organizations")
            .fetch_all(pool)
            .await
    }

    /// Update organization name and return updated org
    pub async fn update_name(pool: &PgPool, org_id: Uuid, name: String) -> sqlx::Result<Organization> {
        sqlx::query_as::<_, Organization>("UPDATE organizations SET name=$1 WHERE id=$2 RETURNING id, name, api_key")
            .bind(name)
            .bind(org_id)
            .fetch_one(pool)
            .await
    }
}
