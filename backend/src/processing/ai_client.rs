use std::time::Duration;
use reqwest::header::{HeaderName, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
struct CustomHeader {
    name: String,
    value: String,
}

#[derive(Debug)]
pub enum AiClientError {
    Request(reqwest::Error),
    CloneError,
    HttpError(reqwest::StatusCode, String),
}

impl std::fmt::Display for AiClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiClientError::Request(e) => write!(f, "request error: {e}"),
            AiClientError::CloneError => write!(f, "failed to clone request"),
            AiClientError::HttpError(status, msg) => {
                write!(f, "http error {status}: {msg}")
            }
        }
    }
}

impl std::error::Error for AiClientError {}

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 500;

fn exponential_backoff(attempt: u32) -> Duration {
    Duration::from_millis(BASE_DELAY_MS * (1 << (attempt - 1)))
}

#[tracing::instrument(skip(input, custom_headers_json))]
pub async fn run_ai(
    input: &serde_json::Value,
    api_endpoint: &str,
    api_key: &str,
    custom_headers_json: Option<&serde_json::Value>,
) -> Result<serde_json::Value, AiClientError> {
    let client = reqwest::Client::new();
    let mut request_builder = client.post(api_endpoint).timeout(Duration::from_secs(10));
    if !api_key.is_empty() {
        request_builder = request_builder.bearer_auth(api_key);
    }
    if let Some(headers_val) = custom_headers_json {
        if headers_val.is_array() {
            match serde_json::from_value::<Vec<CustomHeader>>(headers_val.clone()) {
                Ok(headers_vec) => {
                    for header_obj in headers_vec {
                        if header_obj.name.trim().is_empty() {
                            log::warn!("Skipping custom AI header with empty name: {:?}", header_obj);
                            continue;
                        }
                        match HeaderName::from_bytes(header_obj.name.as_bytes()) {
                            Ok(header_name) => match HeaderValue::from_str(&header_obj.value) {
                                Ok(header_value) => {
                                    request_builder = request_builder.header(header_name, header_value);
                                }
                                Err(e) => {
                                    log::warn!("Invalid custom AI header value for '{}': {:?}. Skipping.", header_obj.name, e);
                                }
                            },
                            Err(e) => {
                                log::warn!("Invalid custom AI header name '{}': {:?}. Skipping.", header_obj.name, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to deserialize ai_custom_headers JSON into Vec<CustomHeader>: {:?}. Headers JSON: {}", e, headers_val);
                }
            }
        } else if !headers_val.is_null() {
            log::warn!("ai_custom_headers is not an array, skipping. Headers JSON: {}", headers_val);
        }
    }
    request_builder = request_builder.header(CONTENT_TYPE, "application/json");
    let mut attempts = 0;
    loop {
        let builder = request_builder
            .try_clone()
            .ok_or(AiClientError::CloneError)?
            .json(input);
        match builder.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    return resp.json().await.map_err(AiClientError::Request);
                } else if attempts < MAX_RETRIES {
                    attempts += 1;
                    let status = resp.status();
                    let msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "unknown".into());
                    log::warn!("AI request failed status {} attempt {}: {}", status, attempts, msg);
                    tokio::time::sleep(exponential_backoff(attempts)).await;
                } else {
                    let status = resp.status();
                    let msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "unknown".into());
                    return Err(AiClientError::HttpError(status, msg));
                }
            }
            Err(e) => {
                if attempts < MAX_RETRIES {
                    attempts += 1;
                    log::warn!("AI request error attempt {}: {:?}", attempts, e);
                    tokio::time::sleep(exponential_backoff(attempts)).await;
                } else {
                    return Err(AiClientError::Request(e));
                }
            }
        }
    }
}
