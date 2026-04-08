use std::path::MAIN_SEPARATOR;
use std::path::Path;

use kx_image::{BatchImageWatermarkOptions, BatchImageWatermarkProgress, Imgs};
use kx_pdf::{PdfTextWatermarkOptions, Pdfs};
use tauri::{Emitter, EventTarget, Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::models::{
    BatchImageWatermarkInput, BatchImageWatermarkProgressPayload, BatchImageWatermarkResult,
    ExtractImagesResult, PdfTextWatermarkInput, SplitPdfResult, WatermarkPdfResult,
};

const BATCH_IMAGE_WATERMARK_PROGRESS_EVENT: &str = "batch-image-watermark-progress";

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
pub fn add_text_watermark(payload: PdfTextWatermarkInput) -> Result<WatermarkPdfResult, String> {
    let input_path = require_value("PDF 文件", payload.input_path)?;
    let output_dir = require_value("输出目录", payload.output_dir)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let watermark_font_size = require_positive_number("水印字号", payload.watermark_font_size)?;
    let options = PdfTextWatermarkOptions {
        watermark_text: &watermark_text,
        font_size: watermark_font_size,
    };

    let output_pdf_path = Pdfs::add_text_watermark(&input_path, Path::new(&output_dir), &options)
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

#[tauri::command]
pub async fn add_text_watermark_to_images(
    window: WebviewWindow,
    payload: BatchImageWatermarkInput,
) -> Result<BatchImageWatermarkResult, String> {
    let window_label = window.label().to_string();
    let app_handle = window.app_handle().clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_batch_image_watermark(payload, |progress| {
            let _ = app_handle.emit_to(
                EventTarget::webview_window(window_label.clone()),
                BATCH_IMAGE_WATERMARK_PROGRESS_EVENT,
                BatchImageWatermarkProgressPayload {
                    scanned_file_count: progress.scanned_file_count,
                    processed_file_count: progress.processed_file_count,
                    success_count: progress.success_count,
                    failure_count: progress.failure_count,
                    current_file: progress.current_file,
                },
            );
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

fn run_batch_image_watermark<F>(
    payload: BatchImageWatermarkInput,
    on_progress: F,
) -> Result<BatchImageWatermarkResult, String>
where
    F: FnMut(BatchImageWatermarkProgress),
{
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let output_dir = require_value("输出目录", payload.output_dir)?;
    ensure_distinct_directories(&input_dir, &output_dir)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let watermark_font_size = require_positive_number("水印字号", payload.watermark_font_size)?;
    let watermark_opacity = require_percentage("水印透明度", payload.watermark_opacity)?;
    let watermark_rotation = require_finite_number("水印角度", payload.watermark_rotation)?;
    let watermark_horizontal_spacing =
        require_spacing("横向间距", payload.watermark_horizontal_spacing)?;
    let watermark_vertical_spacing =
        require_spacing("纵向间距", payload.watermark_vertical_spacing)?;
    let options = BatchImageWatermarkOptions {
        watermark_text: &watermark_text,
        font_size: watermark_font_size,
        opacity: watermark_opacity / 100.0,
        rotation_degrees: watermark_rotation,
        horizontal_spacing: watermark_horizontal_spacing,
        vertical_spacing: watermark_vertical_spacing,
    };

    let result = Imgs::add_text_watermark_to_images_with_progress(
        Path::new(&input_dir),
        Path::new(&output_dir),
        &options,
        on_progress,
    )
    .map_err(|err| err.to_string())?;

    Ok(BatchImageWatermarkResult {
        scanned_file_count: result.scanned_file_count,
        success_count: result.success_count,
        failure_count: result.failure_count,
        output_dir,
    })
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

fn require_percentage(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() || value <= 0.0 || value > 100.0 {
        return Err(format!("{label}必须在 0 到 100 之间"));
    }

    Ok(value)
}

fn require_finite_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() {
        return Err(format!("{label}必须是有效数字"));
    }

    Ok(value)
}

fn require_spacing(label: &str, value: u32) -> Result<u32, String> {
    if value > 4096 {
        return Err(format!("{label}不能大于 4096"));
    }

    Ok(value)
}

fn ensure_distinct_directories(input_dir: &str, output_dir: &str) -> Result<(), String> {
    if normalize_directory_for_compare(input_dir) == normalize_directory_for_compare(output_dir) {
        return Err("输入目录与输出目录不能相同".to_string());
    }

    Ok(())
}

fn normalize_directory_for_compare(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
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
    use super::{
        add_text_watermark, extract_embedded_images, run_batch_image_watermark, split_pdf_to_images,
    };
    use crate::models::{BatchImageWatermarkInput, PdfTextWatermarkInput};

    #[test]
    fn split_command_rejects_empty_input_path() {
        let err = split_pdf_to_images("".into(), "/tmp".into(), "png".into())
            .expect_err("empty input path should fail");

        assert!(err.contains("PDF 文件"));
    }

    #[test]
    fn watermark_command_rejects_empty_text() {
        let err = add_text_watermark(PdfTextWatermarkInput {
            input_path: "a.pdf".into(),
            output_dir: "/tmp".into(),
            watermark_text: "".into(),
            watermark_font_size: 28.0,
        })
        .expect_err("empty watermark text should fail");

        assert!(err.contains("水印文字"));
    }

    #[test]
    fn watermark_command_rejects_non_positive_font_size() {
        let err = add_text_watermark(PdfTextWatermarkInput {
            input_path: "a.pdf".into(),
            output_dir: "/tmp".into(),
            watermark_text: "wm".into(),
            watermark_font_size: 0.0,
        })
        .expect_err("non-positive font size should fail");

        assert!(err.contains("水印字号"));
    }

    #[test]
    fn extract_command_rejects_empty_output_dir() {
        let err = extract_embedded_images("a.pdf".into(), "".into())
            .expect_err("empty output dir should fail");

        assert!(err.contains("输出目录"));
    }

    #[test]
    fn batch_image_watermark_command_rejects_same_input_and_output_dir() {
        let err = run_batch_image_watermark(
            BatchImageWatermarkInput {
                input_dir: "/tmp/images".into(),
                output_dir: "/tmp/images/".into(),
                watermark_text: "wm".into(),
                watermark_font_size: 28.0,
                watermark_opacity: 18.0,
                watermark_rotation: -35.0,
                watermark_horizontal_spacing: 180,
                watermark_vertical_spacing: 120,
            },
            |_| {},
        )
        .expect_err("same directories should fail");

        assert!(err.contains("输入目录与输出目录不能相同"));
    }

    #[test]
    fn batch_image_watermark_command_rejects_invalid_opacity() {
        let err = run_batch_image_watermark(
            BatchImageWatermarkInput {
                input_dir: "/tmp/in".into(),
                output_dir: "/tmp/out".into(),
                watermark_text: "wm".into(),
                watermark_font_size: 28.0,
                watermark_opacity: 0.0,
                watermark_rotation: -35.0,
                watermark_horizontal_spacing: 180,
                watermark_vertical_spacing: 120,
            },
            |_| {},
        )
        .expect_err("zero opacity should fail");

        assert!(err.contains("水印透明度"));
    }

    #[test]
    fn batch_image_watermark_command_rejects_too_large_spacing() {
        let err = run_batch_image_watermark(
            BatchImageWatermarkInput {
                input_dir: "/tmp/in".into(),
                output_dir: "/tmp/out".into(),
                watermark_text: "wm".into(),
                watermark_font_size: 28.0,
                watermark_opacity: 18.0,
                watermark_rotation: -35.0,
                watermark_horizontal_spacing: 4_097,
                watermark_vertical_spacing: 120,
            },
            |_| {},
        )
        .expect_err("too large spacing should fail");

        assert!(err.contains("横向间距"));
    }
}
