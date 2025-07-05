use backend::processing::parse::run_parse_stage;
use serde_json::json;
use uuid::Uuid;

mod test_utils;
use test_utils::generate_jwt_token;

#[actix_rt::test]
async fn regex_extraction_basic() {
    let token = generate_jwt_token(Uuid::new_v4(), Uuid::new_v4(), "user");
    let text = format!("User token: {} Email: user@example.com", token);
    let config = json!({
        "strategy": "regexExtraction",
        "parameters": {
            "patterns": [
                {"name": "token", "regex": r"([A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+)"},
                {"name": "email", "regex": r"[\w.-]+@[\w.-]+"}
            ]
        }
    });
    let res = run_parse_stage(&text, Some(&config)).await.unwrap();
    assert_eq!(res["email"][0], "user@example.com");
    assert_eq!(res["token"][0], token);
}

#[actix_rt::test]
async fn regex_capture_group_index() {
    let text = "Total: 42";
    let config = json!({
        "strategy": "regexExtraction",
        "parameters": {
            "patterns": [
                {"name": "total", "regex": r"Total:\s+(\d+)", "captureGroupIndex": 1}
            ]
        }
    });
    let res = run_parse_stage(text, Some(&config)).await.unwrap();
    assert_eq!(res["total"][0], "42");
}

#[actix_rt::test]
async fn regex_capture_group_out_of_bounds() {
    let text = "Value: 99";
    let config = json!({
        "strategy": "regexExtraction",
        "parameters": {
            "patterns": [
                {"name": "val", "regex": r"Value:\s+(\d+)", "captureGroupIndex": 2}
            ]
        }
    });
    let res = run_parse_stage(text, Some(&config)).await.unwrap();
    assert_eq!(res["val"][0], "Value: 99");
}

#[actix_rt::test]
async fn regex_invalid_pattern() {
    let text = "abc";
    let config = json!({
        "strategy": "regexExtraction",
        "parameters": {
            "patterns": [
                {"name": "bad", "regex": "([a-z"}
            ]
        }
    });
    let res = run_parse_stage(text, Some(&config)).await.unwrap();
    assert!(res["bad"][0].as_str().unwrap().contains("Regex Compile Error"));
}
