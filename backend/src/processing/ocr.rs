use std::time::Duration;
use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use crate::worker::metrics::S3_ERROR_COUNTER;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use reqwest::multipart;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug)]
pub enum OcrError {
    Request(reqwest::Error),
    HttpError(reqwest::StatusCode, String),
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrError::Request(e) => write!(f, "request error: {e}"),
            OcrError::HttpError(status, msg) => write!(f, "http error {status}: {msg}"),
        }
    }
}

impl std::error::Error for OcrError {}

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 500;

fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(BASE_DELAY_MS * (1 << (attempt - 1)))
}

/// Download a PDF from S3 (or from `LOCAL_S3_DIR` when set) and write it to `path`.
///
/// * `s3` - AWS S3 client for fetching the object.
/// * `bucket` - Source bucket name.
/// * `key` - Key of the object to download.
/// * `path` - Local destination for the file.
pub async fn download_pdf(s3: &S3Client, bucket: &str, key: &str, path: &Path) -> Result<()> {
    if let Ok(local_dir) = std::env::var("LOCAL_S3_DIR") {
        let local_path = Path::new(&local_dir).join(key);
        tokio::fs::copy(&local_path, path).await?;
        return Ok(());
    }

    let obj = match s3.get_object().bucket(bucket).key(key).send().await {
        Ok(o) => o,
        Err(e) => {
            S3_ERROR_COUNTER.with_label_values(&["download"]).inc();
            return Err(e.into());
        }
    };
    let bytes = match obj.body.collect().await {
        Ok(b) => b.into_bytes(),
        Err(e) => {
            S3_ERROR_COUNTER.with_label_values(&["download"]).inc();
            return Err(e.into());
        }
    };
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

/// Send a PDF to an external OCR service and return the resulting text.
#[tracing::instrument(skip(file_bytes))]
pub async fn run_external_ocr(
    api_endpoint: &str,
    api_key: Option<&str>,
    file_bytes: Vec<u8>,
    original_filename: &str,
) -> Result<String, OcrError> {
    let client = reqwest::Client::new();
    let mut attempts = 0;
    loop {
        let file_part = multipart::Part::bytes(file_bytes.clone())
            .file_name(original_filename.to_string())
            .mime_str("application/pdf")
            .map_err(OcrError::Request)?;
        let mut request_builder = client
            .post(api_endpoint)
            .timeout(Duration::from_secs(10))
            .multipart(multipart::Form::new().part("file", file_part));
        if let Some(key_str) = api_key.filter(|k| !k.trim().is_empty()) {
            if key_str.to_lowercase().starts_with("bearer ") {
                let token_part = key_str.split_at("bearer ".len()).1;
                if let Ok(mut header_val) = HeaderValue::from_str(token_part) {
                    header_val.set_sensitive(true);
                    request_builder = request_builder.header(AUTHORIZATION, header_val);
                }
            } else if let Ok(mut header_val) = HeaderValue::from_str(key_str) {
                header_val.set_sensitive(true);
                request_builder = request_builder.header("X-API-KEY", header_val);
            }
        }
        match request_builder.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    return resp.text().await.map_err(OcrError::Request);
                } else if attempts < MAX_RETRIES {
                    attempts += 1;
                    let status = resp.status();
                    let msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "unknown".into());
                    log::warn!("OCR request failed status {} attempt {}: {}", status, attempts, msg);
                    tokio::time::sleep(backoff(attempts)).await;
                } else {
                    let status = resp.status();
                    let msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "unknown".into());
                    return Err(OcrError::HttpError(status, msg));
                }
            }
            Err(e) => {
                if attempts < MAX_RETRIES {
                    attempts += 1;
                    log::warn!("OCR request error attempt {}: {:?}", attempts, e);
                    tokio::time::sleep(backoff(attempts)).await;
                } else {
                    return Err(OcrError::Request(e));
                }
            }
        }
    }
}

/// Run the Tesseract OCR command locally.
pub async fn run_ocr(input: &Path, output: &Path) -> Result<()> {
    let status = Command::new("tesseract")
        .arg(input)
        .arg(output)
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("tesseract failed");
    }
    Ok(())
}
