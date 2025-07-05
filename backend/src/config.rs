use dotenvy::dotenv;
use std::env;

pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub s3_bucket: String,
    pub frontend_origin: String,
    pub email_queue_provider: String,
    pub email_queue_size: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        dotenv().ok();
        let database_url =
            env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set".to_string())?;
        let jwt_secret = env::var("JWT_SECRET").map_err(|_| "JWT_SECRET not set".to_string())?;
        if jwt_secret.len() < 32 || jwt_secret == "changeme" {
            return Err("JWT_SECRET must be at least 32 characters and not 'changeme'".into());
        }
        let s3_bucket = env::var("S3_BUCKET").map_err(|_| "S3_BUCKET not set".to_string())?;
        let frontend_origin = env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "*".into());
        let email_queue_provider =
            env::var("EMAIL_QUEUE_PROVIDER").unwrap_or_else(|_| "memory".into());
        let email_queue_size = env::var("EMAIL_QUEUE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);
        Ok(Self {
            database_url,
            jwt_secret,
            s3_bucket,
            frontend_origin,
            email_queue_provider,
            email_queue_size,
        })
    }
}

pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub s3_bucket: String,
    pub process_one_job: bool,
    pub worker_concurrency: usize,
    pub metrics_port: u16,
    pub shutdown_after_idle: Option<u64>,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self, String> {
        dotenv().ok();
        let database_url =
            env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set".to_string())?;
        let redis_url = env::var("REDIS_URL").map_err(|_| "REDIS_URL not set".to_string())?;
        let s3_bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
        let process_one_job = env::var("PROCESS_ONE_JOB").is_ok();
        let worker_concurrency = env::var("WORKER_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let metrics_port = env::var("METRICS_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(9100);
        let shutdown_after_idle = env::var("SHUTDOWN_AFTER_IDLE")
            .ok()
            .and_then(|v| v.parse().ok());
        Ok(Self {
            database_url,
            redis_url,
            s3_bucket,
            process_one_job,
            worker_concurrency,
            metrics_port,
            shutdown_after_idle,
        })
    }
}

pub struct CleanupConfig {
    pub database_url: String,
    pub s3_bucket: String,
    pub interval_minutes: Option<u64>,
}

impl CleanupConfig {
    pub fn from_env() -> Result<Self, String> {
        dotenv().ok();
        let database_url =
            env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set".to_string())?;
        let s3_bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".into());
        let interval_minutes = env::var("CLEANUP_INTERVAL_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok());
        Ok(Self {
            database_url,
            s3_bucket,
            interval_minutes,
        })
    }
}

pub struct AdminConfig {
    pub database_url: String,
}

impl AdminConfig {
    pub fn from_env() -> Result<Self, String> {
        dotenv().ok();
        let database_url =
            env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set".to_string())?;
        Ok(Self { database_url })
    }
}
