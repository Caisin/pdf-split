use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use kx_cmds::traits::Cmd;
use kx_cmds::{
    Cmds,
    traits::video::{CmdVideo, SeriesRecutReq},
};
use kx_image::{ImageFormat, Imgs};
use tauri::{Emitter, EventTarget, Manager, WebviewWindow};

use crate::models::{
    BatchImageWatermarkPreviewInput, BatchVideoWatermarkInput, BatchVideoWatermarkProgressPayload,
    BatchVideoWatermarkResult, InputDirectoryVideoListResult, PreviewImageBytesResult,
    SeriesRecutInput, SeriesRecutProgressPayload, SeriesRecutResult,
};

use super::image::render_watermarked_image;
use super::shared::{
    SUPPORTED_VIDEO_EXTENSIONS, absolutize_path, build_slanted_watermark_options,
    canonicalize_existing_directory, ensure_batch_output_directory, ensure_distinct_directories,
    make_temp_preview_dir, require_value,
};

const BATCH_VIDEO_WATERMARK_PROGRESS_EVENT: &str = "batch-video-watermark-progress";
const SERIES_RECUT_PROGRESS_EVENT: &str = "series-recut-progress";

pub fn list_input_directory_videos(
    input_dir: String,
) -> Result<InputDirectoryVideoListResult, String> {
    Ok(InputDirectoryVideoListResult {
        files: list_previewable_videos(&input_dir)?,
    })
}

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
                    failure_count: progress.failure_count,
                    skipped_count: progress.skipped_count,
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

