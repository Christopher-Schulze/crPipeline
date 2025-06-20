use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Serialize, FromRow, Debug)]
pub struct Pipeline {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub stages: serde_json::Value,
}

pub struct NewPipeline {
    pub org_id: Uuid,
    pub name: String,
    pub stages: serde_json::Value,
}

impl Pipeline {
    pub async fn create(pool: &PgPool, new: NewPipeline) -> sqlx::Result<Pipeline> {
        sqlx::query_as::<_, Pipeline>("INSERT INTO pipelines (id, org_id, name, stages) VALUES ($1,$2,$3,$4) RETURNING *")
            .bind(Uuid::new_v4())
            .bind(new.org_id)
            .bind(new.name)
            .bind(new.stages)
            .fetch_one(pool)
            .await
    }
}
