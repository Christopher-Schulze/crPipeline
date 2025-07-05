use once_cell::sync::Lazy;
use prometheus::{IntCounter, IntCounterVec, Opts, Registry};

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

pub fn register_metrics(registry: &Registry) {
    registry
        .register(Box::new(AUTH_FAILURE_COUNTER.clone()))
        .unwrap();
    registry
        .register(Box::new(RATE_LIMIT_FALLBACK_COUNTER.clone()))
        .unwrap();
}
