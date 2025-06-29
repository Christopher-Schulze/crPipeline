use anyhow::{Result, Context, anyhow};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use printpdf::*;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use regex::Regex;
use pulldown_cmark::{Parser, Event, Tag, Options as MarkdownOptions};
use jsonpath_rust::JsonPathQuery;
use log;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReportStageConfig {
    template: String,
    #[serde(default)]
    summary_fields: Vec<String>,
}

fn replace_placeholders(template: &str, data: &JsonValue) -> String {
    let mut result = template.to_string();
    let placeholder_re = Regex::new(r"\{\{\s*([\w.-]+)\s*\}\}").unwrap();

    for cap in placeholder_re.captures_iter(template) {
        let full_match = cap.get(0).unwrap().as_str();
        let key_path = cap.get(1).unwrap().as_str();

        let replacement_value = match key_path.split('.').collect::<Vec<&str>>().as_slice() {
            [key] => data.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            [key1, key2] => data.get(key1).and_then(|v| v.get(key2)).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            [key1, key2, key3] => data.get(key1).and_then(|v| v.get(key2)).and_then(|v| v.get(key3)).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            _ => {
                match data.path(&format!("$.{}", key_path)) {
                    Ok(nodes) => nodes.first().and_then(|v_ref| v_ref.as_str()).unwrap_or("").to_string(),
                    Err(_) => format!("{{{{UNRESOLVED: {}}}}}", key_path),
                }
            }
        };
        result = result.replace(full_match, &replacement_value);
    }
    result
}

pub async fn generate_report_from_template(
    template_markdown: &str,
    data_for_templating: &JsonValue,
    output_pdf_path: &Path,
) -> Result<()> {
    let processed_markdown = replace_placeholders(template_markdown, data_for_templating);

    let (mut doc, page1, layer1) = PdfDocument::new(
        data_for_templating.get("document_name").and_then(|v|v.as_str()).unwrap_or("Report"),
        Mm(210.0), Mm(297.0), "Layer1"
    );
    doc.set_conformance(PdfConformance::X3_2002_PDF_1_3);

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

    for event in parser {
        if y_cursor < Mm(20.0) {
            log::warn!("Report content exceeded single page (basic renderer). Content may be truncated.");
            break;
        }
        match event {
            Event::Start(Tag::Heading(level_u32, _, _)) => {
                current_layer.begin_text_section();
                 let font_size = match level_u32 {
                    1 => 18.0,
                    2 => 15.0,
                    3 => 13.0,
                    _ => 11.0,
                };
                let line_height = match level_u32 {
                    1 => line_height_heading1,
                    2 => line_height_heading2,
                    3 => line_height_heading3,
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
                for (i, line_str) in text.split('\n').enumerate() {
                    if i > 0 {
                        y_cursor -= line_height_normal;
                        if y_cursor < Mm(20.0) { break; }
                        current_layer.set_text_cursor(left_margin, y_cursor);
                    }
                    current_layer.write_text(line_str.to_string(), &font);
                }
            }
            Event::Code(code_text) => {
                current_layer.begin_text_section();
                current_layer.set_font(&font, 10.0);
                current_layer.set_text_cursor(left_margin, y_cursor);
                current_layer.write_text(format!("`{}`", code_text.to_string()), &font);
                current_layer.end_text_section();
                y_cursor -= line_height_normal;
            }
            Event::HardBreak => {
                y_cursor -= line_height_normal;
                current_layer.set_text_cursor(left_margin, y_cursor);
            }
            _ => {}
        }
    }

    let file = File::create(output_pdf_path).context(format!("Failed to create output PDF file: {:?}", output_pdf_path))?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| anyhow!("Failed to save PDF: {}", e.to_string()))?;
    Ok(())
}

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
