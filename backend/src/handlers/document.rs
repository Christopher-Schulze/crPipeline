use actix_multipart::Multipart;
use actix_web::{post, get, web, HttpResponse};
use futures_util::StreamExt as _;
use aws_sdk_s3::Client;
use aws_sdk_s3::presigning::PresigningConfig;
use redis::AsyncCommands;
use uuid::Uuid;
use lopdf::Document as PdfDoc;
use crate::models::{Document, NewDocument, AnalysisJob, NewAnalysisJob, OrgSettings};
use crate::utils::log_action;
use crate::middleware::auth::AuthUser;
use sqlx::PgPool;
use std::time::Duration;

#[derive(serde::Deserialize)]
pub struct UploadParams {
    pub org_id: Uuid,
    pub owner_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub is_target: Option<bool>,
}

#[post("/upload")]
pub async fn upload(
    mut payload: Multipart,
    params: web::Query<UploadParams>,
    user: AuthUser,
    pool: web::Data<sqlx::PgPool>,
    s3: web::Data<Client>,
) -> HttpResponse {
    let mut filename = String::new();
    let mut bytes = Vec::new();
    while let Some(Ok(mut field)) = payload.next().await {
        if let Some(name) = field.content_disposition().get_filename() {
            filename = name.to_string();
        }
        while let Some(chunk) = field.next().await {
            let data = match chunk { Ok(d) => d, Err(_) => return HttpResponse::BadRequest().finish() };
            bytes.extend_from_slice(&data);
        }
    }
    if !filename.to_lowercase().ends_with(".pdf") || bytes.len() > 200 * 1024 * 1024 {
        return HttpResponse::BadRequest().body("Invalid file");
    }
    let pages = match PdfDoc::load_mem(&bytes).map(|d| d.get_pages().len() as i32) {
        Ok(p) => p,
        Err(_) => return HttpResponse::BadRequest().body("Corrupt PDF"),
    };
    if params.org_id != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }
    // check monthly upload quota for target documents
    if params.is_target.unwrap_or(false) {
        if let Ok(settings) = OrgSettings::find(&pool, params.org_id).await {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM documents WHERE org_id=$1 AND is_target=true AND upload_date >= date_trunc('month', NOW())"
            )
            .bind(params.org_id)
            .fetch_one(pool.as_ref())
            .await
            .unwrap_or((0,));
            if count >= settings.monthly_upload_quota as i64 {
                return HttpResponse::TooManyRequests().body("upload quota exceeded");
            }
        }
    }
    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
    let key = format!("{}-{}", Uuid::new_v4(), filename);
    if s3.put_object().bucket(&bucket).key(&key).body(bytes.into()).send().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    let doc = NewDocument {
        org_id: params.org_id,
        owner_id: params.owner_id,
        filename: key.clone(),
        pages,
        is_target: params.is_target.unwrap_or(false),
        expires_at: None,
    };
    let created = match Document::create(&pool, doc).await {
        Ok(d) => d,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    log_action(&pool, user.org_id, user.user_id, "upload").await;

    if let Some(pipeline_id) = params.pipeline_id {
        if let Ok(settings) = OrgSettings::find(&pool, params.org_id).await {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM analysis_jobs WHERE org_id=$1 AND created_at >= date_trunc('month', NOW())"
            )
            .bind(params.org_id)
            .fetch_one(pool.as_ref())
            .await
            .unwrap_or((0,));
            if count >= settings.monthly_analysis_quota as i64 {
                return HttpResponse::TooManyRequests().body("analysis quota exceeded");
            }
        }
        let job = NewAnalysisJob {
            org_id: params.org_id,
            document_id: created.id,
            pipeline_id,
            status: "pending".into(),
        };
        if let Ok(j) = AnalysisJob::create(&pool, job).await {
            log_action(&pool, user.org_id, user.user_id, &format!("job_created:{}", j.id)).await;
            if let Ok(redis_url) = std::env::var("REDIS_URL") {
                if let Ok(client) = redis::Client::open(redis_url) {
                    if let Ok(mut conn) = client.get_async_connection().await {
                        let _: Result<(), _> = conn.rpush("jobs", j.id.to_string()).await;
                    }
                }
            }
        }
    }

    HttpResponse::Ok().json(created)
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(upload)
        .service(list_documents)
        .service(download);
}

#[get("/documents/{org_id}")]
async fn list_documents(path: web::Path<Uuid>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if *path != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }
    match sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE org_id=$1")
        .bind(*path)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/download/{id}")]
async fn download(
    path: web::Path<Uuid>,
    user: AuthUser,
    pool: web::Data<PgPool>,
    s3: web::Data<Client>,
) -> HttpResponse {
    let doc = match sqlx::query_as::<_, Document>("SELECT * FROM documents WHERE id=$1")
        .bind(*path)
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(d) => d,
        Err(_) => return HttpResponse::NotFound().finish(),
    };

    if doc.org_id != user.org_id {
        return HttpResponse::Unauthorized().finish();
    }

    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
    let presigned = match s3
        .get_object()
        .bucket(&bucket)
        .key(&doc.filename)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(3600)).unwrap())
        .await
    {
        Ok(p) => p,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    log_action(&pool, user.org_id, user.user_id, &format!("download:{}", doc.id)).await;
    HttpResponse::Ok().json(serde_json::json!({ "url": presigned.uri().to_string() }))
}
