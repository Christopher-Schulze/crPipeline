use anyhow::Result;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use regex::Regex;
use log;

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
        Some(ParseConfig::Passthrough {}) | None => {
             let lines: Vec<&str> = text_content.lines().map(|l| l.trim()).collect();
             Ok(serde_json::json!({
                 "strategy_used": if config.is_some() { "Passthrough" } else { "Default (Lines)" },
                 "lines": lines,
             }))
        }
    }
}
