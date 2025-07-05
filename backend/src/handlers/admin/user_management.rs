use actix_web::{get, post, put, web, HttpResponse, Responder};
use serde::{Serialize, Deserialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::middleware::auth::AuthUser;
use crate::models::User as UserModel;
use crate::email::enqueue_email;

#[derive(Deserialize, Debug)]
pub struct AssignRolePayload {
    pub role: String,
    pub org_id: Option<Uuid>,
}

#[derive(Serialize, FromRow, Debug)]
pub struct AdminUserView {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub org_id: Uuid,
    pub organization_name: Option<String>,
    pub confirmed: bool,
    pub is_active: bool,
    pub deactivated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct PaginatedUsers {
    pub items: Vec<AdminUserView>,
    pub total_items: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[get("/admin/users")]
pub async fn list_all_users(
    query: web::Query<PaginationParams>,
    user: AuthUser,
    pool: web::Data<PgPool>,
) -> impl Responder {
    if user.role != "admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "You do not have permission to access this resource."}));
    }

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1).min(100);
    let offset = (page - 1) * limit;

    let total_items: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool.as_ref())
        .await
    {
        Ok(count) => count,
        Err(e) => {
            log::error!("Failed to count users for admin view: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to retrieve user list."}));
        }
    };

    let query_str = r#"
        SELECT
            u.id,
            u.email,
            u.role,
            u.org_id,
            o.name as organization_name,
            u.confirmed,
            u.is_active,
            u.deactivated_at,
            u.created_at
        FROM
            users u
        LEFT JOIN
            organizations o ON u.org_id = o.id
        ORDER BY
            u.email ASC
        LIMIT $1 OFFSET $2
    "#;

    match sqlx::query_as::<_, AdminUserView>(query_str)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(users) => {
            let total_pages = (total_items as f64 / limit as f64).ceil() as i64;
            HttpResponse::Ok().json(PaginatedUsers {
                items: users,
                total_items,
                page,
                per_page: limit,
                total_pages,
            })
        }
        Err(e) => {
            log::error!("Failed to fetch all users for admin view: {:?}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Failed to retrieve user list."}))
        }
    }
}

