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
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPdfWatermarkProgressPayload {
    pub scanned_file_count: usize,
    pub processed_file_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub current_file: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchVideoWatermarkResult {
    pub scanned_file_count: usize,
    pub success_count: usize,
    pub generated_overlay_count: usize,
    pub reused_overlay_count: usize,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchVideoWatermarkProgressPayload {
    pub scanned_file_count: usize,
    pub processed_file_count: usize,
    pub success_count: usize,
    pub generated_overlay_count: usize,
    pub reused_overlay_count: usize,
    pub current_file: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesRecutResult {
    pub generated_file_count: usize,
    pub output_dir: String,
    pub output_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesRecutProgressPayload {
    pub total_count: usize,
    pub processed_count: usize,
    pub current_stage: String,
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
pub struct BatchVideoWatermarkInput {
    pub input_dir: String,
    pub output_dir: String,
    pub watermark_text: String,
    pub watermark_line_count: u32,
    pub watermark_full_screen: bool,
    pub watermark_opacity: f32,
    #[serde(default = "default_slanted_watermark_rotation_degrees")]
    pub watermark_rotation_degrees: f32,
    pub watermark_stripe_gap_chars: f32,
    pub watermark_row_gap_lines: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesRecutInput {
    pub input_dir: String,
    pub output_dir: String,
    pub keep_count: usize,
    pub total_count: usize,
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
pub struct InputDirectoryVideoListResult {
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
    pub watermark_line_count: u32,
    pub watermark_full_screen: bool,
    pub watermark_opacity: f32,
    pub watermark_stripe_gap_chars: f32,
    pub watermark_row_gap_lines: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImageWatermarkPreviewInput {
    pub input_dir: String,
    pub relative_path: String,
    pub watermark_text: String,
    pub watermark_line_count: u32,
    pub watermark_full_screen: bool,
    pub watermark_opacity: f32,
    #[serde(default = "default_slanted_watermark_rotation_degrees")]
    pub watermark_rotation_degrees: f32,
    pub watermark_stripe_gap_chars: f32,
    pub watermark_row_gap_lines: f32,
}

fn default_slanted_watermark_rotation_degrees() -> f32 {
    -1.0_f32.to_degrees()
}
