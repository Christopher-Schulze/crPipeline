use actix_web::web;

pub mod user_management;
pub mod invites;

pub use user_management::{
    list_all_users,
    assign_user_role,
    update_user_profile,
    resend_confirmation_email,
    deactivate_user,
    reactivate_user,
};

pub use invites::invite_user;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_all_users)
        .service(assign_user_role)
        .service(resend_confirmation_email)
        .service(deactivate_user)
        .service(reactivate_user)
        .service(invite_user)
        .service(update_user_profile);
}
