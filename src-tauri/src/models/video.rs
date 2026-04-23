use serde::{Deserialize, Serialize};

use super::common::default_slanted_watermark_rotation_degrees;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchVideoWatermarkResult {
    pub scanned_file_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_count: usize,
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
    pub failure_count: usize,
    pub skipped_count: usize,
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
pub struct InputDirectoryVideoListResult {
    pub files: Vec<String>,
}
