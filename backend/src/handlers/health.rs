use actix_web::{get, web, HttpResponse, Responder};
use sqlx::PgPool;
use aws_sdk_s3::Client as S3Client;

#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

#[get("/readiness")]
pub async fn readiness(
    pool: web::Data<PgPool>,
    s3: web::Data<S3Client>,
) -> impl Responder {
    let db_ok = sqlx::query("SELECT 1")
        .execute(pool.as_ref())
        .await
        .is_ok();

    let s3_ok = s3.list_buckets().send().await.is_ok();

    if db_ok && s3_ok {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::ServiceUnavailable().finish()
    }
}

pub fn routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(health).service(readiness);
}
