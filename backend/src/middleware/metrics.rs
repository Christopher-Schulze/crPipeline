use actix_service::{forward_ready, Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::Error;
use futures_util::future::{ready, LocalBoxFuture, Ready};
use std::rc::Rc;
use std::time::Instant;

use crate::metrics::{HTTP_REQUEST_COUNTER, HTTP_REQUEST_HISTOGRAM};

pub struct Metrics;

impl<S, B> Transform<S, ServiceRequest> for Metrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = MetricsMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MetricsMiddleware { service: Rc::new(service) }))
    }
}

pub struct MetricsMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for MetricsMiddleware<S>
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
        let method = req.method().clone();
        let endpoint = req
            .match_pattern()
            .map(|s| s.to_string())
            .unwrap_or_else(|| req.path().to_string());
        Box::pin(async move {
            let start = Instant::now();
            let res = srv.call(req).await?;
            let duration = start.elapsed().as_secs_f64();
            let status = res.status().as_u16().to_string();
            HTTP_REQUEST_COUNTER
                .with_label_values(&[method.as_str(), &endpoint, &status])
                .inc();
            HTTP_REQUEST_HISTOGRAM
                .with_label_values(&[method.as_str(), &endpoint, &status])
                .observe(duration);
            Ok(res)
        })
    }
}
