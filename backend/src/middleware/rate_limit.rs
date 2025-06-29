use actix_service::{forward_ready, Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error,
};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use std::env;
use std::rc::Rc;

use crate::middleware::jwt::verify_jwt; // Assuming Claims is pub
use actix_web::http::header::{HeaderName, AUTHORIZATION};
use dashmap::DashMap;
use log::{error, warn}; // For logging
use once_cell::sync::Lazy;
use redis::{AsyncCommands, Client as RedisClient};
use std::time::{Duration, Instant};

// Define constants for rate limiting
const WINDOW_SECONDS: u64 = 60; // 1 minute window
const MAX_REQUESTS: u32 = 100; // Max requests per window
const X_API_KEY_HEADER: HeaderName = HeaderName::from_static("x-api-key");

static MEMORY_LIMITS: Lazy<DashMap<String, (u32, Instant)>> = Lazy::new(DashMap::new);

fn check_memory_limit(key: &str) -> bool {
    let now = Instant::now();
    let mut entry = MEMORY_LIMITS.entry(key.to_string()).or_insert((0, now));
    if now.duration_since(entry.1) > Duration::from_secs(WINDOW_SECONDS) {
        *entry = (1, now);
        true
    } else {
        entry.0 += 1;
        entry.0 <= MAX_REQUESTS
    }
}

fn fallback_mode() -> String {
    env::var("REDIS_RATE_LIMIT_FALLBACK")
        .unwrap_or_else(|_| "memory".into())
        .to_lowercase()
}

pub struct RateLimit;

impl<S, B> Transform<S, ServiceRequest> for RateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RateLimitMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);

        Box::pin(async move {
            let mut identifier_key: Option<String> = None;

            // 1. Try to extract org_id from JWT
            let mut jwt_token_str: Option<String> = None;

            // Try Authorization header first
            if let Some(auth_header) = req.headers().get(AUTHORIZATION) {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        jwt_token_str = Some(auth_str[7..].to_string());
                    }
                }
            }

            // If not in Authorization header, try 'token' cookie
            if jwt_token_str.is_none() {
                if let Some(cookie) = req.request().cookie("token") {
                    // Use req.request().cookie()
                    jwt_token_str = Some(cookie.value().to_string());
                }
            }

            if let Some(token_str) = jwt_token_str {
                if let Some(claims) = verify_jwt(&token_str) {
                    // verify_jwt uses env var for secret
                    identifier_key = Some(format!("rate_limit:org_id:{}", claims.org));
                }
            }

            // 2. If no org_id from JWT, try X-API-Key header
            if identifier_key.is_none() {
                if let Some(api_key_value) = req.headers().get(&X_API_KEY_HEADER) {
                    if let Ok(api_key_str) = api_key_value.to_str() {
                        if !api_key_str.is_empty() {
                            identifier_key = Some(format!("rate_limit:api_key:{}", api_key_str));
                        }
                    }
                }
            }

            if let Some(key) = identifier_key {
                let mut redis_failure = false;
                let mut exceeded = false;

                if let Ok(redis_url) = env::var("REDIS_URL") {
                    if let Ok(client) = RedisClient::open(redis_url) {
                        match client.get_async_connection().await {
                            Ok(mut conn) => {
                                let count_res: redis::RedisResult<u32> =
                                    conn.incr(&key, 1i32).await;
                                match count_res {
                                    Ok(count) => {
                                        if count == 1 {
                                            let _: redis::RedisResult<()> =
                                                conn.expire(&key, WINDOW_SECONDS as i64).await;
                                        }
                                        if count > MAX_REQUESTS {
                                            exceeded = true;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Redis INCR failed for key {}: {}", key, e);
                                        redis_failure = true;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Redis connection error: {}", e);
                                redis_failure = true;
                            }
                        }
                    } else {
                        error!("Failed to create Redis client");
                        redis_failure = true;
                    }
                } else {
                    error!("REDIS_URL not set");
                    redis_failure = true;
                }

                if redis_failure {
                    match fallback_mode().as_str() {
                        "deny" => {
                            return Err(actix_web::error::ErrorTooManyRequests(
                                "Too many requests",
                            ));
                        }
                        "memory" => {
                            if !check_memory_limit(&key) {
                                return Err(actix_web::error::ErrorTooManyRequests(
                                    "Too many requests",
                                ));
                            }
                        }
                        _ => {}
                    }
                } else if exceeded {
                    error!("Rate limit exceeded for key: {}", key);
                    return Err(actix_web::error::ErrorTooManyRequests("Too many requests"));
                }
            } else {
                // No identifier found, allow request (or apply global limit if desired later)
                warn!("No JWT org_id or API key found for rate limiting. Request allowed.");
            }

            // Proceed with the request if not rate-limited
            srv.call(req).await
        })
    }
}
