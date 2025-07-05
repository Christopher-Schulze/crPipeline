use backend::processing::report::generate_report_from_template;
use serde_json::json;
use lopdf::Document as PdfDoc;

#[actix_rt::test]
async fn list_and_table_render() {
    let md = "# Items\n- First\n- Second\n\n|A|B|\n|---|---|\n|1|2|";
    let data = json!({"document_name": "Test"});
    let tmp = tempfile::NamedTempFile::new().unwrap();
    generate_report_from_template(md, &data, tmp.path()).await.unwrap();
    let pdf = PdfDoc::load(tmp.path()).unwrap();
    let text = pdf.extract_text(&[1]).unwrap();
    assert!(text.contains("First"));
    assert!(text.contains("Second"));
    assert!(text.contains("A"));
    assert!(text.contains("B"));
    assert!(text.contains("1"));
    assert!(text.contains("2"));
}
