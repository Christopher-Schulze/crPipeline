use actix_web::HttpResponse;
use std::collections::HashSet;

pub fn validate_stages(stages: &serde_json::Value) -> Result<(), HttpResponse> {
    if let Some(stages_array) = stages.as_array() {
        if stages_array.is_empty() {
            return Err(HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "Pipeline must have at least one stage."})));
        }
        let mut seen_ids = HashSet::new();
        for (index, stage_val) in stages_array.iter().enumerate() {
            if let Some(stage_obj) = stage_val.as_object() {
                if let Some(id_val) = stage_obj.get("id").and_then(|v| v.as_str()) {
                    if !seen_ids.insert(id_val.to_string()) {
                        return Err(HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("Duplicate stage id '{}'", id_val)
                        })));
                    }
                }
                let stage_type_str: String;
                if let Some(type_val) = stage_obj.get("type") {
                    if let Some(s) = type_val.as_str() {
                        if s.trim().is_empty() {
                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} 'type' cannot be empty.", index)
                            })));
                        }
                        stage_type_str = s.trim().to_lowercase();
                    } else {
                        return Err(HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("Stage {} 'type' must be a string.", index)
                        })));
                    }
                } else {
                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                        "error": format!("Stage {} must have a 'type' field.", index)
                    })));
                }

                let command_value_opt = stage_obj.get("command");
                let command_missing = match command_value_opt {
                    None => true,
                    Some(v) if v.is_null() => true,
                    Some(v) => {
                        if let Some(command_str) = v.as_str() {
                            if command_str.trim().is_empty() {
                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} 'command', if present and not null, cannot be empty.", index)
                                })));
                            }
                            false
                        } else {
                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} 'command' must be a string or null.", index)
                            })));
                        }
                    }
                };

                match stage_type_str.as_str() {
                    "ai" => {
                        if command_missing {
                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} (AI): 'command' is required.", index)
                            })));
                        }
                        if let Some(prompt_name_val) = stage_obj.get("prompt_name") {
                            if !prompt_name_val.is_string() && !prompt_name_val.is_null() {
                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} (AI): 'prompt_name' must be a string or null.", index)
                                }))); 
                            }
                            if let Some(s) = prompt_name_val.as_str() {
                                if s.trim().is_empty() {
                                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                        "error": format!("Stage {} (AI): 'prompt_name', if a string, cannot be empty.", index)
                                    })));
                                }
                            }
                        }
                    }
                    "ocr" => {
                        if command_missing {
                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} (OCR): 'command' is required.", index)
                            })));
                        }
                        if let Some(engine_val) = stage_obj.get("ocr_engine") {
                            if !engine_val.is_null() {
                                if let Some(engine_str) = engine_val.as_str() {
                                    if engine_str != "default" && engine_str != "external" {
                                        return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                            "error": format!("Stage {} (OCR): 'ocr_engine' must be 'default', 'external', or null.", index)
                                        })));
                                    }
                                    if engine_str == "external" {
                                        if let Some(endpoint_val) =
                                            stage_obj.get("ocr_stage_endpoint")
                                        {
                                            if let Some(endpoint_str) = endpoint_val.as_str() {
                                                if endpoint_str.trim().is_empty() {
                                                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                                        "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' must be a non-empty string when ocr_engine is 'external'.", index)
                                                    })));
                                                }
                                            } else {
                                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                                    "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' must be a non-empty string when ocr_engine is 'external'.", index)
                                                })));
                                            }
                                        } else {
                                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                                "error": format!("Stage {} (OCR): 'ocr_stage_endpoint' is required when ocr_engine is 'external'.", index)
                                            })));
                                        }
                                        if let Some(key_val) = stage_obj.get("ocr_stage_key") {
                                            if !key_val.is_string() && !key_val.is_null() {
                                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                                    "error": format!("Stage {} (OCR): 'ocr_stage_key' for external engine must be a string or null.", index)
                                                }))); 
                                            }
                                        }
                                    }
                                } else {
                                    return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                        "error": format!("Stage {} (OCR): 'ocr_engine' must be a string or null.", index)
                                    }))); 
                                }
                            }
                        }
                        if stage_obj
                            .get("ocr_engine")
                            .as_ref()
                            .map_or(true, |v| v.as_str() != Some("external"))
                        {
                            if stage_obj.contains_key("ocr_stage_endpoint")
                                || stage_obj.contains_key("ocr_stage_key")
                            {
                                if stage_obj.get("ocr_engine").is_none()
                                    || stage_obj.get("ocr_engine").and_then(|v| v.as_str())
                                        == Some("default")
                                {
                                    // allowed: endpoint/key provided but engine default or absent (no validation here)
                                }
                            }
                        }
                        if let Some(key_val) = stage_obj.get("ocr_stage_key") {
                            if stage_obj
                                .get("ocr_engine")
                                .and_then(|v| v.as_str())
                                != Some("external")
                                && !key_val.is_null()
                            {
                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} (OCR): 'ocr_stage_key' can only be set when ocr_engine is 'external'.", index)
                                }))); 
                            }
                            if !key_val.is_string() && !key_val.is_null() {
                                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": format!("Stage {} (OCR): 'ocr_stage_key' must be a string or null.", index)
                                }))); 
                            }
                        }
                    }
                    "parse" | "report" => {
                        if command_missing {
                            return Err(HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Stage {} ({}): 'command' is required.", index, stage_type_str)
                            })));
                        }
                    }
                    _ => {}
                }
            } else {
                return Err(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": format!("Stage {} must be an object.", index)
                })));
            }
        }
    } else {
        return Err(HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "'stages' must be an array."})));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_stages;
    use serde_json::json;

    #[test]
    fn missing_stage_id_is_ok() {
        let stages = json!([
            {"type": "ocr", "command": "run"},
            {"id": "b", "type": "ai", "command": "run"}
        ]);
        assert!(validate_stages(&stages).is_ok());
    }

    #[test]
    fn invalid_ocr_engine_rejected() {
        let stages = json!([
            {"type": "ocr", "ocr_engine": "foo", "command": "run"}
        ]);
        assert!(validate_stages(&stages).is_err());
    }

    #[test]
    fn external_ocr_without_endpoint_rejected() {
        let stages = json!([
            {"type": "ocr", "ocr_engine": "external", "command": "run"}
        ]);
        assert!(validate_stages(&stages).is_err());
    }
}
