use actix_web::{get, web, HttpResponse};
use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::middleware::auth::AuthUser;
use crate::models::{OrgSettings};
use serde::Serialize;

#[get("/dashboard/{org_id}")]
async fn dashboard(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }
    let settings = match OrgSettings::find(pool.as_ref(), *path).await {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().finish(),
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
        return HttpResponse::Unauthorized().finish();
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

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(dashboard)
        .service(usage);
}
