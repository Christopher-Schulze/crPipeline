use actix_service::{forward_ready, Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::Error;
use futures_util::future::{ready, LocalBoxFuture, Ready};
use std::rc::Rc;
use std::time::Instant;

use crate::metrics::{REQUEST_COUNTER, REQUEST_HISTOGRAM};

pub struct RequestMetrics;

impl<S, B> Transform<S, ServiceRequest> for RequestMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestMetricsMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestMetricsMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RequestMetricsMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestMetricsMiddleware<S>
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
        let path = req
            .match_pattern()
            .map(|s| s.to_string())
            .unwrap_or_else(|| req.path().to_string());
        let method = req.method().as_str().to_string();
        let start = Instant::now();

        Box::pin(async move {
            let res = srv.call(req).await?;
            let status = res.status().as_u16().to_string();
            let elapsed = start.elapsed().as_secs_f64();
            REQUEST_COUNTER
                .with_label_values(&[&method, &path, &status])
                .inc();
            REQUEST_HISTOGRAM
                .with_label_values(&[&method, &path, &status])
                .observe(elapsed);
            Ok(res)
        })
    }
}