#[post("/admin/users/{user_id}/assign_role")]
pub async fn assign_user_role(
    path_params: web::Path<Uuid>,
    payload: web::Json<AssignRolePayload>,
    current_admin_user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let target_user_id = path_params.into_inner();

    if current_admin_user.role != "admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Only global administrators can assign roles."}));
    }

    if target_user_id == current_admin_user.user_id {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Administrators cannot change their own role using this endpoint."}));
    }

    let new_role = payload.role.trim().to_lowercase();
    if new_role != "user" && new_role != "org_admin" {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Invalid role specified. Allowed roles are 'user' or 'org_admin'."}));
    }

    if new_role == "org_admin" {
        match payload.org_id {
            Some(org_uuid) => {
                match sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)")
                    .bind(org_uuid)
                    .fetch_one(pool.as_ref())
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => {
                        return HttpResponse::BadRequest()
                            .json(serde_json::json!({"error": format!("Organization with ID {} not found.", org_uuid)}));
                    }
                    Err(e) => {
                        log::error!("Failed to check existence of org_id {}: {:?}", org_uuid, e);
                        return HttpResponse::InternalServerError()
                            .json(serde_json::json!({"error": "Database error validating organization."}));
                    }
                }
            }
            None =>
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "An organization ID is required when assigning the 'org_admin' role."})),
        }
    }

    let target_user = match sqlx::query_as::<_, UserModel>("SELECT * FROM users WHERE id = $1")
        .bind(target_user_id)
        .fetch_optional(pool.as_ref())
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Target user not found."})),
        Err(e) => {
            log::error!("Failed to fetch target user {}: {:?}", target_user_id, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error fetching user."}));
        }
    };

    if target_user.role == "admin" && new_role != "admin" {
        match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE role = 'admin'")
            .fetch_one(pool.as_ref())
            .await
        {
            Ok(admin_count) if admin_count <= 1 => {
                return HttpResponse::Forbidden()
                    .json(serde_json::json!({"error": "Cannot remove the last global administrator's 'admin' role."}));
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("Failed to count admin users: {:?}", e);
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Database error during safety check."}));
            }
        }
    }

    let final_org_id_to_set = match payload.org_id {
        Some(new_org_uuid) => {
            if new_role != "org_admin" {
                match sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM organizations WHERE id = $1)")
                    .bind(new_org_uuid)
                    .fetch_one(pool.as_ref())
                    .await
                {
                    Ok(exists) if exists => new_org_uuid,
                    Ok(_) => {
                        log::warn!("Invalid org_id {} provided for user role update when role is 'user'.", new_org_uuid);
                        return HttpResponse::BadRequest()
                            .json(serde_json::json!({"error": format!("Organization with ID {} not found.", new_org_uuid)}));
                    }
                    Err(e) => {
                        log::error!("Failed to check existence of org_id {} for user role update: {:?}", new_org_uuid, e);
                        return HttpResponse::InternalServerError()
                            .json(serde_json::json!({"error": "Database error validating organization."}));
                    }
                }
            } else {
                new_org_uuid
            }
        }
        None => target_user.org_id,
    };

    let update_query = "UPDATE users SET role = $1, org_id = $2 WHERE id = $3";
    match sqlx::query(update_query)
        .bind(new_role.clone())
        .bind(final_org_id_to_set)
        .bind(target_user_id)
        .execute(pool.as_ref())
        .await
    {
        Ok(result) if result.rows_affected() > 0 => {
            log::info!("User {} role updated to {} for org_id {}. Action by admin: {}",
                target_user_id, new_role, final_org_id_to_set, current_admin_user.user_id);
            HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User role updated successfully."}))
        }
        Ok(_) => {
            log::warn!("User {} role update to {} for org_id {} attempted by admin {}, but no rows affected (user might not exist or data was the same).",
                target_user_id, new_role, final_org_id_to_set, current_admin_user.user_id);
            HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User role and org assignment are already up to date or user not found during update."}))
        }
        Err(e) => {
            log::error!("Failed to update user {} role: {:?}", target_user_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error updating user role."}))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct UpdateUserProfilePayload {
    pub email: Option<String>,
}

#[put("/admin/users/{user_id}/profile")]
pub async fn update_user_profile(
    path_params: web::Path<Uuid>,
    payload: web::Json<UpdateUserProfilePayload>,
    current_admin_user: AuthUser,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let target_user_id = path_params.into_inner();

    if current_admin_user.role != "admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Only global administrators can update user profiles."}));
    }

    let target_user = match UserModel::find_by_id_for_admin(&pool, target_user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Target user not found."})),
        Err(e) => {
            log::error!("Failed to fetch target user {} for profile update: {:?}", target_user_id, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error fetching user."}));
        }
    };

    if let Some(new_email_str) = &payload.email {
        let trimmed_new_email = new_email_str.trim();
        if trimmed_new_email.is_empty() {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Email cannot be empty."}));
        }
        if !trimmed_new_email.contains('@') {
             return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid email format."}));
        }

        if trimmed_new_email.to_lowercase() != target_user.email.to_lowercase() {
            match UserModel::find_by_email(&pool, trimmed_new_email).await {
                Ok(_existing_user) => return HttpResponse::Conflict().json(serde_json::json!({"error": "New email address is already in use."})),
                Err(sqlx::Error::RowNotFound) => {}
                Err(e) => {
                    log::error!("DB error checking new email existence for user {}: {:?}", target_user_id, e);
                    return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error checking email availability."}));
                }
            }

            let new_confirmation_token = Uuid::new_v4();
            match UserModel::update_email_and_set_unconfirmed(&pool, target_user_id, trimmed_new_email, new_confirmation_token).await {
                Ok(rows_affected) if rows_affected > 0 => {
                    log::info!("Admin {} updated email for user {} to {}. Marked unconfirmed.", current_admin_user.user_id, target_user_id, trimmed_new_email);

                    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
                    let confirmation_link = format!("{}/api/confirm/{}", base_url, new_confirmation_token);
                    let email_subject = "Your Email Address Was Changed - Confirm New Email";
                    let email_body = format!(
                        r#"Hello,

Your email address associated with crPipeline was changed by an administrator to {}.
Please confirm this new email address by clicking the link below:
{}

If you did not request this change, please contact support immediately."#,
                        trimmed_new_email, confirmation_link);

                    if let Err(e) = enqueue_email(trimmed_new_email, email_subject, &email_body).await {
                        log::error!("Failed to send confirmation email to new address {}: {:?}", trimmed_new_email, e);
                        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Profile email updated, but failed to send confirmation to new email. User must confirm via other means or contact support."}));
                    }
                    HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "User email updated. A confirmation email has been sent to the new address."}))
                }
                Ok(_) => {
                    log::error!("Failed to update email for user {} (0 rows affected). User ID might be wrong or race condition.", target_user_id);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to update user email."}))
                }
                Err(e) => {
                    log::error!("Database error updating email for user {}: {:?}", target_user_id, e);
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error updating email."}))
                }
            }
        } else {
            HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "No changes to email address."}))
        }
    } else {
        HttpResponse::Ok().json(serde_json::json!({"success": true, "message": "No email provided for update."}))
    }
}

