use once_cell::sync::Lazy;
use prometheus::{HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, Registry};

pub static AUTH_FAILURE_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("login_failures_total", "Total failed login attempts");
    IntCounterVec::new(opts, &["reason"]).unwrap()
});

pub static RATE_LIMIT_FALLBACK_COUNTER: Lazy<IntCounter> = Lazy::new(|| {
    let opts = Opts::new(
        "rate_limit_fallback_total",
        "Number of times Redis rate limiting failed and fallback was used",
    );
    IntCounter::with_opts(opts).unwrap()
});

pub static STAGE_HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = HistogramOpts::new(
        "stage_duration_seconds",
        "Time spent processing each stage",
    );
    HistogramVec::new(opts, &["stage"]).unwrap()
});

pub static JOB_HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = HistogramOpts::new(
        "job_duration_seconds",
        "Total time spent processing a job",
    );
    HistogramVec::new(opts, &["status"]).unwrap()
});

pub static REQUEST_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new(
        "http_requests_total",
        "Total number of HTTP requests by endpoint",
    );
    IntCounterVec::new(opts, &["method", "endpoint", "status"]).unwrap()
});

pub static REQUEST_HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = HistogramOpts::new(
        "http_request_duration_seconds",
        "HTTP request duration by endpoint",
    );
    HistogramVec::new(opts, &["method", "endpoint", "status"]).unwrap()
});

pub fn register_metrics(registry: &Registry) {
    registry.register(Box::new(AUTH_FAILURE_COUNTER.clone())).unwrap();
    registry
        .register(Box::new(RATE_LIMIT_FALLBACK_COUNTER.clone()))
        .unwrap();
    registry
        .register(Box::new(STAGE_HISTOGRAM.clone()))
        .unwrap();
    registry.register(Box::new(JOB_HISTOGRAM.clone())).unwrap();
    registry.register(Box::new(REQUEST_COUNTER.clone())).unwrap();
    registry.register(Box::new(REQUEST_HISTOGRAM.clone())).unwrap();
}
