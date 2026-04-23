use std::fs;
use std::path::{Path, PathBuf};

use kx_pdf::Pdfs;
use tauri::{Emitter, EventTarget, Manager, WebviewWindow};

use crate::models::{
    BatchPdfTextWatermarkInput, BatchPdfWatermarkPreviewInput, BatchPdfWatermarkProgressPayload,
    BatchPdfWatermarkResult, ExtractImagesResult, InputDirectoryPdfListResult,
    PdfTextWatermarkInput, PreviewImageBytesResult, SplitPdfResult, WatermarkPdfResult,
};

use super::shared::{
    build_pdf_text_watermark_options, canonicalize_existing_directory,
    ensure_batch_output_directory, ensure_distinct_directories, ensure_trailing_separator,
    make_temp_preview_dir, require_value,
};

const BATCH_PDF_WATERMARK_PROGRESS_EVENT: &str = "batch-pdf-watermark-progress";

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

pub fn add_text_watermark(payload: PdfTextWatermarkInput) -> Result<WatermarkPdfResult, String> {
    let input_path = require_value("PDF 文件", payload.input_path)?;
    let output_dir = require_value("输出目录", payload.output_dir)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let options = build_pdf_text_watermark_options(
        &watermark_text,
        payload.watermark_long_edge_font_ratio,
        payload.watermark_opacity,
        payload.watermark_rotation_degrees,
        payload.watermark_stripe_gap_chars,
        payload.watermark_row_gap_lines,
    )?;

    let output_pdf_path = Pdfs::add_text_watermark(&input_path, Path::new(&output_dir), &options)
        .map_err(|err| err.to_string())?;

    Ok(WatermarkPdfResult { output_pdf_path })
}

