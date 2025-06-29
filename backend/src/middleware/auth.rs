use actix_web::{dev::Payload, error::ErrorUnauthorized, FromRequest, HttpRequest};
use futures_util::future::{ready, Ready};
use uuid::Uuid;

use crate::middleware::jwt::verify_jwt;

pub struct AuthUser {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub role: String,
}

impl FromRequest for AuthUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // First try Authorization header
        if let Some(auth) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    if let Some(claims) = verify_jwt(token) {
                        return ready(Ok(AuthUser {
                            user_id: claims.sub,
                            org_id: claims.org,
                            role: claims.role,
                        }));
                    }
                }
            }
        }

        // Fallback to cookie based authentication
        if let Some(cookie) = req.cookie("token") {
            if let Some(claims) = verify_jwt(cookie.value()) {
                return ready(Ok(AuthUser {
                    user_id: claims.sub,
                    org_id: claims.org,
                    role: claims.role,
                }));
            }
        }

        ready(Err(ErrorUnauthorized("Unauthorized")))
    }
}
