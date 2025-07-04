use actix_service::{Service, Transform, forward_ready};
use actix_web::{dev::{ServiceRequest, ServiceResponse}, Error, http::header::HeaderName};
use futures_util::future::{LocalBoxFuture, ready, Ready};
use once_cell::sync::Lazy;
use std::{env, rc::Rc};

static CSRF_TOKEN: Lazy<Option<String>> = Lazy::new(|| env::var("CSRF_TOKEN").ok());
const CSRF_HEADER: HeaderName = HeaderName::from_static("x-csrf-token");

pub fn init_csrf_token() {
    Lazy::force(&CSRF_TOKEN);
}

pub struct CsrfCheck;

impl<S, B> Transform<S, ServiceRequest> for CsrfCheck
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = CsrfCheckMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CsrfCheckMiddleware { service: Rc::new(service) }))
    }
}

pub struct CsrfCheckMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for CsrfCheckMiddleware<S>
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
        let token = req
            .headers()
            .get(&CSRF_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Box::pin(async move {
            if let Some(expected) = CSRF_TOKEN.as_ref() {
                if token.as_deref() != Some(expected.as_str()) {
                    return Err(actix_web::error::ErrorForbidden("Invalid CSRF token"));
                }
            }
            srv.call(req).await
        })
    }
}
