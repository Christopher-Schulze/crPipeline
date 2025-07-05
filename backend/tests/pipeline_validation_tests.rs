use backend::pipeline_validation::validate_stages;
use serde_json::json;

#[test]
fn valid_configurations() {
    let stages = json!([
        {"id": "a", "type": "ai", "command": "run_ai"},
        {"id": "b", "type": "ocr", "command": "run_ocr", "ocr_engine": "external", "ocr_stage_endpoint": "http://ex", "ocr_stage_key": "k"},
        {"id": "c", "type": "parse", "command": "run_parse"},
        {"id": "d", "type": "report", "command": "run_report"}
    ]);
    assert!(validate_stages(&stages).is_ok());
}

#[test]
fn missing_command_not_allowed() {
    let stages = json!([
        {"id": "a", "type": "ai"}
    ]);
    assert!(validate_stages(&stages).is_err());
}

#[test]
fn ocr_key_without_external_engine_rejected() {
    let stages = json!([
        {"id": "b", "type": "ocr", "command": "run", "ocr_stage_key": "k"}
    ]);
    assert!(validate_stages(&stages).is_err());
}
