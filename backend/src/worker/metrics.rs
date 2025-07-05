use actix_web::{web, App, HttpResponse, HttpServer};
use once_cell::sync::Lazy;
use prometheus::{
    Encoder, HistogramVec, IntCounterVec, IntCounter, Registry, TextEncoder,
};

pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

pub static STAGE_HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = prometheus::HistogramOpts::new(
        "stage_duration_seconds",
        "Time spent processing each stage",
    );
    let hist = HistogramVec::new(opts, &["stage"]).unwrap();
    REGISTRY.register(Box::new(hist.clone())).unwrap();
    hist
});

pub static JOB_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = prometheus::Opts::new("jobs_total", "Total jobs processed");
    let counter = IntCounterVec::new(opts, &["status"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

pub static JOB_HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = prometheus::HistogramOpts::new(
        "job_duration_seconds",
        "Total time spent processing a job",
    );
    let hist = HistogramVec::new(opts, &["status"]).unwrap();
    REGISTRY.register(Box::new(hist.clone())).unwrap();
    hist
});

pub static S3_ERROR_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = prometheus::Opts::new("s3_errors_total", "Total number of S3 errors");
    let counter = IntCounterVec::new(opts, &["operation"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

pub static OCR_HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    let opts = prometheus::HistogramOpts::new(
        "ocr_duration_seconds",
        "Time spent performing OCR",
    );
    let hist = HistogramVec::new(opts, &["engine"]).unwrap();
    REGISTRY.register(Box::new(hist.clone())).unwrap();
    hist
});

pub static API_ERROR_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = prometheus::Opts::new(
        "ai_ocr_errors_total",
        "Total failed AI or OCR API calls",
    );
    let counter = IntCounterVec::new(opts, &["service"]).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

pub static WORKER_SHUTDOWN_COUNTER: Lazy<IntCounter> = Lazy::new(|| {
    let opts = prometheus::Opts::new(
        "worker_shutdowns_total",
        "Total number of worker shutdowns",
    );
    let counter = IntCounter::with_opts(opts).unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

async fn metrics() -> HttpResponse {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    HttpResponse::Ok()
        .content_type(encoder.format_type())
        .body(buffer)
}

pub fn spawn_metrics_server(port: u16) {
    tokio::spawn(async move {
        if let Err(e) = HttpServer::new(|| App::new().route("/metrics", web::get().to(metrics)))
            .bind(("0.0.0.0", port))
            .unwrap()
            .run()
            .await
        {
            eprintln!("failed to start metrics server: {:?}", e);
        }
    });
}
