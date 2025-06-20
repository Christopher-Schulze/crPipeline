use actix_web::{get, HttpResponse, Responder};

#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

pub fn routes(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(health);
}
