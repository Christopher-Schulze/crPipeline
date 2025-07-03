use backend::processing::run_parse_stage;
use serde_json::json;

#[actix_rt::test]
async fn test_simple_table_extraction() {
    let text = "HEADER\nItem Qty Price\nApple 1 2.00\nBanana 2 4.00\nTotal 6.00";
    let config = json!({
        "strategy": "SimpleTableExtraction",
        "parameters": {
            "headerKeywords": ["item", "qty", "price"],
            "stopKeywords": ["total"]
        }
    });
    let res = run_parse_stage(text, Some(&config)).await.unwrap();
    assert_eq!(res["status"], "ok");
    assert_eq!(res["headers"].as_array().unwrap().len(), 3);
    assert_eq!(res["rows"].as_array().unwrap().len(), 2);
}

#[actix_rt::test]
async fn test_simple_table_extraction_no_headers() {
    let text = "No table here";
    let config = json!({
        "strategy": "SimpleTableExtraction",
        "parameters": {
            "headerKeywords": ["item", "qty"]
        }
    });
    let res = run_parse_stage(text, Some(&config)).await.unwrap();
    assert_eq!(res["status"], "header_not_found");
}

#[actix_rt::test]
async fn test_numeric_summary() {
    let text = "HEADER\nItem Qty Price\nApple 1 2\nBanana 2 4\nTotal 6";
    let config = json!({
        "strategy": "SimpleTableExtraction",
        "parameters": {
            "headerKeywords": ["item", "qty", "price"],
            "stopKeywords": ["total"],
            "numericSummary": true
        }
    });
    let res = run_parse_stage(text, Some(&config)).await.unwrap();
    assert_eq!(res["status"], "ok");
    let summary = res["numeric_summary"].as_object().unwrap();
    assert_eq!(summary["qty"]["sum"], 3.0);
    assert_eq!(summary["price"]["sum"], 6.0);
}
