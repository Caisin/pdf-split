use std::path::MAIN_SEPARATOR;
use std::path::Path;

use kx_pdf::Pdfs;
use tauri::WebviewWindow;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::models::{ExtractImagesResult, SplitPdfResult, WatermarkPdfResult};

#[tauri::command]
pub fn select_pdf_file(window: WebviewWindow) -> Result<Option<String>, String> {
    let file = window
        .dialog()
        .file()
        .add_filter("PDF", &["pdf"])
        .blocking_pick_file();

    match file {
        Some(file) => Ok(Some(dialog_path_to_string(file)?)),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn select_output_dir(window: WebviewWindow) -> Result<Option<String>, String> {
    let folder = window.dialog().file().blocking_pick_folder();

    match folder {
        Some(folder) => Ok(Some(dialog_path_to_string(folder)?)),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn split_pdf_to_images(
    input_path: String,
    output_dir: String,
    image_format: String,
) -> Result<SplitPdfResult, String> {
    let input_path = require_value("PDF 文件", input_path)?;
    let output_dir = require_value("输出目录", output_dir)?;
    let image_format = require_value("图片格式", image_format)?;

    let result = Pdfs::render_pages_to_images(
        &input_path,
        Path::new(&output_dir),
        &image_format.to_ascii_lowercase(),
    )
    .map_err(|err| err.to_string())?;

    Ok(SplitPdfResult {
        page_count: result.page_count,
        generated_file_count: result.generated_files.len(),
        output_dir,
    })
}

#[tauri::command]
pub fn add_text_watermark(
    input_path: String,
    output_dir: String,
    watermark_text: String,
    watermark_font_size: f32,
) -> Result<WatermarkPdfResult, String> {
    let input_path = require_value("PDF 文件", input_path)?;
    let output_dir = require_value("输出目录", output_dir)?;
    let watermark_text = require_value("水印文字", watermark_text)?;
    let watermark_font_size = require_positive_number("水印字号", watermark_font_size)?;

    let output_pdf_path = Pdfs::add_text_watermark(
        &input_path,
        Path::new(&output_dir),
        &watermark_text,
        watermark_font_size,
    )
    .map_err(|err| err.to_string())?;

    Ok(WatermarkPdfResult { output_pdf_path })
}

#[tauri::command]
pub fn extract_embedded_images(
    input_path: String,
    output_dir: String,
) -> Result<ExtractImagesResult, String> {
    let input_path = require_value("PDF 文件", input_path)?;
    let output_dir = require_value("输出目录", output_dir)?;
    let output_dir_for_pdf = ensure_trailing_separator(&output_dir);

    Pdfs::extra_img(&input_path, &output_dir_for_pdf).map_err(|err| err.to_string())?;

    Ok(ExtractImagesResult { output_dir })
}

fn require_value(label: &str, value: String) -> Result<String, String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(format!("{label}不能为空"));
    }

    Ok(trimmed)
}

fn require_positive_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{label}必须大于 0"));
    }

    Ok(value)
}

fn dialog_path_to_string(file_path: FilePath) -> Result<String, String> {
    let path = file_path
        .into_path()
        .map_err(|_| "无法读取系统选择结果".to_string())?;

    path.into_os_string()
        .into_string()
        .map_err(|_| "选择的路径不是有效的 UTF-8".to_string())
}

fn ensure_trailing_separator(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    format!("{trimmed}{MAIN_SEPARATOR}")
}

#[cfg(test)]
mod tests {
    use super::{add_text_watermark, extract_embedded_images, split_pdf_to_images};

    #[test]
    fn split_command_rejects_empty_input_path() {
        let err = split_pdf_to_images("".into(), "/tmp".into(), "png".into())
            .expect_err("empty input path should fail");

        assert!(err.contains("PDF 文件"));
    }

    #[test]
    fn watermark_command_rejects_empty_text() {
        let err = add_text_watermark("a.pdf".into(), "/tmp".into(), "".into(), 28.0)
            .expect_err("empty watermark text should fail");

        assert!(err.contains("水印文字"));
    }

    #[test]
    fn watermark_command_rejects_non_positive_font_size() {
        let err = add_text_watermark("a.pdf".into(), "/tmp".into(), "wm".into(), 0.0)
            .expect_err("non-positive font size should fail");

        assert!(err.contains("水印字号"));
    }

    #[test]
    fn extract_command_rejects_empty_output_dir() {
        let err = extract_embedded_images("a.pdf".into(), "".into())
            .expect_err("empty output dir should fail");

        assert!(err.contains("输出目录"));
    }
}
