use serde::{Deserialize, Serialize};

use super::common::default_slanted_watermark_rotation_degrees;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImageWatermarkResult {
    pub scanned_file_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_count: usize,
    pub output_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDirectoryImageListResult {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImageWatermarkProgressPayload {
    pub scanned_file_count: usize,
    pub processed_file_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_count: usize,
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
