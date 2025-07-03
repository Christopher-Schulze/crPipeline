use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Serialize, FromRow, Debug)]
pub struct AnalysisJob {
    pub id: Uuid,
    pub org_id: Uuid,
    pub document_id: Uuid,
    pub pipeline_id: Uuid,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct NewAnalysisJob {
    pub org_id: Uuid,
    pub document_id: Uuid,
    pub pipeline_id: Uuid,
    pub status: String,
}

/// Analysis job with joined document and pipeline names.
#[derive(Serialize, FromRow, Debug)]
pub struct JobWithNames {
    pub id: Uuid,
    pub org_id: Uuid,
    pub document_id: Uuid,
    pub pipeline_id: Uuid,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub document_name: String,
    pub pipeline_name: String,
}

impl AnalysisJob {
    pub async fn create(pool: &PgPool, new: NewAnalysisJob) -> sqlx::Result<AnalysisJob> {
        sqlx::query_as::<_, AnalysisJob>("INSERT INTO analysis_jobs (id, org_id, document_id, pipeline_id, status) VALUES ($1,$2,$3,$4,$5) RETURNING *")
            .bind(Uuid::new_v4())
            .bind(new.org_id)
            .bind(new.document_id)
            .bind(new.pipeline_id)
            .bind(new.status)
            .fetch_one(pool)
            .await
    }

    pub async fn next_pending(pool: &PgPool) -> sqlx::Result<Option<AnalysisJob>> {
        sqlx::query_as::<_, AnalysisJob>(
            "SELECT * FROM analysis_jobs WHERE status='pending' ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn update_status(pool: &PgPool, id: Uuid, status: &str) -> sqlx::Result<()> {
        sqlx::query("UPDATE analysis_jobs SET status=$1 WHERE id=$2")
            .bind(status)
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(())
    }

    pub async fn find_by_org(pool: &PgPool, org: Uuid) -> sqlx::Result<Vec<JobWithNames>> {
        sqlx::query_as::<_, JobWithNames>(
            r#"
            SELECT aj.*, d.display_name AS document_name, p.name AS pipeline_name
            FROM analysis_jobs aj
            JOIN documents d ON aj.document_id = d.id
            JOIN pipelines p ON aj.pipeline_id = p.id
            WHERE aj.org_id = $1
            ORDER BY aj.created_at DESC
            "#,
        )
        .bind(org)
        .fetch_all(pool)
        .await
    }

    pub async fn find(pool: &PgPool, id: Uuid) -> sqlx::Result<AnalysisJob> {
        sqlx::query_as::<_, AnalysisJob>("SELECT * FROM analysis_jobs WHERE id=$1")
            .bind(id)
            .fetch_one(pool)
            .await
    }
}
