use actix_web::web;

pub mod auth;
pub mod org;
pub mod document;
pub mod pipeline;
pub mod job;
pub mod health;
pub mod settings;
pub mod audit;
pub mod dashboard;
pub mod admin; // New module

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api")
        .service(auth::logout)
        .configure(auth::routes)
        .service(auth::logout)
        .configure(org::routes)
        .configure(document::routes)
        .configure(pipeline::routes)
        .configure(job::routes)
        .configure(settings::routes)
        .configure(audit::routes)
        .configure(dashboard::routes)
        .configure(admin::routes) // Add this line
        .configure(health::routes)
    );
}
