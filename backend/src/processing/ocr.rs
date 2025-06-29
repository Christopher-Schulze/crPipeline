use std::path::Path;
use aws_sdk_s3::Client as S3Client;
use anyhow::{Result, anyhow, Context};
use reqwest::header::{HeaderValue, AUTHORIZATION};
use reqwest::multipart;
use tokio::process::Command;
use log;

pub async fn download_pdf(s3: &S3Client, bucket: &str, key: &str, path: &Path) -> Result<()> {
    let obj = s3.get_object().bucket(bucket).key(key).send().await?;
    let bytes = obj.body.collect().await?.into_bytes();
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

pub async fn run_external_ocr(
    api_endpoint: &str,
    api_key: Option<&str>,
    file_bytes: Vec<u8>,
    original_filename: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let file_part = multipart::Part::bytes(file_bytes)
        .file_name(original_filename.to_string())
        .mime_str("application/pdf")
        .context("Failed to create file part for external OCR")?;

    let form = multipart::Form::new().part("file", file_part);

    let mut request_builder = client.post(api_endpoint).multipart(form);

    if let Some(key_str) = api_key.filter(|k| !k.trim().is_empty()) {
        if key_str.to_lowercase().starts_with("bearer ") {
            let token_part = key_str.split_at("bearer ".len()).1;
            match HeaderValue::from_str(token_part) {
                Ok(mut header_val) => {
                    header_val.set_sensitive(true);
                    request_builder = request_builder.header(AUTHORIZATION, header_val);
                    log::debug!("Added Bearer token to external OCR request.");
                }
                Err(e) => {
                    log::warn!("Invalid characters in Bearer token for external OCR: {:?}. Authorization header not sent.", e);
                }
            }
        } else {
            match HeaderValue::from_str(key_str) {
                Ok(mut header_val) => {
                    header_val.set_sensitive(true);
                    request_builder = request_builder.header("X-API-KEY", header_val);
                    log::debug!("Added X-API-KEY to external OCR request.");
                }
                Err(e) => {
                    log::warn!("Invalid characters in X-API-KEY for external OCR: {:?}. Header not sent.", e);
                }
            }
        }
    }

    log::debug!("Sending file {} to external OCR API: {}", original_filename, api_endpoint);
    let response = request_builder.send().await
        .with_context(|| format!("External OCR API request to {} failed to send", api_endpoint))?;

    if response.status().is_success() {
        let ocr_text = response.text().await
            .context("Failed to read text response from external OCR API")?;
        Ok(ocr_text)
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error content from OCR API".to_string());
        log::error!("External OCR API request to {} failed with status {}: {}", api_endpoint, status, error_text);
        Err(anyhow!("External OCR API request failed with status {}: {}", status, error_text))
    }
}

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
