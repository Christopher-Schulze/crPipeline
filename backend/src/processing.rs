use std::path::Path;
use std::fs::{File};
use std::io::BufWriter;
use aws_sdk_s3::Client as S3Client;
use anyhow::{Result, anyhow, Context};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value as JsonValue; // For new parse stage
use std::collections::HashMap; // For new parse stage
use regex::Regex; // For new parse stage
use reqwest::multipart; // Added for multipart form data
use printpdf::*;
use tokio::process::Command;
use serde::Deserialize; // For CustomHeader
use serde_json::Value; // For Value type hint
use log; // For logging
use uuid::Uuid;

// For new report generation
use pulldown_cmark::{Parser, Event, Tag, Options as MarkdownOptions, HeadingLevel};
use jsonpath_rust::JsonPath;
// printpdf types are already imported via printpdf::*

/// Download a PDF from S3 (or from `LOCAL_S3_DIR` when set) and write it to `path`.
///
/// * `s3` - AWS S3 client for fetching the object.
/// * `bucket` - Source bucket name.
/// * `key` - Key of the object to download.
/// * `path` - Local destination for the file.
///
/// An error is returned if the object cannot be retrieved or written.
pub async fn download_pdf(s3: &S3Client, bucket: &str, key: &str, path: &Path) -> Result<()> {
    if let Ok(local_dir) = std::env::var("LOCAL_S3_DIR") {
        let local_path = Path::new(&local_dir).join(key);
        tokio::fs::copy(&local_path, path).await?;
        return Ok(());
    }

    let obj = s3.get_object().bucket(bucket).key(key).send().await?;
    let bytes = obj.body.collect().await?.into_bytes();
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

/// Send a PDF to an external OCR service and return the resulting text.
///
/// * `api_endpoint` - URL of the OCR service.
/// * `api_key` - Optional API key or bearer token for authentication.
/// * `file_bytes` - Contents of the PDF to OCR.
/// * `original_filename` - File name sent to the service.
///
/// Errors are propagated for HTTP failures or non-successful status codes.
pub async fn run_external_ocr(
    api_endpoint: &str,
    api_key: Option<&str>,
    file_bytes: Vec<u8>,
    original_filename: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let file_part = multipart::Part::bytes(file_bytes)
        .file_name(original_filename.to_string())
        .mime_str("application/pdf") // Assuming PDF input for now
        .context("Failed to create file part for external OCR")?;

    let form = multipart::Form::new().part("file", file_part); // "file" is a common field name

    let mut request_builder = client.post(api_endpoint).multipart(form);

    if let Some(key_str) = api_key.filter(|k| !k.trim().is_empty()) {
        if key_str.to_lowercase().starts_with("bearer ") {
            let token_part = key_str.split_at("bearer ".len()).1;
            match HeaderValue::from_str(token_part) {
                Ok(mut header_val) => { // Make mutable to potentially set sensitive
                    header_val.set_sensitive(true); // Mark bearer token as sensitive
                    request_builder = request_builder.header(AUTHORIZATION, header_val);
                    log::debug!("Added Bearer token to external OCR request.");
                }
                Err(e) => {
                    log::warn!("Invalid characters in Bearer token for external OCR: {:?}. Authorization header not sent.", e);
                }
            }
        } else { // Assume it's a simple key for X-API-KEY or similar
            match HeaderValue::from_str(key_str) {
                Ok(mut header_val) => {
                    header_val.set_sensitive(true); // Also mark this as sensitive
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

/// Run the Tesseract OCR command locally.
///
/// * `input` - Path to the PDF file to process.
/// * `output` - Path (without extension) where Tesseract writes the text.
///
/// Errors if the `tesseract` command fails to run or returns a non-zero status.
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


#[derive(Deserialize, Debug)]
#[serde(tag = "strategy", content = "parameters", rename_all = "camelCase")]
enum ParseConfig {
    KeywordExtraction {
        keywords: Vec<String>,
        #[serde(default)]
        case_sensitive: bool,
    },
    RegexExtraction {
        patterns: Vec<RegexPattern>,
    },
    SimpleTableExtraction {
        header_keywords: Vec<String>,
        stop_keywords: Option<Vec<String>>,
    },
    Passthrough {
        // No parameters needed
    },
}

// Function to provide default value for capture_group_index
fn default_capture_group_index() -> usize {
    1 // Default to capturing the first group
}

#[derive(Deserialize, Debug, Clone)] // Added Clone
#[serde(rename_all = "camelCase")]
struct RegexPattern {
    name: String,
    regex: String,
    #[serde(default = "default_capture_group_index")]
    capture_group_index: usize,
}

/// Parse OCR text according to the optional configuration and return structured
/// JSON.
///
/// * `text_content` - The plain text produced by OCR.
/// * `config_json` - Optional configuration describing the parse strategy.
///
/// Errors if regex patterns fail to compile or if serialization of the result
/// fails.
pub async fn run_parse_stage(
    text_content: &str,
    config_json: Option<&JsonValue>,
) -> Result<JsonValue> {
    let config: Option<ParseConfig> = config_json
        .and_then(|c_val| serde_json::from_value(c_val.clone()).ok());

    match config {
        Some(ParseConfig::KeywordExtraction { keywords, case_sensitive }) => {
            let mut counts = HashMap::new();
            for keyword_orig in keywords {
                let keyword_to_search = if case_sensitive { keyword_orig.clone() } else { keyword_orig.to_lowercase() };
                let content_to_search = if case_sensitive { text_content.to_string() } else { text_content.to_lowercase() };

                let count = content_to_search.matches(&keyword_to_search).count();
                counts.insert(keyword_orig, count);
            }
            Ok(serde_json::to_value(counts)?)
        }
        Some(ParseConfig::RegexExtraction { patterns }) => {
            let mut extractions = HashMap::new();
            for pattern_def in patterns {
                match Regex::new(&pattern_def.regex) {
                    Ok(re) => {
                        let mut field_matches = Vec::new();
                        for cap in re.captures_iter(text_content) {
                            match cap.get(pattern_def.capture_group_index) {
                                Some(capture_match) => {
                                    field_matches.push(capture_match.as_str().to_string());
                                }
                                None => {
                                    if pattern_def.capture_group_index != 0 {
                                        if let Some(full_match) = cap.get(0) {
                                            log::warn!(
                                                "Capture group index {} out of bounds for regex '{}' (pattern: {}). Using full match instead.",
                                                pattern_def.capture_group_index, pattern_def.regex, pattern_def.name
                                            );
                                            field_matches.push(full_match.as_str().to_string());
                                        } else {
                                            log::error!(
                                                "Critical error: No group 0 (full match) found for a regex match. Regex: '{}', Pattern: {}",
                                                pattern_def.regex, pattern_def.name
                                            );
                                        }
                                    } else {
                                        log::warn!(
                                            "Capture group 0 (full match) not found for regex '{}' (pattern: {}), though it matched. Skipping this capture.",
                                            pattern_def.regex, pattern_def.name
                                        );
                                    }
                                }
                            }
                        }
                        if !field_matches.is_empty() {
                            extractions.insert(pattern_def.name.clone(), field_matches);
                        }
                    }
                    Err(e) => {
                        log::warn!("Invalid regex pattern '{}' for field '{}': {:?}. Skipping.", pattern_def.regex, pattern_def.name.clone(), e);
                        extractions.insert(pattern_def.name.clone(), vec![format!("Regex Compile Error: {}", e)]);
                    }
                }
            }
            Ok(serde_json::to_value(extractions)?)
        }
        Some(ParseConfig::SimpleTableExtraction { header_keywords, stop_keywords }) => {
            log::warn!("SimpleTableExtraction is a basic placeholder and may not yield useful results.");
            let mut result_data = HashMap::new();
            result_data.insert("status".to_string(), JsonValue::String("SimpleTableExtraction (placeholder)".to_string()));
            result_data.insert("matched_headers".to_string(), serde_json::to_value(header_keywords)?);
            if let Some(sk) = stop_keywords {
                 result_data.insert("stop_keywords_provided".to_string(), serde_json::to_value(sk)?);
            }
            Ok(serde_json::to_value(result_data)?)
        }
        Some(ParseConfig::Passthrough {}) | None => { // Default or Passthrough
             let lines: Vec<&str> = text_content.lines().map(|l| l.trim()).collect();
             Ok(serde_json::json!({
                 "strategy_used": if config.is_some() { "Passthrough" } else { "Default (Lines)" },
                 "lines": lines,
             }))
        }
    }
}

// Helper struct for report stage config deserialization
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReportStageConfig {
    template: String,
    #[serde(default)]
    summary_fields: Vec<String>,
}

// Helper for basic placeholder replacement
fn replace_placeholders(template: &str, data: &JsonValue) -> String {
    let mut result = template.to_string();
    let placeholder_re = Regex::new(r"\{\{\s*([\w.-]+)\s*\}\}").unwrap(); // This unwrap is fine if regex is static & tested

    for cap in placeholder_re.captures_iter(template) {
        let full_match = cap.get(0).unwrap().as_str();
        let key_path = cap.get(1).unwrap().as_str();

        let replacement_value = match key_path.split('.').collect::<Vec<&str>>().as_slice() {
            [key] => data.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            [key1, key2] => data.get(key1).and_then(|v| v.get(key2)).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            [key1, key2, key3] => data.get(key1).and_then(|v| v.get(key2)).and_then(|v| v.get(key3)).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            _ => {
                // Basic jsonpath_rust usage for deeper or more complex paths
                // Note: jsonpath_rust::JsonPath::query returns a Vec<&JsonValue>
                // For simplicity, we'll try to get the first element and convert to string.
                // A more robust solution would handle arrays/objects returned by path differently.
                match data.query(&format!("$.{}", key_path)) {
                    Ok(nodes) => nodes.first().and_then(|v_ref| v_ref.as_str()).unwrap_or("").to_string(),
                    Err(_) => format!("{{{{UNRESOLVED: {}}}}}", key_path),
                }
            }
        };
        result = result.replace(full_match, &replacement_value);
    }
    result
}

/// Generate a PDF report using a markdown template and provided data.
///
/// * `template_markdown` - Markdown with `{{placeholder}}` syntax.
/// * `data_for_templating` - JSON used to replace placeholders.
/// * `output_pdf_path` - Where the PDF will be written.
///
/// Fails if the template cannot be rendered or the PDF cannot be written.
pub async fn generate_report_from_template(
    template_markdown: &str,
    data_for_templating: &JsonValue,
    output_pdf_path: &std::path::Path,
) -> Result<()> {
    let processed_markdown = replace_placeholders(template_markdown, data_for_templating);

    let (mut doc, page1, layer1) = PdfDocument::new(
        data_for_templating.get("document_name").and_then(|v|v.as_str()).unwrap_or("Report"),
        Mm(210.0), Mm(297.0), "Layer1"
    );
    doc = doc.with_conformance(PdfConformance::X3_2002_PDF_1_3); // Example conformance

    let font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| anyhow!("Failed to add font: {}", e.to_string()))?;
    let mut current_layer = doc.get_page(page1).get_layer(layer1);

    let mut options = MarkdownOptions::empty();
    options.insert(MarkdownOptions::ENABLE_TABLES);
    options.insert(MarkdownOptions::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(&processed_markdown, options);

    let mut y_cursor = Mm(280.0);
    let line_height_normal = Mm(6.0);
    let line_height_heading1 = Mm(10.0);
    let line_height_heading2 = Mm(8.0);
    let line_height_heading3 = Mm(7.0);
    let left_margin = Mm(15.0);
    // let right_margin = Mm(210.0 - 15.0); // Not directly used in this basic renderer for line breaks

    for event in parser {
        if y_cursor < Mm(20.0) { // Rudimentary page break check
            log::warn!("Report content exceeded single page (basic renderer). Content may be truncated.");
            break;
        }
        match event {
            Event::Start(Tag::Heading(level, _, _)) => {
                current_layer.begin_text_section();
                let font_size = match level {
                    HeadingLevel::H1 => 18.0,
                    HeadingLevel::H2 => 15.0,
                    HeadingLevel::H3 => 13.0,
                    _ => 11.0,
                };
                let line_height = match level {
                    HeadingLevel::H1 => line_height_heading1,
                    HeadingLevel::H2 => line_height_heading2,
                    HeadingLevel::H3 => line_height_heading3,
                    _ => line_height_normal,
                };
                current_layer.set_font(&font, font_size);
                current_layer.set_text_cursor(left_margin, y_cursor);
                y_cursor -= line_height;
            }
            Event::End(Tag::Heading(_, _, _)) => {
                current_layer.end_text_section();
            }
            Event::Start(Tag::Paragraph) => {
                current_layer.begin_text_section();
                current_layer.set_font(&font, 11.0);
                current_layer.set_text_cursor(left_margin, y_cursor);
            }
            Event::End(Tag::Paragraph) => {
                current_layer.end_text_section();
                y_cursor -= line_height_normal;
            }
            Event::Text(text) => {
                for (i, line_str) in text.split("\n").enumerate() {
                    if i > 0 {
                        y_cursor -= line_height_normal;
                        if y_cursor < Mm(20.0) { break; }
                        current_layer.set_text_cursor(left_margin, y_cursor);
                    }
                    current_layer.write_text(line_str.to_string(), &font);
                }
            }
            Event::Code(code_text) => {
                // Basic inline code representation
                current_layer.begin_text_section();
                current_layer.set_font(&font, 10.0); // Ideally a monospace font
                current_layer.set_text_cursor(left_margin, y_cursor);
                current_layer.write_text(format!("`{}`", code_text.to_string()), &font);
                current_layer.end_text_section();
                y_cursor -= line_height_normal;
            }
            Event::HardBreak => {
                y_cursor -= line_height_normal;
                current_layer.set_text_cursor(left_margin, y_cursor);
            }
            Event::Start(Tag::List(_)) => {
                current_layer.begin_text_section();
                current_layer.set_font(&font, 11.0);
                current_layer.set_text_cursor(left_margin, y_cursor);
            }
            Event::End(Tag::List(_)) => {
                current_layer.end_text_section();
                y_cursor -= line_height_normal;
            }
            Event::Start(Tag::Item) => {
                current_layer.write_text("\u{2022} ", &font);
            }
            Event::End(Tag::Item) => {
                y_cursor -= line_height_normal;
                current_layer.set_text_cursor(left_margin, y_cursor);
            }
            Event::Start(Tag::Table(_)) => {
                current_layer.begin_text_section();
                current_layer.set_font(&font, 11.0);
                current_layer.set_text_cursor(left_margin, y_cursor);
            }
            Event::End(Tag::Table(_)) => {
                current_layer.end_text_section();
                y_cursor -= line_height_normal;
            }
            Event::Start(Tag::TableRow) => {
                current_layer.write_text("| ", &font);
            }
            Event::End(Tag::TableRow) => {
                current_layer.write_text("|", &font);
                y_cursor -= line_height_normal;
                current_layer.set_text_cursor(left_margin, y_cursor);
            }
            Event::Start(Tag::TableCell) => {}
            Event::End(Tag::TableCell) => {
                current_layer.write_text(" | ", &font);
            }
            Event::Start(Tag::BlockQuote) => {
                current_layer.begin_text_section();
                current_layer.set_font(&font, 11.0);
                current_layer.set_text_cursor(left_margin + Mm(5.0), y_cursor);
                current_layer.write_text("> ", &font);
            }
            Event::End(Tag::BlockQuote) => {
                current_layer.end_text_section();
                y_cursor -= line_height_normal;
            }
            Event::Rule => {
                current_layer.begin_text_section();
                current_layer.set_text_cursor(left_margin, y_cursor);
                current_layer.write_text("-----", &font);
                current_layer.end_text_section();
                y_cursor -= line_height_normal;
            }
            _ => { /* log::debug!("Unhandled Markdown Event: {:?}", event); */ }
        }
    }

    let file = File::create(output_pdf_path).context(format!("Failed to create output PDF file: {:?}", output_pdf_path))?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| anyhow!("Failed to save PDF: {}", e.to_string()))?;
    Ok(())
}


#[derive(Deserialize, Debug, Clone)]
struct CustomHeader {
    name: String,
    value: String,
}

/// Call an external AI API with the given JSON payload and return the parsed
/// response body.
///
/// * `input` - JSON payload sent in the request body.
/// * `api_endpoint` - Endpoint URL to post to.
/// * `api_key` - Bearer token used for authorization.
/// * `custom_headers_json` - Optional array of `{name, value}` pairs to include
///   as additional headers.
///
/// Returns an error if the request fails or if a non-successful status code is
/// returned.
pub async fn run_ai(
    input: &serde_json::Value,
    api_endpoint: &str, // Renamed from 'endpoint'
    api_key: &str,      // Renamed from 'key'
    custom_headers_json: Option<&serde_json::Value>,
) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let mut request_builder = client.post(api_endpoint);

    // Add Bearer token
    if !api_key.is_empty() {
        request_builder = request_builder.bearer_auth(api_key);
    }

    // Add custom headers if provided
    if let Some(headers_val) = custom_headers_json {
        // Ensure headers_val is an array before trying to deserialize
        if headers_val.is_array() {
            match serde_json::from_value::<Vec<CustomHeader>>(headers_val.clone()) {
                Ok(headers_vec) => {
                    for header_obj in headers_vec {
                        if header_obj.name.trim().is_empty() {
                            log::warn!("Skipping custom AI header with empty name: {:?}", header_obj);
                            continue;
                        }
                        match HeaderName::from_bytes(header_obj.name.as_bytes()) {
                            Ok(header_name) => {
                                match HeaderValue::from_str(&header_obj.value) {
                                    Ok(header_value) => {
                                        request_builder = request_builder.header(header_name, header_value);
                                    }
                                    Err(e) => {
                                        log::warn!("Invalid custom AI header value for '{}': {:?}. Skipping.", header_obj.name, e);
                                    }
                                }
                            }
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
        } else if !headers_val.is_null() { // Allow null, but not other non-array types
            log::warn!("ai_custom_headers is not an array, skipping. Headers JSON: {}", headers_val);
        }
    }

    // Set content type and send JSON body
    request_builder = request_builder.header(CONTENT_TYPE, "application/json");

    let response = request_builder.json(input).send().await?;

    if response.status().is_success() {
        Ok(response.json().await?)
    } else {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error content".to_string());
        log::error!("AI API request to {} failed with status {}: {}", api_endpoint, status, error_text);
        Err(anyhow!("AI API request failed with status {}: {}", status, error_text))
    }
}

/// Write a very basic PDF report containing the JSON representation of `json`.
///
/// * `json` - Data to render as text in the PDF.
/// * `path` - Destination path for the generated PDF.
///
/// Fails if the PDF cannot be written to disk.
pub fn generate_report(json: &serde_json::Value, path: &Path) -> Result<()> {
    let (mut doc, page1, layer1) = PdfDocument::new("Report", Mm(210.0), Mm(297.0), "Layer1");
    let current_layer = doc.get_page(page1).get_layer(layer1);
    let text = json.to_string();
    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    current_layer.use_text(text, 12.0, Mm(10.0), Mm(280.0), &font);
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer)?;
    Ok(())
}
