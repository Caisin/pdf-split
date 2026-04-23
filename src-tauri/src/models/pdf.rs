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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPdfWatermarkResult {
    pub scanned_file_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_count: usize,
    pub output_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDirectoryPdfListResult {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPdfWatermarkProgressPayload {
    pub scanned_file_count: usize,
    pub processed_file_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_count: usize,
    pub current_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfTextWatermarkInput {
    pub input_path: String,
    pub output_dir: String,
    pub watermark_text: String,
    pub watermark_long_edge_font_ratio: f32,
    pub watermark_opacity: f32,
    pub watermark_rotation_degrees: f32,
    pub watermark_stripe_gap_chars: f32,
    pub watermark_row_gap_lines: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPdfTextWatermarkInput {
    pub input_dir: String,
    pub output_dir: String,
    pub watermark_text: String,
    pub watermark_long_edge_font_ratio: f32,
    pub watermark_opacity: f32,
    pub watermark_rotation_degrees: f32,
    pub watermark_stripe_gap_chars: f32,
    pub watermark_row_gap_lines: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPdfWatermarkPreviewInput {
    pub input_dir: String,
    pub relative_path: String,
    pub watermark_text: String,
    pub watermark_long_edge_font_ratio: f32,
    pub watermark_opacity: f32,
    pub watermark_rotation_degrees: f32,
    pub watermark_stripe_gap_chars: f32,
    pub watermark_row_gap_lines: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractImagesResult {
    pub output_dir: String,
}
