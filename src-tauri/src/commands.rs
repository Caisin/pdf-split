use std::fs;
use std::io::Cursor;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use kx_cmds::{
    Cmds,
    traits::video::{CmdVideo, SeriesRecutReq},
};
use kx_image::{
    BatchImageWatermarkProgress, BatchVideoWatermarkProgress, ImageFormat, Imgs,
    SlantedWatermarkOptions,
};
use kx_pdf::{PdfTextWatermarkOptions, Pdfs};
use tauri::{Emitter, EventTarget, Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::models::{
    BatchImageWatermarkInput, BatchImageWatermarkPreviewInput, BatchImageWatermarkProgressPayload,
    BatchImageWatermarkResult, BatchPdfTextWatermarkInput, BatchPdfWatermarkProgressPayload,
    BatchPdfWatermarkResult, BatchVideoWatermarkInput, BatchVideoWatermarkProgressPayload,
    BatchVideoWatermarkResult, ExtractImagesResult, InputDirectoryImageListResult,
    InputDirectoryVideoListResult, PdfTextWatermarkInput, PreviewImageBytesResult,
    SeriesRecutInput, SeriesRecutProgressPayload, SeriesRecutResult, SplitPdfResult,
    WatermarkPdfResult,
};

const BATCH_IMAGE_WATERMARK_PROGRESS_EVENT: &str = "batch-image-watermark-progress";
const BATCH_PDF_WATERMARK_PROGRESS_EVENT: &str = "batch-pdf-watermark-progress";
const BATCH_VIDEO_WATERMARK_PROGRESS_EVENT: &str = "batch-video-watermark-progress";
const SERIES_RECUT_PROGRESS_EVENT: &str = "series-recut-progress";
const SUPPORTED_VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v", "mkv", "avi", "webm"];
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
    let watermark_long_edge_font_ratio =
        require_positive_number("长边字号比例", payload.watermark_long_edge_font_ratio)?;
    let watermark_opacity = require_zero_to_one_number("水印透明度", payload.watermark_opacity)?;
    let watermark_rotation_degrees =
        require_finite_number("水印角度", payload.watermark_rotation_degrees)?;
    let watermark_stripe_gap_chars =
        require_non_negative_number("条间距", payload.watermark_stripe_gap_chars)?;
    let watermark_row_gap_lines =
        require_non_negative_number("行间距", payload.watermark_row_gap_lines)?;
    let options = PdfTextWatermarkOptions::new(&watermark_text)
        .with_long_edge_font_ratio(watermark_long_edge_font_ratio)
        .with_opacity(watermark_opacity)
        .with_rotation_degrees(watermark_rotation_degrees)
        .with_stripe_gap_chars(watermark_stripe_gap_chars)
        .with_row_gap_lines(watermark_row_gap_lines);

    let output_pdf_path = Pdfs::add_text_watermark(&input_path, Path::new(&output_dir), &options)
        .map_err(|err| err.to_string())?;

    Ok(WatermarkPdfResult { output_pdf_path })
}

#[tauri::command]
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
                    current_file: progress.current_file,
                },
            );
        })
    })
    .await
    .map_err(|error| error.to_string())?
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

#[tauri::command]
pub fn list_input_directory_images(
    input_dir: String,
) -> Result<InputDirectoryImageListResult, String> {
    Ok(InputDirectoryImageListResult {
        files: list_previewable_images(&input_dir)?,
    })
}

#[tauri::command]
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

#[tauri::command]
pub fn list_input_directory_videos(
    input_dir: String,
) -> Result<InputDirectoryVideoListResult, String> {
    Ok(InputDirectoryVideoListResult {
        files: list_previewable_videos(&input_dir)?,
    })
}