pub async fn add_text_watermark_to_pdfs(
    window: WebviewWindow,
    payload: BatchPdfTextWatermarkInput,
) -> Result<BatchPdfWatermarkResult, String> {
    let window_label = window.label().to_string();
    let app_handle = window.app_handle().clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_batch_pdf_watermark(payload, |progress| {
            let _ = app_handle.emit_to(
                EventTarget::webview_window(window_label.clone()),
                BATCH_PDF_WATERMARK_PROGRESS_EVENT,
                BatchPdfWatermarkProgressPayload {
                    scanned_file_count: progress.scanned_file_count,
                    processed_file_count: progress.processed_file_count,
                    success_count: progress.success_count,
                    failure_count: progress.failure_count,
                    skipped_count: progress.skipped_count,
                    current_file: progress.current_file,
                },
            );
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

pub fn list_input_directory_pdfs(input_dir: String) -> Result<InputDirectoryPdfListResult, String> {
    Ok(InputDirectoryPdfListResult {
        files: list_previewable_pdfs(&input_dir)?,
    })
}

pub async fn generate_input_directory_pdf_preview(
    payload: BatchPdfWatermarkPreviewInput,
) -> Result<PreviewImageBytesResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(PreviewImageBytesResult {
            bytes: generate_pdf_preview_image_bytes(payload)?,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

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

pub(super) fn run_batch_pdf_watermark<F>(
    payload: BatchPdfTextWatermarkInput,
    mut on_progress: F,
) -> Result<BatchPdfWatermarkResult, String>
where
    F: FnMut(BatchPdfWatermarkProgressPayload),
{
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let output_dir = require_value("输出目录", payload.output_dir)?;
    ensure_distinct_directories(&input_dir, &output_dir)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let options = build_pdf_text_watermark_options(
        &watermark_text,
        payload.watermark_long_edge_font_ratio,
        payload.watermark_opacity,
        payload.watermark_rotation_degrees,
        payload.watermark_stripe_gap_chars,
        payload.watermark_row_gap_lines,
    )?;

    let input_root = canonicalize_existing_directory("输入目录", &input_dir)?;
    let output_root = ensure_batch_output_directory(&input_root, &output_dir)?;
    let source_files = collect_pdf_files(input_root.as_path())?;
    if source_files.is_empty() {
        return Err("输入目录内未找到 PDF 文件".to_string());
    }

    on_progress(BatchPdfWatermarkProgressPayload {
        scanned_file_count: source_files.len(),
        processed_file_count: 0,
        success_count: 0,
        failure_count: 0,
        skipped_count: 0,
        current_file: None,
    });

    let mut result = BatchPdfWatermarkResult {
        scanned_file_count: source_files.len(),
        success_count: 0,
        failure_count: 0,
        skipped_count: 0,
        output_dir: output_root.to_string_lossy().into_owned(),
    };

    for source_path in source_files {
        let relative_path = source_path
            .strip_prefix(input_root.as_path())
            .map_err(|err| err.to_string())?;
        let relative_display = relative_path.to_string_lossy().replace('\\', "/");
        let output_path = build_batch_pdf_output_path(output_root.as_path(), relative_path)?;
        let target_dir = match relative_path.parent() {
            Some(parent) => output_root.join(parent),
            None => output_root.clone(),
        };

        if output_path.exists() {
            result.skipped_count += 1;
        } else {
            match Pdfs::add_text_watermark(
                source_path.to_string_lossy().as_ref(),
                target_dir.as_path(),
                &options,
            ) {
                Ok(_) => result.success_count += 1,
                Err(_) => result.failure_count += 1,
            }
        }

        on_progress(BatchPdfWatermarkProgressPayload {
            scanned_file_count: result.scanned_file_count,
            processed_file_count: result.success_count
                + result.failure_count
                + result.skipped_count,
            success_count: result.success_count,
            failure_count: result.failure_count,
            skipped_count: result.skipped_count,
            current_file: Some(relative_display),
        });
    }

    Ok(result)
}

pub(super) fn list_previewable_pdfs(input_dir: &str) -> Result<Vec<String>, String> {
    let input_dir = require_value("输入目录", input_dir.to_string())?;
    let root = PathBuf::from(&input_dir);
    if !root.is_dir() {
        return Err("输入目录不存在或不是有效目录".to_string());
    }

    collect_pdf_files(root.as_path())?
        .into_iter()
        .map(|path| {
            path.strip_prefix(root.as_path())
                .map(|relative_path| relative_path.to_string_lossy().replace('\\', "/"))
                .map_err(|err| err.to_string())
        })
        .collect()
}

fn collect_pdf_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_pdf_files_recursive(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_pdf_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_pdf_files_recursive(&path, files)?;
            continue;
        }

        if is_preview_pdf_path(path.as_path()) {
            files.push(path);
        }
    }

    Ok(())
}

fn build_batch_pdf_output_path(
    output_root: &Path,
    relative_path: &Path,
) -> Result<PathBuf, String> {
    let stem = relative_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("无法识别 PDF 文件名：{}", relative_path.display()))?;
    let file_name = format!("{stem}-watermarked.pdf");

    Ok(match relative_path.parent() {
        Some(parent) => output_root.join(parent).join(file_name),
        None => output_root.join(file_name),
    })
}

pub(super) fn generate_pdf_preview_image_bytes(
    payload: BatchPdfWatermarkPreviewInput,
) -> Result<Vec<u8>, String> {
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let root = PathBuf::from(&input_dir)
        .canonicalize()
        .map_err(|_| "输入目录不存在或不是有效目录".to_string())?;
    let source_path = resolve_preview_pdf_path(root.as_path(), &payload.relative_path)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let options = build_pdf_text_watermark_options(
        &watermark_text,
        payload.watermark_long_edge_font_ratio,
        payload.watermark_opacity,
        payload.watermark_rotation_degrees,
        payload.watermark_stripe_gap_chars,
        payload.watermark_row_gap_lines,
    )?;

    let temp_preview_dir = make_temp_preview_dir("pdf-split-pdf-preview");
    let result = (|| {
        let watermarked_output_dir = temp_preview_dir.join("watermarked");
        let output_pdf_path = Pdfs::add_text_watermark(
            source_path.to_string_lossy().as_ref(),
            watermarked_output_dir.as_path(),
            &options,
        )
        .map_err(|err| err.to_string())?;
        let rendered_output_dir = temp_preview_dir.join("rendered");
        let render_result =
            Pdfs::render_pages_to_images(&output_pdf_path, rendered_output_dir.as_path(), "png")
                .map_err(|err| err.to_string())?;
        let first_image_path = render_result
            .generated_files
            .first()
            .ok_or_else(|| "PDF 预览生成失败：未生成预览图片".to_string())?;
        fs::read(first_image_path).map_err(|err| err.to_string())
    })();
    let _ = fs::remove_dir_all(&temp_preview_dir);
    result
}

fn resolve_preview_pdf_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    if !root.is_dir() {
        return Err("输入目录不存在或不是有效目录".to_string());
    }

    let relative_path = require_value("预览 PDF", relative_path.to_string())?;
    let candidate = Path::new(&relative_path);
    if candidate.is_absolute() {
        return Err("预览 PDF 路径不合法".to_string());
    }

    let preview_path = root.join(candidate);
    let preview_path = preview_path
        .canonicalize()
        .map_err(|_| "预览 PDF 不存在".to_string())?;
    if !preview_path.starts_with(root)
        || !preview_path.is_file()
        || !is_preview_pdf_path(&preview_path)
    {
        return Err("预览 PDF 路径不合法".to_string());
    }

    Ok(preview_path)
}

fn is_preview_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}