#[post("/admin/users/{user_id}/resend_confirmation")]
pub async fn resend_confirmation_email(
    admin: AuthUser,
    path: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if admin.role != "admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Only global administrators can resend confirmations."}));
    }
    let user_id = path.into_inner();
    let target_user = match UserModel::find_by_id_for_admin(&pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "User not found."})),
        Err(e) => {
            log::error!("Failed to fetch user {} for resend confirmation: {:?}", user_id, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error."}));
        }
    };
    if target_user.confirmed {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "User is already confirmed."}));
    }
    let new_token = Uuid::new_v4();
    match sqlx::query("UPDATE users SET confirmed=false, confirmation_token=$1 WHERE id=$2")
        .bind(new_token)
        .bind(user_id)
        .execute(pool.as_ref())
        .await
    {
        Ok(res) if res.rows_affected() > 0 => {
            let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
            let confirmation_link = format!("{}/api/confirm/{}", base_url, new_token);
            let subject = "Confirm your email";
            let body = format!(
                "Hello {},\n\nPlease confirm your email by clicking the link below:\n{}\n",
                target_user.email, confirmation_link
            );
            if let Err(e) = enqueue_email(&target_user.email, subject, &body).await {
                log::error!("Failed to send confirmation email to {}: {:?}", target_user.email, e);
                HttpResponse::InternalServerError().json(serde_json::json!({"error": "Token updated but email failed."}))
            } else {
                HttpResponse::Ok().json(serde_json::json!({"success": true}))
            }
        }
        Ok(_) => HttpResponse::InternalServerError().json(serde_json::json!({"error": "No rows updated."})),
        Err(e) => {
            log::error!("DB error updating confirmation token for {}: {:?}", user_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error."}))
        }
    }
}

#[post("/admin/users/{user_id}/deactivate")]
pub async fn deactivate_user(
    admin: AuthUser,
    path: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if admin.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only global administrators can perform this action."}));
    }
    let user_id = path.into_inner();
    match sqlx::query("UPDATE users SET is_active=false, deactivated_at=NOW() WHERE id=$1 AND is_active=true")
        .bind(user_id)
        .execute(pool.as_ref())
        .await
    {
        Ok(res) if res.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(_) => HttpResponse::BadRequest().json(serde_json::json!({"error": "User already deactivated or not found."})),
        Err(e) => {
            log::error!("Failed to deactivate user {}: {:?}", user_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error."}))
        }
    }
}

#[post("/admin/users/{user_id}/reactivate")]
pub async fn reactivate_user(
    admin: AuthUser,
    path: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    if admin.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Only global administrators can perform this action."}));
    }
    let user_id = path.into_inner();
    match sqlx::query("UPDATE users SET is_active=true, deactivated_at=NULL WHERE id=$1 AND is_active=false")
        .bind(user_id)
        .execute(pool.as_ref())
        .await
    {
        Ok(res) if res.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({"success": true})),
        Ok(_) => HttpResponse::BadRequest().json(serde_json::json!({"error": "User already active or not found."})),
        Err(e) => {
            log::error!("Failed to reactivate user {}: {:?}", user_id, e);
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Database error."}))
        }
    }
}

