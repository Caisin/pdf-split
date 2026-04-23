use std::fs;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use kx_image::SlantedWatermarkOptions;
use kx_pdf::PdfTextWatermarkOptions;
use tauri_plugin_dialog::FilePath;

pub(super) const SUPPORTED_VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v", "mkv", "avi", "webm"];

pub(super) fn require_value(label: &str, value: String) -> Result<String, String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(format!("{label}不能为空"));
    }

    Ok(trimmed)
}

pub(super) fn require_positive_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{label}必须大于 0"));
    }

    Ok(value)
}

pub(super) fn require_positive_count(label: &str, value: u32) -> Result<u32, String> {
    if value == 0 {
        return Err(format!("{label}必须大于 0"));
    }

    Ok(value)
}

pub(super) fn require_zero_to_one_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{label}必须在 0 到 1 之间"));
    }

    Ok(value)
}

pub(super) fn require_non_negative_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{label}必须大于等于 0"));
    }

    Ok(value)
}

pub(super) fn require_finite_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() {
        return Err(format!("{label}必须是有效数字"));
    }

    Ok(value)
}

pub(super) fn ensure_distinct_directories(input_dir: &str, output_dir: &str) -> Result<(), String> {
    if normalize_directory_for_compare(input_dir) == normalize_directory_for_compare(output_dir) {
        return Err("输入目录与输出目录不能相同".to_string());
    }

    Ok(())
}

pub(super) fn canonicalize_existing_directory(label: &str, path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(format!("{label}不存在或不是有效目录"));
    }
    if !path.is_dir() {
        return Err(format!("{label}不存在或不是有效目录"));
    }

    path.canonicalize()
        .map_err(|_| format!("{label}不存在或不是有效目录"))
}

pub(super) fn ensure_batch_output_directory(
    input_root: &Path,
    output_dir: &str,
) -> Result<PathBuf, String> {
    let output_root = absolutize_path(Path::new(output_dir))?;
    let comparable_output = if output_root.exists() {
        output_root
            .canonicalize()
            .map_err(|_| "输出目录不存在或不是有效目录".to_string())?
    } else {
        output_root.clone()
    };

    if comparable_output == input_root {
        return Err("输入目录与输出目录不能相同".to_string());
    }
    if comparable_output.starts_with(input_root) {
        return Err("输出目录不能位于输入目录内".to_string());
    }

    fs::create_dir_all(&output_root).map_err(|err| err.to_string())?;
    Ok(output_root)
}

fn normalize_directory_for_compare(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
}

pub(super) fn dialog_path_to_string(file_path: FilePath) -> Result<String, String> {
    let path = file_path
        .into_path()
        .map_err(|_| "无法读取系统选择结果".to_string())?;

    path.into_os_string()
        .into_string()
        .map_err(|_| "选择的路径不是有效的 UTF-8".to_string())
}

pub(super) fn absolutize_path(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("输出目录不能为空".to_string());
    }

    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    std::env::current_dir()
        .map(|current_dir| current_dir.join(path))
        .map_err(|err| err.to_string())
}

pub(super) fn ensure_trailing_separator(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    format!("{trimmed}{MAIN_SEPARATOR}")
}

pub(super) fn make_temp_preview_dir(prefix: &str) -> PathBuf {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{unique_suffix}"));
    let _ = fs::create_dir_all(&path);
    path
}

pub(super) fn build_slanted_watermark_options<'a>(
    watermark_text: &'a str,
    watermark_line_count: u32,
    watermark_full_screen: bool,
    watermark_opacity: f32,
    watermark_rotation_degrees: f32,
    watermark_stripe_gap_chars: f32,
    watermark_row_gap_lines: f32,
) -> Result<SlantedWatermarkOptions<'a>, String> {
    let line_count = require_positive_count("水印行数", watermark_line_count)?;
    let opacity = require_zero_to_one_number("水印透明度", watermark_opacity)?;
    let rotation_degrees = require_finite_number("水印角度", watermark_rotation_degrees)?;
    let stripe_gap_chars = require_non_negative_number("条间距", watermark_stripe_gap_chars)?;
    let row_gap_lines = require_non_negative_number("行间距", watermark_row_gap_lines)?;

    Ok(SlantedWatermarkOptions::new(watermark_text, line_count)
        .with_full_screen(watermark_full_screen)
        .with_opacity(opacity)
        .with_rotation_degrees(rotation_degrees)
        .with_stripe_gap_chars(stripe_gap_chars)
        .with_row_gap_lines(row_gap_lines))
}

pub(super) fn build_pdf_text_watermark_options<'a>(
    watermark_text: &'a str,
    watermark_long_edge_font_ratio: f32,
    watermark_opacity: f32,
    watermark_rotation_degrees: f32,
    watermark_stripe_gap_chars: f32,
    watermark_row_gap_lines: f32,
) -> Result<PdfTextWatermarkOptions<'a>, String> {
    let long_edge_font_ratio =
        require_positive_number("长边字号比例", watermark_long_edge_font_ratio)?;
    let opacity = require_zero_to_one_number("水印透明度", watermark_opacity)?;
    let rotation_degrees = require_finite_number("水印角度", watermark_rotation_degrees)?;
    let stripe_gap_chars = require_non_negative_number("条间距", watermark_stripe_gap_chars)?;
    let row_gap_lines = require_non_negative_number("行间距", watermark_row_gap_lines)?;

    Ok(PdfTextWatermarkOptions::new(watermark_text)
        .with_long_edge_font_ratio(long_edge_font_ratio)
        .with_opacity(opacity)
        .with_rotation_degrees(rotation_degrees)
        .with_stripe_gap_chars(stripe_gap_chars)
        .with_row_gap_lines(row_gap_lines))
}
