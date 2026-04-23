use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use kx_image::{ImageFormat, Imgs};
use tauri::{Emitter, EventTarget, Manager, WebviewWindow};

use crate::models::{
    BatchImageWatermarkInput, BatchImageWatermarkPreviewInput, BatchImageWatermarkProgressPayload,
    BatchImageWatermarkResult, InputDirectoryImageListResult, PreviewImageBytesResult,
};

use super::shared::{
    build_slanted_watermark_options, canonicalize_existing_directory,
    ensure_batch_output_directory, ensure_distinct_directories, require_value,
};

const BATCH_IMAGE_WATERMARK_PROGRESS_EVENT: &str = "batch-image-watermark-progress";

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
                    skipped_count: progress.skipped_count,
                    current_file: progress.current_file,
                },
            );
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

pub fn list_input_directory_images(
    input_dir: String,
) -> Result<InputDirectoryImageListResult, String> {
    Ok(InputDirectoryImageListResult {
        files: list_previewable_images(&input_dir)?,
    })
}

pub async fn generate_input_directory_image_preview(
    payload: BatchImageWatermarkPreviewInput,
) -> Result<PreviewImageBytesResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(PreviewImageBytesResult {
            bytes: generate_preview_image_bytes(payload)?,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

pub(super) fn run_batch_image_watermark<F>(
    payload: BatchImageWatermarkInput,
    mut on_progress: F,
) -> Result<BatchImageWatermarkResult, String>
where
    F: FnMut(BatchImageWatermarkProgressPayload),
{
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let output_dir = require_value("输出目录", payload.output_dir)?;
    ensure_distinct_directories(&input_dir, &output_dir)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let options = build_slanted_watermark_options(
        &watermark_text,
        payload.watermark_line_count,
        payload.watermark_full_screen,
        payload.watermark_opacity,
        -1.0_f32.to_degrees(),
        payload.watermark_stripe_gap_chars,
        payload.watermark_row_gap_lines,
    )?;
    let input_root = canonicalize_existing_directory("输入目录", &input_dir)?;
    let output_root = ensure_batch_output_directory(input_root.as_path(), &output_dir)?;
    let source_files = collect_image_files(input_root.as_path())?;
    if source_files.is_empty() {
        return Err("输入目录内未找到可处理图片".to_string());
    }

    on_progress(BatchImageWatermarkProgressPayload {
        scanned_file_count: source_files.len(),
        processed_file_count: 0,
        success_count: 0,
        failure_count: 0,
        skipped_count: 0,
        current_file: None,
    });

    let mut result = BatchImageWatermarkResult {
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
        let output_path = output_root.join(relative_path);

        if output_path.exists() {
            result.skipped_count += 1;
        } else {
            match render_watermarked_image_to_path(
                source_path.as_path(),
                output_path.as_path(),
                &options,
            ) {
                Ok(()) => result.success_count += 1,
                Err(_) => result.failure_count += 1,
            }
        }

        on_progress(BatchImageWatermarkProgressPayload {
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

pub(super) fn list_previewable_images(input_dir: &str) -> Result<Vec<String>, String> {
    let input_dir = require_value("输入目录", input_dir.to_string())?;
    let root = PathBuf::from(&input_dir);
    if !root.is_dir() {
        return Err("输入目录不存在或不是有效目录".to_string());
    }

    let mut files = Vec::new();
    collect_previewable_images(&root, &root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_image_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_image_files_recursive(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_image_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_image_files_recursive(&path, files)?;
            continue;
        }

        if is_preview_image_path(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn collect_previewable_images(
    root: &Path,
    current_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(current_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_previewable_images(root, &path, files)?;
            continue;
        }

        if !is_preview_image_path(&path) {
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .map_err(|err| err.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        files.push(relative_path);
    }

    Ok(())
}

pub(super) fn generate_preview_image_bytes(
    payload: BatchImageWatermarkPreviewInput,
) -> Result<Vec<u8>, String> {
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let root = PathBuf::from(&input_dir)
        .canonicalize()
        .map_err(|_| "输入目录不存在或不是有效目录".to_string())?;
    let source_path = resolve_preview_image_path(root.as_path(), &payload.relative_path)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let options = build_slanted_watermark_options(
        &watermark_text,
        payload.watermark_line_count,
        payload.watermark_full_screen,
        payload.watermark_opacity,
        -1.0_f32.to_degrees(),
        payload.watermark_stripe_gap_chars,
        payload.watermark_row_gap_lines,
    )?;
    let rendered = render_watermarked_image(source_path.as_path(), &options)?;
    let mut bytes = Vec::new();
    rendered
        .write_to(
            &mut Cursor::new(&mut bytes),
            image_format_from_path(source_path.as_path())?,
        )
        .map_err(|err| err.to_string())?;
    Ok(bytes)
}

fn resolve_preview_image_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    if !root.is_dir() {
        return Err("输入目录不存在或不是有效目录".to_string());
    }

    let relative_path = require_value("预览图片", relative_path.to_string())?;
    let candidate = Path::new(&relative_path);
    if candidate.is_absolute() {
        return Err("预览图片路径不合法".to_string());
    }

    let preview_path = root.join(candidate);
    let preview_path = preview_path
        .canonicalize()
        .map_err(|_| "预览图片不存在".to_string())?;
    if !preview_path.starts_with(root)
        || !preview_path.is_file()
        || !is_preview_image_path(&preview_path)
    {
        return Err("预览图片路径不合法".to_string());
    }

    Ok(preview_path)
}

fn is_preview_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tif" | "tiff"
            )
        })
        .unwrap_or(false)
}

pub(super) fn render_watermarked_image(
    source_path: &Path,
    options: &kx_image::SlantedWatermarkOptions<'_>,
) -> Result<kx_image::DynamicImage, String> {
    let image = kx_image::open(source_path).map_err(|err| err.to_string())?;
    Imgs::watermark()
        .slanted(options.text, options.line_count)
        .full_screen(options.full_screen)
        .opacity(options.opacity)
        .rotation_degrees(options.rotation_degrees)
        .stripe_gap_chars(options.stripe_gap_chars)
        .row_gap_lines(options.row_gap_lines)
        .render_image(image)
        .map_err(|err| err.to_string())
}

fn render_watermarked_image_to_path(
    source_path: &Path,
    output_path: &Path,
    options: &kx_image::SlantedWatermarkOptions<'_>,
) -> Result<(), String> {
    if output_path.exists() {
        return Err(format!("目标文件已存在：{}", output_path.display()));
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let rendered = render_watermarked_image(source_path, options)?;
    save_rendered_image(rendered, output_path)
}

fn save_rendered_image(image: kx_image::DynamicImage, output_path: &Path) -> Result<(), String> {
    match image_format_from_path(output_path)? {
        ImageFormat::Jpeg => image
            .to_rgb8()
            .save_with_format(output_path, ImageFormat::Jpeg)
            .map_err(|err| err.to_string()),
        format => image
            .save_with_format(output_path, format)
            .map_err(|err| err.to_string()),
    }
}

fn image_format_from_path(path: &Path) -> Result<ImageFormat, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| format!("无法识别图片格式：{}", path.display()))?;

    match extension.as_str() {
        "png" => Ok(ImageFormat::Png),
        "jpg" | "jpeg" => Ok(ImageFormat::Jpeg),
        "webp" => Ok(ImageFormat::WebP),
        "bmp" => Ok(ImageFormat::Bmp),
        "tif" | "tiff" => Ok(ImageFormat::Tiff),
        other => Err(format!("不支持的图片格式：{other}")),
    }
}
