use actix_web::{web, get, post, HttpResponse, Scope};
use serde::Deserialize;
use sqlx::PgPool;
use crate::models::{Organization, NewOrganization, OrgSettings};
use crate::middleware::auth::AuthUser;

pub mod invites;
pub mod user_management;

pub use invites::invite_user_to_organization;
pub use user_management::{
    get_organization_users,
    remove_user_from_organization,
    deactivate_user_in_organization,
    reactivate_user_in_organization,
    resend_confirmation_email_org_user,
};

#[derive(Deserialize)]
pub struct OrgInput {
    pub name: String,
}

#[post("/orgs")]
async fn create_org(data: web::Json<OrgInput>, user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    if user.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "You do not have permission to create organizations."}));
    }

    let org_name = data.name.trim();
    if org_name.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Organization name cannot be empty."}));
    }

    let new_org = NewOrganization { name: org_name.to_string() };
    match Organization::create(&pool, new_org).await {
        Ok(created_org) => match OrgSettings::create_default(&pool, created_org.id).await {
            Ok(_) => HttpResponse::Ok().json(created_org),
            Err(e) => {
                log::error!("Failed to create default settings for new org {}: {:?}", created_org.id, e);
                HttpResponse::Ok().json(serde_json::json!({
                    "warning": "Organization created, but default settings failed to initialize.",
                    "organization": created_org
                }))
            }
        },
        Err(sqlx::Error::Database(db_err)) if db_err.constraint() == Some("organizations_name_key") => {
            HttpResponse::Conflict().json(serde_json::json!({"error": "An organization with this name already exists."}))
        }
        Err(e) => {
            log::error!("Failed to create organization '{}': {:?}", org_name, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create organization."}))
        }
    }
}

#[get("/orgs")]
async fn list_orgs(user: AuthUser, pool: web::Data<PgPool>) -> HttpResponse {
    let result = if user.role == "admin" {
        Organization::all(pool.as_ref()).await
    } else {
        sqlx::query_as::<_, Organization>("SELECT id, name, api_key FROM organizations WHERE id = $1")
            .bind(user.org_id)
            .fetch_optional(pool.as_ref())
            .await
            .map(|opt_org| opt_org.map_or_else(Vec::new, |org| vec![org]))
            .map_err(|e| {
                log::error!("Database error fetching single organization for user {}: {:?}", user.user_id, e);
                e
            })
    };

    match result {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to retrieve organizations."})),
    }
}

pub fn org_routes() -> Scope {
    web::scope("/orgs")
        .service(create_org)
        .service(list_orgs)
}

pub fn org_me_routes() -> Scope {
    web::scope("/organizations/me")
        .route("/users", web::get().to(get_organization_users))
        .route("/invite", web::post().to(invite_user_to_organization))
        .route("/users/{user_id}/remove", web::post().to(remove_user_from_organization))
        .route("/users/{user_id}/deactivate", web::post().to(deactivate_user_in_organization))
        .route("/users/{user_id}/reactivate", web::post().to(reactivate_user_in_organization))
        .route("/users/{user_id}/resend_confirmation", web::post().to(resend_confirmation_email_org_user))
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(org_routes())
        .service(org_me_routes());
}

