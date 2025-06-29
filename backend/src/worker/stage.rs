use serde::Deserialize;
use serde_json::Value;

/// A single pipeline step loaded from the database.
///
/// Stages can define custom commands or OCR parameters used during
/// processing.
#[derive(Deserialize, Debug, Clone)]
pub struct Stage {
    #[serde(rename = "type")]
    pub stage_type: String,
    pub command: Option<String>,
    pub prompt_name: Option<String>,

    // New fields for OCR stage specific configuration
    pub ocr_engine: Option<String>, // e.g., "default", "external"
    pub ocr_stage_endpoint: Option<String>,
    pub ocr_stage_key: Option<String>,

    // New generic config field for structured configurations
    pub config: Option<Value>,
}
