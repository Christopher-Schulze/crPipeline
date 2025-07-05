use backend::processing::report::generate_report_from_template;
use serde_json::json;
use lopdf::Document as PdfDoc;
use uuid::Uuid;

mod test_utils;
use test_utils::generate_jwt_token;

#[actix_rt::test]
async fn placeholder_nested() {
    let token = generate_jwt_token(Uuid::new_v4(), Uuid::new_v4(), "user");
    let md = "## Info\nToken: {{auth.token}}";
    let data = json!({"document_name": "Doc", "auth": {"token": token}});
    let tmp = tempfile::NamedTempFile::new().unwrap();
    generate_report_from_template(md, &data, tmp.path()).await.unwrap();
    let pdf = PdfDoc::load(tmp.path()).unwrap();
    let text = pdf.extract_text(&[1]).unwrap();
    assert!(text.contains(&token));
}

#[actix_rt::test]
async fn placeholder_jsonpath() {
    let md = "First item: {{summary.section.sub.value}}";
    let data = json!({
        "document_name": "Items",
        "summary": {"section": {"sub": {"value": "Apple"}}}
    });
    let tmp = tempfile::NamedTempFile::new().unwrap();
    generate_report_from_template(md, &data, tmp.path()).await.unwrap();
    let pdf = PdfDoc::load(tmp.path()).unwrap();
    let text = pdf.extract_text(&[1]).unwrap();
    assert!(text.contains("Apple"));
}

#[actix_rt::test]
async fn invalid_output_path() {
    let md = "Hi";
    let data = json!({"document_name": "Doc"});
    let dir = tempfile::tempdir().unwrap();
    let bad_path = dir.path().join("missing").join("out.pdf");
    let res = generate_report_from_template(md, &data, &bad_path).await;
    assert!(res.is_err());
}
