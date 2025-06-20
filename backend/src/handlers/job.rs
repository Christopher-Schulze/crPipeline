use actix_web::{get, web, HttpResponse};
use uuid::Uuid;
use sqlx::PgPool;
use crate::models::AnalysisJob;
use actix_web_lab::sse::{self, ChannelStream, Sse};
use std::time::Duration;

#[get("/jobs/{org_id}")]
async fn list_jobs(path: web::Path<Uuid>, pool: web::Data<PgPool>) -> HttpResponse {
    match AnalysisJob::find_by_org(pool.as_ref(), *path).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/jobs/{id}/events")]
async fn job_events(path: web::Path<Uuid>, pool: web::Data<PgPool>) -> Sse<ChannelStream> {
    let (tx, rx) = sse::channel(10);
    let job_id = *path;
    actix_web::rt::spawn(async move {
        loop {
            match AnalysisJob::find(pool.as_ref(), job_id).await {
                Ok(job) => {
                    if tx.send(sse::Data::new(job.status.clone())).await.is_err() { break; }
                    if job.status == "completed" || job.status == "failed" { break; }
                }
                Err(_) => { let _ = tx.send(sse::Data::new("error")).await; break; }
            }
            actix_web::rt::time::sleep(Duration::from_secs(2)).await;
        }
    });
    rx
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_jobs).service(job_events);
}
