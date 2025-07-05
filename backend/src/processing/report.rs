use anyhow::{anyhow, Context, Result};
use printpdf::*;
use regex::Regex;
use jsonpath_rust::JsonPath;
use pulldown_cmark::{Event, HeadingLevel, Options as MarkdownOptions, Parser, Tag};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn replace_placeholders(template: &str, data: &serde_json::Value) -> String {
    let mut result = template.to_string();
    let placeholder_re = match Regex::new(r"\{\{\s*([\w.-]+)\s*\}\}") {
        Ok(re) => re,
        Err(e) => {
            log::error!("Failed to compile placeholder regex: {:?}", e);
            return result;
        }
    };
    for cap in placeholder_re.captures_iter(template) {
        let (Some(full_match), Some(key_match)) = (cap.get(0), cap.get(1)) else { continue }; 
        let full_match = full_match.as_str();
        let key_path = key_match.as_str();
        let replacement_value = match key_path.split('.').collect::<Vec<&str>>().as_slice() {
            [key] => data.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            [key1, key2] => data.get(key1).and_then(|v| v.get(key2)).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            [key1, key2, key3] => data.get(key1).and_then(|v| v.get(key2)).and_then(|v| v.get(key3)).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            _ => match data.query(&format!("$.{}", key_path)) {
                Ok(nodes) => nodes.first().and_then(|v_ref| v_ref.as_str()).unwrap_or("").to_string(),
                Err(_) => format!("{{{{UNRESOLVED: {}}}}}", key_path),
            },
        };
        result = result.replace(full_match, &replacement_value);
    }
    result
}

pub async fn generate_report_from_template(
    template_markdown: &str,
    data_for_templating: &serde_json::Value,
    output_pdf_path: &Path,
) -> Result<()> {
    let processed_markdown = replace_placeholders(template_markdown, data_for_templating);
    let (mut doc, page1, layer1) = PdfDocument::new(
        data_for_templating
            .get("document_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Report"),
        Mm(210.0),
        Mm(297.0),
        "Layer1",
    );
    doc = doc.with_conformance(PdfConformance::X3_2002_PDF_1_3);
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| anyhow!("Failed to add font: {}", e.to_string()))?;
    let current_layer = doc.get_page(page1).get_layer(layer1);
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
        if y_cursor < Mm(20.0) { break; }
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
            Event::End(Tag::Heading(_, _, _)) => { current_layer.end_text_section(); }
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
                    if i > 0 { y_cursor -= line_height_normal; if y_cursor < Mm(20.0) { break; } current_layer.set_text_cursor(left_margin, y_cursor); }
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
            Event::HardBreak => { y_cursor -= line_height_normal; current_layer.set_text_cursor(left_margin, y_cursor); }
            Event::Start(Tag::List(_)) => { current_layer.begin_text_section(); current_layer.set_font(&font, 11.0); current_layer.set_text_cursor(left_margin, y_cursor); }
            Event::End(Tag::List(_)) => { current_layer.end_text_section(); y_cursor -= line_height_normal; }
            Event::Start(Tag::Item) => { current_layer.write_text("\u{2022} ", &font); }
            Event::End(Tag::Item) => { y_cursor -= line_height_normal; current_layer.set_text_cursor(left_margin, y_cursor); }
            Event::Start(Tag::Table(_)) => { current_layer.begin_text_section(); current_layer.set_font(&font, 11.0); current_layer.set_text_cursor(left_margin, y_cursor); }
            Event::End(Tag::Table(_)) => { current_layer.end_text_section(); y_cursor -= line_height_normal; }
            Event::Start(Tag::TableRow) => { current_layer.write_text("| ", &font); }
            Event::End(Tag::TableRow) => { current_layer.write_text("|", &font); y_cursor -= line_height_normal; current_layer.set_text_cursor(left_margin, y_cursor); }
            Event::Start(Tag::TableCell) => {}
            Event::End(Tag::TableCell) => { current_layer.write_text(" | ", &font); }
            Event::Start(Tag::BlockQuote) => { current_layer.begin_text_section(); current_layer.set_font(&font, 11.0); current_layer.set_text_cursor(left_margin + Mm(5.0), y_cursor); current_layer.write_text("> ", &font); }
            Event::End(Tag::BlockQuote) => { current_layer.end_text_section(); y_cursor -= line_height_normal; }
            Event::Rule => { current_layer.begin_text_section(); current_layer.set_text_cursor(left_margin, y_cursor); current_layer.write_text("-----", &font); current_layer.end_text_section(); y_cursor -= line_height_normal; }
            _ => {}
        }
    }
    let file = File::create(output_pdf_path).context(format!("Failed to create output PDF file: {:?}", output_pdf_path))?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| anyhow!("Failed to save PDF: {}", e.to_string()))?;
    Ok(())
}

pub fn generate_report(json: &serde_json::Value, path: &Path) -> Result<()> {
    let (doc, page1, layer1) = PdfDocument::new("Report", Mm(210.0), Mm(297.0), "Layer1");
    let current_layer = doc.get_page(page1).get_layer(layer1);
    let text = json.to_string();
    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    current_layer.use_text(text, 12.0, Mm(10.0), Mm(280.0), &font);
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer)?;
    Ok(())
}
