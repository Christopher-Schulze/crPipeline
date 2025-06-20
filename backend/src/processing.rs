use std::path::Path;
use std::fs::{File};
use std::io::BufWriter;
use aws_sdk_s3::Client as S3Client;
use anyhow::Result;
use reqwest::Client;
use printpdf::*;
use tokio::process::Command;
use uuid::Uuid;

pub async fn download_pdf(s3: &S3Client, bucket: &str, key: &str, path: &Path) -> Result<()> {
    let obj = s3.get_object().bucket(bucket).key(key).send().await?;
    let bytes = obj.body.collect().await?.into_bytes();
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

pub async fn run_ocr(input: &Path, output: &Path) -> Result<()> {
    let status = Command::new("tesseract")
        .arg(input)
        .arg(output)
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("tesseract failed");
    }
    Ok(())
}

pub async fn parse_text(path: &Path) -> Result<serde_json::Value> {
    let text = tokio::fs::read_to_string(path).await?;
    // Simple parsing: split lines into JSON array
    let lines: Vec<String> = text.lines().map(|l| l.trim().to_owned()).collect();
    Ok(serde_json::json!({"lines": lines}))
}

pub async fn run_ai(input: &serde_json::Value, endpoint: &str, key: &str) -> Result<serde_json::Value> {
    let client = Client::new();
    let resp = client
        .post(endpoint)
        .bearer_auth(key)
        .json(input)
        .send()
        .await?;
    Ok(resp.json().await?)
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
