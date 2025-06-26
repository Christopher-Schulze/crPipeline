use actix_web::{web, get, post, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use crate::models::{Organization, NewOrganization, OrgSettings}; // OrgSettings for create_org, Organization for list_orgs
use crate::middleware::auth::AuthUser;
use log; // Added for logging

#[derive(Deserialize)]
pub struct OrgInput { pub name: String }

#[post("/orgs")]
async fn create_org(data: web::Json<OrgInput>, _user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    let org = NewOrganization { name: data.name.clone() };
    match Organization::create(&pool, org).await {
        Ok(o) => {
            let _ = OrgSettings::create_default(&pool, o.id).await;
            HttpResponse::Ok().json(o)
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/orgs")]
async fn list_orgs(user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse { // Changed _user to user
    let result = if user.role == "admin" {
        // Global admin gets all organizations
        Organization::all(pool.as_ref()).await
    } else {
        // Non-global admins (e.g., "user", future "org_admin") get only their own organization
        sqlx::query_as::<_, Organization>("SELECT id, name, api_key FROM organizations WHERE id = $1")
            .bind(user.org_id)
            .fetch_optional(pool.as_ref())
            .await
            .map(|opt_org| opt_org.map_or_else(Vec::new, |org| vec![org])) // Convert Option<Org> to Vec<Org>
            // For sqlx::Error conversion, if not using map_err on the query chain:
            .map_err(|e| {
                // Log the original sqlx error before converting to a generic one if needed
                log::error!("Database error fetching single organization: {:?}", e);
                // This specific conversion might not be needed if Organization::all already returns sqlx::Error
                // and the outer match handles it. This is more for converting a specific query error type.
                // For now, assume the error type from this branch is compatible with Organization::all's error type.
                e
            })
    };

    match result {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(e) => {
            log::error!("Failed to list organizations: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to retrieve organizations"}))
        }
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_org).service(list_orgs);
}
