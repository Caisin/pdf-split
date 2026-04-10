use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitPdfResult {
    pub page_count: usize,
    pub generated_file_count: usize,
    pub output_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkPdfResult {
    pub output_pdf_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfTextWatermarkInput {
    pub input_path: String,
    pub output_dir: String,
    pub watermark_text: String,
    pub watermark_font_size: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractImagesResult {
    pub output_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImageWatermarkResult {
    pub scanned_file_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub output_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDirectoryImageListResult {
    pub files: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImageBytesResult {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImageWatermarkProgressPayload {
    pub scanned_file_count: usize,
    pub processed_file_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub current_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImageWatermarkInput {
    pub input_dir: String,
    pub output_dir: String,
    pub watermark_text: String,
    pub watermark_long_edge_font_ratio: f32,
    pub watermark_opacity: f32,
    pub watermark_rotation: f32,
    pub watermark_horizontal_spacing_ratio: f32,
    pub watermark_vertical_spacing_ratio: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImageWatermarkPreviewInput {
    pub input_dir: String,
    pub relative_path: String,
    pub watermark_text: String,
    pub watermark_long_edge_font_ratio: f32,
    pub watermark_opacity: f32,
    pub watermark_rotation: f32,
    pub watermark_horizontal_spacing_ratio: f32,
    pub watermark_vertical_spacing_ratio: f32,
}