#[tauri::command]
pub async fn generate_input_directory_video_preview(
    payload: BatchImageWatermarkPreviewInput,
) -> Result<PreviewImageBytesResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(PreviewImageBytesResult {
            bytes: generate_video_preview_image_bytes(payload)?,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn add_slanted_watermark_to_videos(
    window: WebviewWindow,
    payload: BatchVideoWatermarkInput,
) -> Result<BatchVideoWatermarkResult, String> {
    let window_label = window.label().to_string();
    let app_handle = window.app_handle().clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_batch_video_watermark(payload, |progress| {
            let _ = app_handle.emit_to(
                EventTarget::webview_window(window_label.clone()),
                BATCH_VIDEO_WATERMARK_PROGRESS_EVENT,
                BatchVideoWatermarkProgressPayload {
                    scanned_file_count: progress.scanned_file_count,
                    processed_file_count: progress.processed_file_count,
                    success_count: progress.success_count,
                    generated_overlay_count: progress.generated_overlay_count,
                    reused_overlay_count: progress.reused_overlay_count,
                    current_file: progress.current_file,
                },
            );
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn video_recut_series(
    window: WebviewWindow,
    payload: SeriesRecutInput,
) -> Result<SeriesRecutResult, String> {
    let window_label = window.label().to_string();
    let app_handle = window.app_handle().clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_series_recut(payload, |progress| {
            let _ = app_handle.emit_to(
                EventTarget::webview_window(window_label.clone()),
                SERIES_RECUT_PROGRESS_EVENT,
                SeriesRecutProgressPayload {
                    total_count: progress.total_count,
                    processed_count: progress.processed_count,
                    current_stage: progress.current_stage,
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
    mut on_progress: F,
) -> Result<BatchImageWatermarkResult, String>
where
    F: FnMut(BatchImageWatermarkProgress),
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
    let output_root = ensure_batch_output_directory(&input_root, &output_dir)?;
    let source_files = collect_image_files(input_root.as_path())?;
    if source_files.is_empty() {
        return Err("输入目录内未找到可处理图片".to_string());
    }

    on_progress(BatchImageWatermarkProgress {
        scanned_file_count: source_files.len(),
        processed_file_count: 0,
        success_count: 0,
        failure_count: 0,
        current_file: None,
    });

    let mut result = BatchImageWatermarkResult {
        scanned_file_count: source_files.len(),
        success_count: 0,
        failure_count: 0,
        output_dir: output_root.to_string_lossy().into_owned(),
    };

    for source_path in source_files {
        let relative_path = source_path
            .strip_prefix(input_root.as_path())
            .map_err(|err| err.to_string())?;
        let relative_display = relative_path.to_string_lossy().replace('\\', "/");
        let output_path = output_root.join(relative_path);

        match render_watermarked_image_to_path(
            source_path.as_path(),
            output_path.as_path(),
            &options,
        ) {
            Ok(()) => result.success_count += 1,
            Err(_) => result.failure_count += 1,
        }

        on_progress(BatchImageWatermarkProgress {
            scanned_file_count: result.scanned_file_count,
            processed_file_count: result.success_count + result.failure_count,
            success_count: result.success_count,
            failure_count: result.failure_count,
            current_file: Some(relative_display),
        });
    }

    Ok(result)
}

fn run_batch_pdf_watermark<F>(
    payload: BatchPdfTextWatermarkInput,
    mut on_progress: F,
) -> Result<BatchPdfWatermarkResult, String>
where
    F: FnMut(BatchImageWatermarkProgress),
{
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let output_dir = require_value("输出目录", payload.output_dir)?;
    ensure_distinct_directories(&input_dir, &output_dir)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let watermark_long_edge_font_ratio =
        require_positive_number("长边字号比例", payload.watermark_long_edge_font_ratio)?;
    let watermark_opacity = require_zero_to_one_number("水印透明度", payload.watermark_opacity)?;
    let watermark_rotation_degrees =
        require_finite_number("水印角度", payload.watermark_rotation_degrees)?;
    let watermark_stripe_gap_chars =
        require_non_negative_number("条间距", payload.watermark_stripe_gap_chars)?;
    let watermark_row_gap_lines =
        require_non_negative_number("行间距", payload.watermark_row_gap_lines)?;
    let options = PdfTextWatermarkOptions::new(&watermark_text)
        .with_long_edge_font_ratio(watermark_long_edge_font_ratio)
        .with_opacity(watermark_opacity)
        .with_rotation_degrees(watermark_rotation_degrees)
        .with_stripe_gap_chars(watermark_stripe_gap_chars)
        .with_row_gap_lines(watermark_row_gap_lines);

    let input_root = canonicalize_existing_directory("输入目录", &input_dir)?;
    let output_root = ensure_batch_output_directory(&input_root, &output_dir)?;
    let source_files = collect_pdf_files(input_root.as_path())?;
    if source_files.is_empty() {
        return Err("输入目录内未找到 PDF 文件".to_string());
    }

    on_progress(BatchImageWatermarkProgress {
        scanned_file_count: source_files.len(),
        processed_file_count: 0,
        success_count: 0,
        failure_count: 0,
        current_file: None,
    });

    let mut result = BatchPdfWatermarkResult {
        scanned_file_count: source_files.len(),
        success_count: 0,
        failure_count: 0,
        output_dir: output_root.to_string_lossy().into_owned(),
    };

    for source_path in source_files {
        let relative_path = source_path
            .strip_prefix(input_root.as_path())
            .map_err(|err| err.to_string())?;
        let relative_display = relative_path.to_string_lossy().replace('\\', "/");
        let target_dir = match relative_path.parent() {
            Some(parent) => output_root.join(parent),
            None => output_root.clone(),
        };

        match Pdfs::add_text_watermark(
            source_path.to_string_lossy().as_ref(),
            target_dir.as_path(),
            &options,
        ) {
            Ok(_) => result.success_count += 1,
            Err(_) => result.failure_count += 1,
        }

        on_progress(BatchImageWatermarkProgress {
            scanned_file_count: result.scanned_file_count,
            processed_file_count: result.success_count + result.failure_count,
            success_count: result.success_count,
            failure_count: result.failure_count,
            current_file: Some(relative_display),
        });
    }

    Ok(result)
}

fn run_batch_video_watermark<F>(
    payload: BatchVideoWatermarkInput,
    on_progress: F,
) -> Result<BatchVideoWatermarkResult, String>
where
    F: FnMut(BatchVideoWatermarkProgress),
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
        payload.watermark_rotation_degrees,
        payload.watermark_stripe_gap_chars,
        payload.watermark_row_gap_lines,
    )?;

    let upstream = Imgs::overlay_slanted_watermark_onto_videos_with_progress(
        Path::new(&input_dir),
        Path::new(&output_dir),
        &options,
        on_progress,
    )
    .map_err(|err| err.to_string())?;

    let output_dir = absolutize_path(Path::new(&output_dir))?
        .to_string_lossy()
        .into_owned();

    Ok(BatchVideoWatermarkResult {
        scanned_file_count: upstream.scanned_file_count,
        success_count: upstream.success_count,
        generated_overlay_count: upstream.generated_overlay_count,
        reused_overlay_count: upstream.reused_overlay_count,
        output_dir,
    })
}

#[derive(Debug, Clone)]
struct SeriesRecutProgressState {
    total_count: usize,
    processed_count: usize,
    current_stage: String,
    current_file: Option<String>,
}

fn run_series_recut<F>(
    payload: SeriesRecutInput,
    mut on_progress: F,
) -> Result<SeriesRecutResult, String>
where
    F: FnMut(SeriesRecutProgressState),
{
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let output_dir = require_value("输出目录", payload.output_dir)?;
    ensure_distinct_directories(&input_dir, &output_dir)?;
    let keep_count = payload.keep_count;
    let total_count = payload.total_count;
    if total_count < keep_count {
        return Err("目标总集数不能小于前面保留集数".to_string());
    }

    let input_root = canonicalize_existing_directory("输入目录", &input_dir)?;
    let output_root = absolutize_path(Path::new(&output_dir))?;
    on_progress(SeriesRecutProgressState {
        total_count,
        processed_count: 0,
        current_stage: "扫描输入剧集".to_string(),
        current_file: None,
    });
    let _episodes = collect_series_episode_files_for_progress(input_root.as_path())?;

    on_progress(SeriesRecutProgressState {
        total_count,
        processed_count: 0,
        current_stage: "准备切分任务".to_string(),
        current_file: None,
    });

    let input_dir_for_worker = input_dir.clone();
    let output_dir_for_worker = output_dir.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let req = SeriesRecutReq::new(
            &input_dir_for_worker,
            &output_dir_for_worker,
            keep_count,
            total_count,
        );
        let result = Cmds::video_recut_series(&req).map_err(|err| err.to_string());
        let _ = tx.send(result);
    });

    loop {
        match rx.recv_timeout(Duration::from_millis(300)) {
            Ok(result) => {
                let output_files = result?;
                let output_dir = output_root.to_string_lossy().into_owned();
                on_progress(SeriesRecutProgressState {
                    total_count,
                    processed_count: output_files.len().min(total_count),
                    current_stage: "完成".to_string(),
                    current_file: output_files.last().cloned(),
                });
                return Ok(SeriesRecutResult {
                    generated_file_count: output_files.len(),
                    output_dir,
                    output_files,
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let generated_count =
                    count_generated_series_outputs(output_root.as_path()).unwrap_or_default();
                let current_file =
                    latest_generated_series_output(output_root.as_path()).unwrap_or_default();
                on_progress(SeriesRecutProgressState {
                    total_count,
                    processed_count: generated_count.min(total_count),
                    current_stage: "执行剧集切分".to_string(),
                    current_file: current_file.or_else(|| {
                        if generated_count > 0 {
                            Some(format!("{generated_count:02}.mp4"))
                        } else {
                            None
                        }
                    }),
                });
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("剧集切分任务意外中断".to_string());
            }
        }
    }
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

fn require_positive_count(label: &str, value: u32) -> Result<u32, String> {
    if value == 0 {
        return Err(format!("{label}必须大于 0"));
    }

    Ok(value)
}

fn require_zero_to_one_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{label}必须在 0 到 1 之间"));
    }

    Ok(value)
}

fn require_non_negative_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{label}必须大于等于 0"));
    }

    Ok(value)
}

fn require_finite_number(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() {
        return Err(format!("{label}必须是有效数字"));
    }

    Ok(value)
}

fn ensure_distinct_directories(input_dir: &str, output_dir: &str) -> Result<(), String> {
    if normalize_directory_for_compare(input_dir) == normalize_directory_for_compare(output_dir) {
        return Err("输入目录与输出目录不能相同".to_string());
    }

    Ok(())
}

fn canonicalize_existing_directory(label: &str, path: &str) -> Result<PathBuf, String> {
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

fn ensure_batch_output_directory(input_root: &Path, output_dir: &str) -> Result<PathBuf, String> {
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

fn dialog_path_to_string(file_path: FilePath) -> Result<String, String> {
    let path = file_path
        .into_path()
        .map_err(|_| "无法读取系统选择结果".to_string())?;

    path.into_os_string()
        .into_string()
        .map_err(|_| "选择的路径不是有效的 UTF-8".to_string())
}

fn absolutize_path(path: &Path) -> Result<PathBuf, String> {
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

fn ensure_trailing_separator(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    format!("{trimmed}{MAIN_SEPARATOR}")
}

fn list_previewable_images(input_dir: &str) -> Result<Vec<String>, String> {
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

fn list_previewable_videos(input_dir: &str) -> Result<Vec<String>, String> {
    let input_dir = require_value("输入目录", input_dir.to_string())?;
    let root = PathBuf::from(&input_dir);
    if !root.is_dir() {
        return Err("输入目录不存在或不是有效目录".to_string());
    }

    let mut files = Vec::new();
    collect_previewable_videos(&root, &root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_pdf_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_pdf_files_recursive(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_series_episode_files_for_progress(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        if !entry.file_type().map_err(|err| err.to_string())?.is_file() {
            continue;
        }

        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if stem.parse::<usize>().is_ok() {
            files.push(path);
        }
    }
    if files.is_empty() {
        return Err("输入目录中没有可处理的视频文件".to_string());
    }
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

        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false)
        {
            files.push(path);
        }
    }

    Ok(())
}

fn collect_image_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_image_files_recursive(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn count_generated_series_outputs(output_dir: &Path) -> Result<usize, String> {
    if !output_dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in fs::read_dir(output_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if !entry.file_type().map_err(|err| err.to_string())?.is_file() {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if stem.parse::<usize>().is_ok() {
            count += 1;
        }
    }

    Ok(count)
}

fn latest_generated_series_output(output_dir: &Path) -> Result<Option<String>, String> {
    if !output_dir.exists() {
        return Ok(None);
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(output_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if !entry.file_type().map_err(|err| err.to_string())?.is_file() {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if stem.parse::<usize>().is_ok() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files
        .last()
        .and_then(|path| path.file_name())
        .map(|value| value.to_string_lossy().into_owned()))
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

fn collect_previewable_videos(
    root: &Path,
    current_dir: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(current_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_previewable_videos(root, &path, files)?;
            continue;
        }

        if !is_preview_video_path(&path) {
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

fn generate_preview_image_bytes(
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

fn generate_video_preview_image_bytes(
    payload: BatchImageWatermarkPreviewInput,
) -> Result<Vec<u8>, String> {
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let root = PathBuf::from(&input_dir)
        .canonicalize()
        .map_err(|_| "输入目录不存在或不是有效目录".to_string())?;
    let source_path = resolve_preview_video_path(root.as_path(), &payload.relative_path)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let options = build_slanted_watermark_options(
        &watermark_text,
        payload.watermark_line_count,
        payload.watermark_full_screen,
        payload.watermark_opacity,
        payload.watermark_rotation_degrees,
        payload.watermark_stripe_gap_chars,
        payload.watermark_row_gap_lines,
    )?;

    let temp_frame_dir = make_temp_preview_dir("pdf-split-video-preview");
    let frame_path = temp_frame_dir.join("frame.png");
    extract_video_first_frame(source_path.as_path(), frame_path.as_path())?;
    let rendered = render_watermarked_image(frame_path.as_path(), &options)?;
    let mut bytes = Vec::new();
    rendered
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|err| err.to_string())?;
    let _ = fs::remove_dir_all(&temp_frame_dir);
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

fn resolve_preview_video_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    if !root.is_dir() {
        return Err("输入目录不存在或不是有效目录".to_string());
    }

    let relative_path = require_value("预览视频", relative_path.to_string())?;
    let candidate = Path::new(&relative_path);
    if candidate.is_absolute() {
        return Err("预览视频路径不合法".to_string());
    }

    let preview_path = root.join(candidate);
    let preview_path = preview_path
        .canonicalize()
        .map_err(|_| "预览视频不存在".to_string())?;
    if !preview_path.starts_with(root)
        || !preview_path.is_file()
        || !is_preview_video_path(&preview_path)
    {
        return Err("预览视频路径不合法".to_string());
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

fn is_preview_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            SUPPORTED_VIDEO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
        .unwrap_or(false)
}

fn make_temp_preview_dir(prefix: &str) -> PathBuf {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{unique_suffix}"));
    let _ = fs::create_dir_all(&path);
    path
}

fn extract_video_first_frame(video_path: &Path, output_path: &Path) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            video_path
                .to_str()
                .ok_or_else(|| "预览视频路径不合法".to_string())?,
            "-frames:v",
            "1",
            output_path
                .to_str()
                .ok_or_else(|| "预览输出路径不合法".to_string())?,
        ])
        .output()
        .map_err(|err| err.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "视频首帧提取失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(())
}

fn build_slanted_watermark_options<'a>(
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

fn render_watermarked_image(
    source_path: &Path,
    options: &SlantedWatermarkOptions<'_>,
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
    options: &SlantedWatermarkOptions<'_>,
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

#[cfg(test)]
mod tests {
    use super::{
        add_text_watermark, build_slanted_watermark_options, extract_embedded_images,
        generate_preview_image_bytes, list_previewable_images, render_watermarked_image,
        run_batch_image_watermark, run_batch_pdf_watermark, split_pdf_to_images,
    };
    use crate::models::{
        BatchImageWatermarkInput, BatchImageWatermarkPreviewInput, BatchPdfTextWatermarkInput,
        PdfTextWatermarkInput,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

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
            watermark_long_edge_font_ratio: 0.028,
            watermark_opacity: 50.0 / 255.0,
            watermark_rotation_degrees: -57.29578,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        })
        .expect_err("empty watermark text should fail");

        assert!(err.contains("水印文字"));
    }

    #[test]
    fn watermark_command_rejects_non_positive_long_edge_font_ratio() {
        let err = add_text_watermark(PdfTextWatermarkInput {
            input_path: "a.pdf".into(),
            output_dir: "/tmp".into(),
            watermark_text: "wm".into(),
            watermark_long_edge_font_ratio: 0.0,
            watermark_opacity: 50.0 / 255.0,
            watermark_rotation_degrees: -57.29578,
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        })
        .expect_err("non-positive long edge font ratio should fail");

        assert!(err.contains("长边字号比例"));
    }

    #[test]
    fn batch_pdf_watermark_command_rejects_same_input_and_output_dir() {
        let err = run_batch_pdf_watermark(
            BatchPdfTextWatermarkInput {
                input_dir: "/tmp/pdfs".into(),
                output_dir: "/tmp/pdfs/".into(),
                watermark_text: "wm".into(),
                watermark_long_edge_font_ratio: 0.028,
                watermark_opacity: 50.0 / 255.0,
                watermark_rotation_degrees: -57.29578,
                watermark_stripe_gap_chars: 2.0,
                watermark_row_gap_lines: 3.0,
            },
            |_| {},
        )
        .expect_err("same directories should fail");

        assert!(err.contains("输入目录与输出目录不能相同"));
    }

    #[test]
    fn batch_pdf_watermark_command_preserves_nested_structure() {
        let temp_dir = TestDir::new();
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");
        let nested_dir = input_dir.join("nested");
        fs::create_dir_all(&nested_dir).expect("nested dir should be created");
        create_test_pdf(&nested_dir.join("demo.pdf")).expect("pdf should be written");

        let result = run_batch_pdf_watermark(
            BatchPdfTextWatermarkInput {
                input_dir: input_dir.to_string_lossy().into_owned(),
                output_dir: output_dir.to_string_lossy().into_owned(),
                watermark_text: "wm".into(),
                watermark_long_edge_font_ratio: 0.028,
                watermark_opacity: 50.0 / 255.0,
                watermark_rotation_degrees: -57.29578,
                watermark_stripe_gap_chars: 2.0,
                watermark_row_gap_lines: 3.0,
            },
            |_| {},
        )
        .expect("batch pdf watermark should succeed");

        assert_eq!(result.scanned_file_count, 1);
        assert_eq!(result.success_count, 1);
        assert_eq!(result.failure_count, 0);
        assert!(output_dir.join("nested/demo-watermarked.pdf").exists());
    }

    #[test]
    fn batch_pdf_watermark_command_reports_progress() {
        let temp_dir = TestDir::new();
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");
        let nested_dir = input_dir.join("nested");
        fs::create_dir_all(&nested_dir).expect("nested dir should be created");
        create_test_pdf(&nested_dir.join("a.pdf")).expect("first pdf should be written");
        create_test_pdf(&nested_dir.join("b.pdf")).expect("second pdf should be written");

        let mut progress_events = Vec::new();
        let result = run_batch_pdf_watermark(
            BatchPdfTextWatermarkInput {
                input_dir: input_dir.to_string_lossy().into_owned(),
                output_dir: output_dir.to_string_lossy().into_owned(),
                watermark_text: "wm".into(),
                watermark_long_edge_font_ratio: 0.028,
                watermark_opacity: 50.0 / 255.0,
                watermark_rotation_degrees: -57.29578,
                watermark_stripe_gap_chars: 2.0,
                watermark_row_gap_lines: 3.0,
            },
            |progress| progress_events.push(progress),
        )
        .expect("batch pdf watermark should succeed");

        assert_eq!(result.scanned_file_count, 2);
        assert_eq!(progress_events.len(), 3);
        assert_eq!(progress_events[0].processed_file_count, 0);
        assert_eq!(progress_events[2].processed_file_count, 2);
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
                watermark_line_count: 3,
                watermark_full_screen: true,
                watermark_opacity: 0.2,
                watermark_stripe_gap_chars: 2.0,
                watermark_row_gap_lines: 3.0,
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
                watermark_line_count: 3,
                watermark_full_screen: true,
                watermark_opacity: 1.2,
                watermark_stripe_gap_chars: 2.0,
                watermark_row_gap_lines: 3.0,
            },
            |_| {},
        )
        .expect_err("opacity greater than one should fail");

        assert!(err.contains("水印透明度"));
    }

    #[test]
    fn batch_image_watermark_command_rejects_negative_spacing() {
        let err = run_batch_image_watermark(
            BatchImageWatermarkInput {
                input_dir: "/tmp/in".into(),
                output_dir: "/tmp/out".into(),
                watermark_text: "wm".into(),
                watermark_line_count: 3,
                watermark_full_screen: true,
                watermark_opacity: 0.2,
                watermark_stripe_gap_chars: -1.0,
                watermark_row_gap_lines: 3.0,
            },
            |_| {},
        )
        .expect_err("negative spacing should fail");

        assert!(err.contains("条间距"));
    }

    #[test]
    fn build_slanted_watermark_options_preserves_rotation_degrees() {
        let options = build_slanted_watermark_options("wm", 3, true, 0.2, -30.0, 2.0, 3.0)
            .expect("options should be built");

        assert_eq!(options.rotation_degrees, -30.0);
    }

    #[test]
    fn render_watermarked_image_respects_rotation_degrees() {
        let temp_dir = TestDir::new();
        let source_path = temp_dir.path().join("source.png");
        create_test_png(&source_path).expect("png should be written");

        let low_rotation = build_slanted_watermark_options("wm", 3, true, 0.2, -20.0, 2.0, 3.0)
            .expect("low rotation options should build");
        let high_rotation =
            build_slanted_watermark_options("wm", 3, true, 0.2, -60.0, 2.0, 3.0)
                .expect("high rotation options should build");

        let low = render_watermarked_image(&source_path, &low_rotation)
            .expect("low rotation render should succeed")
            .to_rgba8();
        let high = render_watermarked_image(&source_path, &high_rotation)
            .expect("high rotation render should succeed")
            .to_rgba8();

        assert_ne!(low, high);
    }

    #[test]
    fn list_previewable_images_returns_sorted_relative_image_paths() {
        let temp_dir = TestDir::new();
        let nested_dir = temp_dir.path().join("nested");
        fs::create_dir_all(&nested_dir).expect("nested dir should be created");
        fs::write(temp_dir.path().join("cover.png"), [1, 2, 3]).expect("png should be written");
        fs::write(nested_dir.join("demo.jpg"), [4, 5, 6]).expect("jpg should be written");
        fs::write(temp_dir.path().join("ignore.txt"), b"noop").expect("txt should be written");

        let files = list_previewable_images(temp_dir.path().to_string_lossy().as_ref())
            .expect("image files should be listed");

        assert_eq!(
            files,
            vec!["cover.png".to_string(), "nested/demo.jpg".to_string()]
        );
    }

    #[test]
    fn generate_preview_image_bytes_rejects_path_escape() {
        let temp_dir = TestDir::new();
        let nested_dir = temp_dir.path().join("nested");
        fs::create_dir_all(&nested_dir).expect("nested dir should be created");
        fs::write(nested_dir.join("demo.jpg"), [4, 5, 6]).expect("jpg should be written");

        let err = generate_preview_image_bytes(BatchImageWatermarkPreviewInput {
            input_dir: temp_dir.path().to_string_lossy().into_owned(),
            relative_path: "../nested/demo.jpg".into(),
            watermark_text: "wm".into(),
            watermark_line_count: 3,
            watermark_full_screen: true,
            watermark_opacity: 0.2,
            watermark_rotation_degrees: -1.0_f32.to_degrees(),
            watermark_stripe_gap_chars: 2.0,
            watermark_row_gap_lines: 3.0,
        })
        .expect_err("path escape should fail");

        assert!(err.contains("预览图片路径不合法") || err.contains("预览图片不存在"));
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let unique_suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("pdf-split-preview-test-{unique_suffix}"));
            fs::create_dir_all(&path).expect("temp dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn create_test_pdf(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, minimal_pdf_bytes())?;
        Ok(())
    }

    fn create_test_png(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let image = kx_image::RgbaImage::from_pixel(
            120,
            80,
            kx_image::Rgba([245_u8, 245_u8, 245_u8, 255_u8]),
        );
        image.save(path)?;
        Ok(())
    }

    fn minimal_pdf_bytes() -> &'static [u8] {
        br#"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Count 1 /Kids [3 0 R] >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj
4 0 obj
<< /Length 40 >>
stream
BT
/F1 24 Tf
72 72 Td
(Hello PDF) Tj
ET
endstream
endobj
5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000241 00000 n 
0000000330 00000 n 
trailer
<< /Size 6 /Root 1 0 R >>
startxref
400
%%EOF
"#
    }
}
