use once_cell::sync::Lazy;
use prometheus::{IntCounterVec, Opts, Registry};

pub static AUTH_FAILURE_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("login_failures_total", "Total failed login attempts");
    IntCounterVec::new(opts, &["reason"]).unwrap()
});

pub fn register_metrics(registry: &Registry) {
    registry
        .register(Box::new(AUTH_FAILURE_COUNTER.clone()))
        .unwrap();
}