fn run_batch_video_watermark<F>(
    payload: BatchVideoWatermarkInput,
    mut on_progress: F,
) -> Result<BatchVideoWatermarkResult, String>
where
    F: FnMut(BatchVideoWatermarkProgressPayload),
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

    let input_root = canonicalize_existing_directory("输入目录", &input_dir)?;
    let output_root = ensure_batch_output_directory(&input_root, &output_dir)?;
    let source_files = collect_video_files(input_root.as_path())?;
    if source_files.is_empty() {
        return Err("输入目录中没有可处理的视频文件".to_string());
    }

    let cache_dir = output_root.join(".kx-image-video-watermark-cache");
    fs::create_dir_all(&cache_dir).map_err(|err| err.to_string())?;

    on_progress(BatchVideoWatermarkProgressPayload {
        scanned_file_count: source_files.len(),
        processed_file_count: 0,
        success_count: 0,
        failure_count: 0,
        skipped_count: 0,
        generated_overlay_count: 0,
        reused_overlay_count: 0,
        current_file: None,
    });

    let mut result = BatchVideoWatermarkResult {
        scanned_file_count: source_files.len(),
        success_count: 0,
        failure_count: 0,
        skipped_count: 0,
        generated_overlay_count: 0,
        reused_overlay_count: 0,
        output_dir: output_root.to_string_lossy().into_owned(),
    };
    let mut overlay_cache = std::collections::HashMap::<(u32, u32), PathBuf>::new();

    for source_path in source_files {
        let relative_path = source_path
            .strip_prefix(input_root.as_path())
            .map_err(|err| err.to_string())?;
        let relative_display = relative_path.to_string_lossy().replace('\\', "/");
        let output_path = output_root.join(relative_path);

        if output_path.exists() {
            result.skipped_count += 1;
            on_progress(BatchVideoWatermarkProgressPayload {
                scanned_file_count: result.scanned_file_count,
                processed_file_count: result.success_count
                    + result.failure_count
                    + result.skipped_count,
                success_count: result.success_count,
                failure_count: result.failure_count,
                skipped_count: result.skipped_count,
                generated_overlay_count: result.generated_overlay_count,
                reused_overlay_count: result.reused_overlay_count,
                current_file: Some(relative_display.clone()),
            });
            continue;
        }

        if let Some(parent) = output_path.parent()
            && let Err(_error) = fs::create_dir_all(parent)
        {
            result.failure_count += 1;
            on_progress(BatchVideoWatermarkProgressPayload {
                scanned_file_count: result.scanned_file_count,
                processed_file_count: result.success_count
                    + result.failure_count
                    + result.skipped_count,
                success_count: result.success_count,
                failure_count: result.failure_count,
                skipped_count: result.skipped_count,
                generated_overlay_count: result.generated_overlay_count,
                reused_overlay_count: result.reused_overlay_count,
                current_file: Some(relative_display.clone()),
            });
            continue;
        }

        let dimensions =
            match Imgs::video_dimensions(source_path.as_path()).map_err(|err| err.to_string()) {
                Ok(dimensions) => dimensions,
                Err(_) => {
                    result.failure_count += 1;
                    on_progress(BatchVideoWatermarkProgressPayload {
                        scanned_file_count: result.scanned_file_count,
                        processed_file_count: result.success_count
                            + result.failure_count
                            + result.skipped_count,
                        success_count: result.success_count,
                        failure_count: result.failure_count,
                        skipped_count: result.skipped_count,
                        generated_overlay_count: result.generated_overlay_count,
                        reused_overlay_count: result.reused_overlay_count,
                        current_file: Some(relative_display.clone()),
                    });
                    continue;
                }
            };

        let overlay_path = if let Some(path) = overlay_cache.get(&dimensions) {
            result.reused_overlay_count += 1;
            path.clone()
        } else {
            let overlay_path = cache_dir.join(format!("{}x{}.png", dimensions.0, dimensions.1));
            let overlay_image = match Imgs::transparent_from_video_with_slanted_watermark(
                source_path.as_path(),
                &options,
            )
            .map_err(|err| err.to_string())
            {
                Ok(image) => image,
                Err(_) => {
                    result.failure_count += 1;
                    on_progress(BatchVideoWatermarkProgressPayload {
                        scanned_file_count: result.scanned_file_count,
                        processed_file_count: result.success_count
                            + result.failure_count
                            + result.skipped_count,
                        success_count: result.success_count,
                        failure_count: result.failure_count,
                        skipped_count: result.skipped_count,
                        generated_overlay_count: result.generated_overlay_count,
                        reused_overlay_count: result.reused_overlay_count,
                        current_file: Some(relative_display.clone()),
                    });
                    continue;
                }
            };

            if overlay_image
                .save(&overlay_path)
                .map_err(|err| err.to_string())
                .is_err()
            {
                result.failure_count += 1;
                on_progress(BatchVideoWatermarkProgressPayload {
                    scanned_file_count: result.scanned_file_count,
                    processed_file_count: result.success_count
                        + result.failure_count
                        + result.skipped_count,
                    success_count: result.success_count,
                    failure_count: result.failure_count,
                    skipped_count: result.skipped_count,
                    generated_overlay_count: result.generated_overlay_count,
                    reused_overlay_count: result.reused_overlay_count,
                    current_file: Some(relative_display.clone()),
                });
                continue;
            }

            overlay_cache.insert(dimensions, overlay_path.clone());
            result.generated_overlay_count += 1;
            overlay_path
        };

        let ffmpeg_output = match run_video_watermark_ffmpeg_overlay(
            source_path.as_path(),
            output_path.as_path(),
            overlay_path.as_path(),
        ) {
            Ok(output) => output,
            Err(_) => {
                let _ = fs::remove_file(&output_path);
                result.failure_count += 1;
                on_progress(BatchVideoWatermarkProgressPayload {
                    scanned_file_count: result.scanned_file_count,
                    processed_file_count: result.success_count
                        + result.failure_count
                        + result.skipped_count,
                    success_count: result.success_count,
                    failure_count: result.failure_count,
                    skipped_count: result.skipped_count,
                    generated_overlay_count: result.generated_overlay_count,
                    reused_overlay_count: result.reused_overlay_count,
                    current_file: Some(relative_display.clone()),
                });
                continue;
            }
        };

        if !ffmpeg_output.status.success() {
            let _ = fs::remove_file(&output_path);
            result.failure_count += 1;
            on_progress(BatchVideoWatermarkProgressPayload {
                scanned_file_count: result.scanned_file_count,
                processed_file_count: result.success_count
                    + result.failure_count
                    + result.skipped_count,
                success_count: result.success_count,
                failure_count: result.failure_count,
                skipped_count: result.skipped_count,
                generated_overlay_count: result.generated_overlay_count,
                reused_overlay_count: result.reused_overlay_count,
                current_file: Some(relative_display.clone()),
            });
            continue;
        }

        result.success_count += 1;
        on_progress(BatchVideoWatermarkProgressPayload {
            scanned_file_count: result.scanned_file_count,
            processed_file_count: result.success_count
                + result.failure_count
                + result.skipped_count,
            success_count: result.success_count,
            failure_count: result.failure_count,
            skipped_count: result.skipped_count,
            generated_overlay_count: result.generated_overlay_count,
            reused_overlay_count: result.reused_overlay_count,
            current_file: Some(relative_display),
        });
    }

    let _ = fs::remove_dir_all(&cache_dir);
    Ok(result)
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

pub(super) fn list_previewable_videos(input_dir: &str) -> Result<Vec<String>, String> {
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

fn collect_video_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_video_files_recursive(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_video_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_video_files_recursive(path.as_path(), files)?;
            continue;
        }

        if is_preview_video_path(path.as_path()) {
            files.push(path);
        }
    }

    Ok(())
}

fn run_video_watermark_ffmpeg_overlay(
    input_video: &Path,
    output_video: &Path,
    overlay_path: &Path,
) -> Result<std::process::Output, String> {
    Cmds::cmd("ffmpeg")
        .args([
            "-i",
            input_video
                .to_str()
                .ok_or_else(|| "输入视频路径无效".to_string())?,
            "-i",
            overlay_path
                .to_str()
                .ok_or_else(|| "水印图片路径无效".to_string())?,
            "-filter_complex",
            "overlay=0:0",
            "-c:v",
            "libx264",
            "-crf",
            "23",
            "-c:a",
            "copy",
        ])
        .arg(output_video)
        .output()
        .map_err(|err| err.to_string())
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

fn is_preview_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            SUPPORTED_VIDEO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
        .unwrap_or(false)
}

fn extract_video_first_frame(video_path: &Path, output_path: &Path) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let output = Cmds::cmd("ffmpeg")
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
