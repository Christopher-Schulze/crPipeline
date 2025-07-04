use actix_web::{get, web, HttpResponse, http::StatusCode, ResponseError};
use crate::error::ApiError;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::OrgSettings; // Assuming OrgSettings is used elsewhere or can be if dashboard grows
use serde::Serialize;
use chrono::{DateTime, Utc}; // Added for DateTime<Utc>

// New struct for the response of get_recent_analyses
#[derive(Serialize, sqlx::FromRow)]
struct RecentAnalysisJob {
    job_id: Uuid,
    document_name: String,
    pipeline_name: String,
    status: String,
    created_at: DateTime<Utc>,
    document_id: Uuid,
}

#[get("/dashboard/{org_id}")]
async fn dashboard(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }
    let settings = match OrgSettings::find(pool.as_ref(), *path).await {
        Ok(s) => s,
        Err(_) => return ApiError::new("Failed to fetch settings", StatusCode::INTERNAL_SERVER_ERROR)
            .error_response(),
    };
    let (uploads,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM documents WHERE org_id=$1 AND is_target=true AND upload_date >= date_trunc('month', NOW())"
    )
        .bind(*path)
        .fetch_one(pool.as_ref())
        .await
        .unwrap_or((0,));
    let (analyses,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM analysis_jobs WHERE org_id=$1 AND created_at >= date_trunc('month', NOW())"
    )
        .bind(*path)
        .fetch_one(pool.as_ref())
        .await
        .unwrap_or((0,));

    HttpResponse::Ok().json(serde_json::json!({
        "upload_remaining": settings.monthly_upload_quota as i64 - uploads,
        "analysis_remaining": settings.monthly_analysis_quota as i64 - analyses,
    }))
}

#[derive(Serialize)]
struct UsageItem {
    month: String,
    uploads: i64,
    analyses: i64,
}

#[get("/dashboard/{org_id}/usage")]
async fn usage(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }
    let uploads_rows = sqlx::query(
        "SELECT to_char(date_trunc('month', upload_date), 'YYYY-MM') as month, COUNT(*) as count \
         FROM documents WHERE org_id=$1 AND is_target=true GROUP BY month ORDER BY month DESC LIMIT 6"
    )
    .bind(*path)
    .fetch_all(pool.as_ref())
    .await
    .unwrap_or_default();

    let analyses_rows = sqlx::query(
        "SELECT to_char(date_trunc('month', created_at), 'YYYY-MM') as month, COUNT(*) as count \
         FROM analysis_jobs WHERE org_id=$1 GROUP BY month ORDER BY month DESC LIMIT 6"
    )
    .bind(*path)
    .fetch_all(pool.as_ref())
    .await
    .unwrap_or_default();

    let mut map = std::collections::BTreeMap::new();
    for row in uploads_rows {
        let month: String = row.try_get("month").unwrap_or_default();
        let count: i64 = row.try_get("count").unwrap_or(0);
        map.entry(month).or_insert((0i64,0i64)).0 = count;
    }
    for row in analyses_rows {
        let month: String = row.try_get("month").unwrap_or_default();
        let count: i64 = row.try_get("count").unwrap_or(0);
        map.entry(month).or_insert((0i64,0i64)).1 = count;
    }
    let mut data: Vec<UsageItem> = map.into_iter()
        .map(|(m,(u,a))| UsageItem{month:m, uploads:u, analyses:a})
        .collect();
    data.sort_by(|a,b| a.month.cmp(&b.month));
    HttpResponse::Ok().json(data)
}

#[get("/dashboard/{org_id}/recent_analyses")]
async fn get_recent_analyses(
    path: web::Path<Uuid>,
    user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let org_id = *path;
    // Authorization check
    if org_id != user.org_id {
        return ApiError::new("Unauthorized", StatusCode::UNAUTHORIZED)
            .error_response();
    }

    let query = r#"
        SELECT
            aj.id as job_id,
            d.display_name as document_name,
            p.name as pipeline_name,
            aj.status,
            aj.created_at,
            aj.document_id
        FROM
            analysis_jobs aj
        JOIN
            documents d ON aj.document_id = d.id
        JOIN
            pipelines p ON aj.pipeline_id = p.id
        WHERE
            aj.org_id = $1
        ORDER BY
            aj.created_at DESC
        LIMIT 5
    "#;

    match sqlx::query_as::<_, RecentAnalysisJob>(query)
        .bind(org_id)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(e) => {
            log::error!("Failed to fetch recent analyses: {}", e);
            ApiError::new("Failed to fetch recent analyses", StatusCode::INTERNAL_SERVER_ERROR)
                .error_response()
        }
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(dashboard)
        .service(usage)
        .service(get_recent_analyses); // Added new route
}
