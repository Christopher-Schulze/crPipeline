use actix_web::web;

pub mod admin;
pub mod audit;
pub mod auth;
pub mod dashboard;
pub mod document;
pub mod health;
pub mod job;
pub mod org;
pub mod pipeline;
pub mod settings; // New module

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(auth::routes)
            .configure(org::routes)
            .configure(document::routes)
            .configure(pipeline::routes)
            .configure(job::routes)
            .configure(settings::routes)
            .configure(audit::routes)
            .configure(dashboard::routes)
            .configure(admin::routes) // Add this line
            .configure(health::routes),
    );
}
