use anyhow::{anyhow, Result};
use reqwest::header::{HeaderName, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
struct CustomHeader {
    name: String,
    value: String,
}

#[tracing::instrument(skip(input, custom_headers_json))]
pub async fn run_ai(
    input: &serde_json::Value,
    api_endpoint: &str,
    api_key: &str,
    custom_headers_json: Option<&serde_json::Value>,
) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let mut request_builder = client.post(api_endpoint);
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
    let response = loop {
        let builder = match request_builder.try_clone() {
            Some(b) => b,
            None => return Err(anyhow!("Failed to clone request builder")),
        };
        match builder.json(input).send().await {
            Ok(resp) => break resp,
            Err(e) if attempts < 3 => {
                attempts += 1;
                log::warn!("AI request failed attempt {}: {:?}", attempts, e);
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempts as u64)).await;
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    };
    if response.status().is_success() {
        Ok(response.json().await?)
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error content".to_string());
        log::error!(
            "AI API request to {} failed with status {}: {}",
            api_endpoint,
            status,
            error_text
        );
        Err(anyhow!(
            "AI API request failed with status {}: {}",
            status,
            error_text
        ))
    }
}
