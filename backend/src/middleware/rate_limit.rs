use actix_service::{Service, Transform, forward_ready};
use actix_web::{dev::{ServiceRequest, ServiceResponse}, Error};
use futures_util::future::{LocalBoxFuture, ready, Ready};
use std::rc::Rc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use once_cell::sync::Lazy;

static LIMITS: Lazy<DashMap<String, (u32, Instant)>> = Lazy::new(DashMap::new);
const WINDOW: Duration = Duration::from_secs(60);
const MAX_REQ: u32 = 100;

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
        ready(Ok(RateLimitMiddleware { service: Rc::new(service) }))
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
        let key = req.connection_info().realip_remote_addr().unwrap_or("unknown").to_string();
        let mut entry = LIMITS.entry(key).or_insert((0, Instant::now()));
        if entry.1.elapsed() > WINDOW {
            *entry = (1, Instant::now());
        } else {
            entry.0 += 1;
        }
        if entry.0 > MAX_REQ {
            let fut = async move { Err(actix_web::error::ErrorTooManyRequests("rate limit")) };
            return Box::pin(fut);
        }
        let srv = Rc::clone(&self.service);
        Box::pin(async move { srv.call(req).await })
    }
}
