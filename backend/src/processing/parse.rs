use anyhow::{anyhow, Result};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;

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
        #[serde(default)]
        delimiter_regex: Option<String>,
        #[serde(default)]
        numeric_summary: bool,
    },
    Passthrough {},
}

fn default_capture_group_index() -> usize {
    1
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct RegexPattern {
    name: String,
    regex: String,
    #[serde(default = "default_capture_group_index")]
    capture_group_index: usize,
}

#[tracing::instrument(skip(text_content, config_json))]
pub async fn run_parse_stage(
    text_content: &str,
    config_json: Option<&serde_json::Value>,
) -> Result<serde_json::Value> {
    let config: Option<ParseConfig> =
        config_json.and_then(|c_val| serde_json::from_value(c_val.clone()).ok());

    match config {
        Some(ParseConfig::KeywordExtraction { keywords, case_sensitive }) => {
            let mut counts = HashMap::new();
            for keyword_orig in keywords {
                let keyword_to_search = if case_sensitive {
                    keyword_orig.clone()
                } else {
                    keyword_orig.to_lowercase()
                };
                let content_to_search = if case_sensitive {
                    text_content.to_string()
                } else {
                    text_content.to_lowercase()
                };
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
                        log::warn!(
                            "Invalid regex pattern '{}' for field '{}': {:?}. Skipping.",
                            pattern_def.regex,
                            pattern_def.name.clone(),
                            e
                        );
                        extractions.insert(
                            pattern_def.name.clone(),
                            vec![format!("Regex Compile Error: {}", e)],
                        );
                    }
                }
            }
            Ok(serde_json::to_value(extractions)?)
        }
        Some(ParseConfig::SimpleTableExtraction {
            header_keywords,
            stop_keywords,
            delimiter_regex,
            numeric_summary,
        }) => {
            let lines: Vec<&str> = text_content.lines().collect();
            let mut header_index = None;
            for (idx, line) in lines.iter().enumerate() {
                let lower = line.to_lowercase();
                if header_keywords.iter().all(|kw| lower.contains(&kw.to_lowercase())) {
                    header_index = Some(idx);
                    break;
                }
            }
            if let Some(h_idx) = header_index {
                let regex_pattern = delimiter_regex.as_deref().unwrap_or(r"\s{2,}|\t|\s*\|\s*");
                let delim_re = match Regex::new(regex_pattern) {
                    Ok(re) => re,
                    Err(e) => {
                        log::warn!(
                            "Invalid delimiter regex '{}': {:?}. Using default delimiter.",
                            regex_pattern,
                            e
                        );
                        Regex::new(r"\s{2,}|\t|\s*\|\s*")
                            .map_err(|err| anyhow!("Failed to compile fallback delimiter regex: {}", err))?
                    }
                };
                let headers: Vec<String> = delim_re
                    .split(lines[h_idx].trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().to_string())
                    .collect();
                let mut rows: Vec<Vec<String>> = Vec::new();
                for line in lines.iter().skip(h_idx + 1) {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let lower = trimmed.to_lowercase();
                    if let Some(stops) = &stop_keywords {
                        if stops.iter().any(|kw| lower.contains(&kw.to_lowercase())) {
                            break;
                        }
                    }
                    let row: Vec<String> = delim_re
                        .split(trimmed)
                        .filter(|s| !s.is_empty())
                        .map(|s| s.trim().to_string())
                        .collect();
                    if !row.is_empty() {
                        rows.push(row);
                    }
                }
                let mut result = serde_json::json!({
                    "status": "ok",
                    "headers": headers,
                    "rows": rows,
                });
                if numeric_summary {
                    if let (Some(h), Some(r)) = (result.get("headers"), result.get("rows")) {
                        let headers_vec = match h.as_array() {
                            Some(arr) => arr,
                            None => {
                                log::error!("Expected headers to be an array");
                                return Ok(result);
                            }
                        };
                        let rows_vec = match r.as_array() {
                            Some(arr) => arr,
                            None => {
                                log::error!("Expected rows to be an array");
                                return Ok(result);
                            }
                        };
                        let mut summary = serde_json::Map::new();
                        for (col_idx, header_val) in headers_vec.iter().enumerate() {
                            if let Some(header_str) = header_val.as_str() {
                                let mut numeric: Vec<f64> = Vec::new();
                                for row in rows_vec {
                                    if let Some(cell) = row.get(col_idx) {
                                        let normalized = cell.as_str().unwrap_or("").replace(',', ".");
                                        if let Ok(n) = normalized.parse::<f64>() {
                                            numeric.push(n);
                                        } else {
                                            numeric.clear();
                                            break;
                                        }
                                    } else {
                                        numeric.clear();
                                        break;
                                    }
                                }
                                if !numeric.is_empty() {
                                    let sum: f64 = numeric.iter().sum();
                                    let avg = sum / numeric.len() as f64;
                                    summary.insert(
                                        header_str.to_string(),
                                        serde_json::json!({"sum": sum, "avg": avg}),
                                    );
                                }
                            }
                        }
                        if !summary.is_empty() {
                            if let Some(obj) = result.as_object_mut() {
                                obj.insert(
                                    "numeric_summary".to_string(),
                                    serde_json::Value::Object(summary),
                                );
                            }
                        }
                    }
                }
                Ok(result)
            } else {
                Ok(serde_json::json!({
                    "status": "header_not_found"
                }))
            }
        }
        Some(ParseConfig::Passthrough {}) | None => {
            let lines: Vec<&str> = text_content.lines().map(|l| l.trim()).collect();
            Ok(serde_json::json!({
                "strategy_used": if config.is_some() { "Passthrough" } else { "Default (Lines)" },
                "lines": lines,
            }))
        }
    }
}
