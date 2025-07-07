use serde::{Serialize, Deserialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Defines a sequence of stages to run on a document.
#[derive(Serialize, Deserialize, FromRow, Debug, Clone)]
pub struct Pipeline {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub stages: serde_json::Value,
}

/// Information needed to create a pipeline.
pub struct NewPipeline {
    pub org_id: Uuid,
    pub name: String,
    pub stages: serde_json::Value,
}

impl Pipeline {
    /// Insert a new pipeline and return it.
    pub async fn create(pool: &PgPool, new: NewPipeline) -> sqlx::Result<Pipeline> {
        sqlx::query_as::<_, Pipeline>("INSERT INTO pipelines (id, org_id, name, stages) VALUES ($1,$2,$3,$4) RETURNING *")
            .bind(Uuid::new_v4())
            .bind(new.org_id)
            .bind(new.name)
            .bind(new.stages)
            .fetch_one(pool)
            .await
    }

    /// Update an existing pipeline's name and stages.
    pub async fn update(pool: &PgPool, id: Uuid, name: &str, stages: serde_json::Value) -> sqlx::Result<Pipeline> {
        sqlx::query_as::<_, Pipeline>("UPDATE pipelines SET name=$1, stages=$2 WHERE id=$3 RETURNING *")
            .bind(name)
            .bind(stages)
            .bind(id)
            .fetch_one(pool)
            .await
    }

    /// Remove a pipeline by id.
    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<u64> {
        let res = sqlx::query("DELETE FROM pipelines WHERE id=$1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(res.rows_affected())
    }

    /// List pipelines by organization with optional search and pagination.
    pub async fn list_paginated(
        pool: &PgPool,
        org_id: Uuid,
        search: Option<String>,
        page: i64,
        limit: i64,
    ) -> sqlx::Result<(Vec<Pipeline>, i64)> {
        let offset = (page - 1) * limit;
        if let Some(term) = search {
            let like_term = format!("%{}%", term);
            let items = sqlx::query_as::<_, Pipeline>(
                "SELECT * FROM pipelines WHERE org_id=$1 AND name ILIKE $2 ORDER BY name ASC LIMIT $3 OFFSET $4",
            )
            .bind(org_id)
            .bind(&like_term)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;

            let total = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM pipelines WHERE org_id=$1 AND name ILIKE $2",
            )
            .bind(org_id)
            .bind(&like_term)
            .fetch_one(pool)
            .await?;

            Ok((items, total))
        } else {
            let items = sqlx::query_as::<_, Pipeline>(
                "SELECT * FROM pipelines WHERE org_id=$1 ORDER BY name ASC LIMIT $2 OFFSET $3",
            )
            .bind(org_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;

            let total = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM pipelines WHERE org_id=$1",
            )
            .bind(org_id)
            .fetch_one(pool)
            .await?;

            Ok((items, total))
        }
    }
}
