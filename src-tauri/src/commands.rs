use std::fs;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use kx_image::{BatchImageWatermarkOptions, BatchImageWatermarkProgress, Imgs};
use kx_pdf::{PdfTextWatermarkOptions, Pdfs};
use tauri::{Emitter, EventTarget, Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::models::{
    BatchImageWatermarkInput, BatchImageWatermarkPreviewInput, BatchImageWatermarkProgressPayload,
    BatchImageWatermarkResult, ExtractImagesResult, InputDirectoryImageListResult,
    PdfTextWatermarkInput, PreviewImageBytesResult, SplitPdfResult, WatermarkPdfResult,
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
    let watermark_long_edge_font_ratio =
        require_percentage("长边字号比例", payload.watermark_long_edge_font_ratio)?;
    let watermark_opacity = require_percentage("水印透明度", payload.watermark_opacity)?;
    let watermark_rotation = require_finite_number("水印角度", payload.watermark_rotation)?;
    let watermark_horizontal_spacing_ratio = require_zero_to_hundred_percentage(
        "横向间距比例",
        payload.watermark_horizontal_spacing_ratio,
    )?;
    let watermark_vertical_spacing_ratio = require_zero_to_hundred_percentage(
        "纵向间距比例",
        payload.watermark_vertical_spacing_ratio,
    )?;
    let options = BatchImageWatermarkOptions {
        watermark_text: &watermark_text,
        long_edge_font_ratio: watermark_long_edge_font_ratio / 100.0,
        opacity: watermark_opacity / 100.0,
        rotation_degrees: watermark_rotation,
        horizontal_spacing_ratio: watermark_horizontal_spacing_ratio / 100.0,
        vertical_spacing_ratio: watermark_vertical_spacing_ratio / 100.0,
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

fn require_zero_to_hundred_percentage(label: &str, value: f32) -> Result<f32, String> {
    if !value.is_finite() || value < 0.0 || value > 100.0 {
        return Err(format!("{label}必须在 0 到 100 之间"));
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

fn generate_preview_image_bytes(
    payload: BatchImageWatermarkPreviewInput,
) -> Result<Vec<u8>, String> {
    let input_dir = require_value("输入目录", payload.input_dir)?;
    let root = PathBuf::from(&input_dir)
        .canonicalize()
        .map_err(|_| "输入目录不存在或不是有效目录".to_string())?;
    let source_path = resolve_preview_image_path(root.as_path(), &payload.relative_path)?;
    let watermark_text = require_value("水印文字", payload.watermark_text)?;
    let watermark_long_edge_font_ratio =
        require_percentage("长边字号比例", payload.watermark_long_edge_font_ratio)?;
    let watermark_opacity = require_percentage("水印透明度", payload.watermark_opacity)?;
    let watermark_rotation = require_finite_number("水印角度", payload.watermark_rotation)?;
    let watermark_horizontal_spacing_ratio = require_zero_to_hundred_percentage(
        "横向间距比例",
        payload.watermark_horizontal_spacing_ratio,
    )?;
    let watermark_vertical_spacing_ratio = require_zero_to_hundred_percentage(
        "纵向间距比例",
        payload.watermark_vertical_spacing_ratio,
    )?;
    let relative_path = source_path
        .strip_prefix(root.as_path())
        .map_err(|err| err.to_string())?
        .to_path_buf();

    let temp_preview_dirs = TempPreviewDirs::new()?;
    let preview_input_path = temp_preview_dirs.input_dir.join(&relative_path);
    if let Some(parent) = preview_input_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::copy(&source_path, &preview_input_path).map_err(|err| err.to_string())?;

    let options = BatchImageWatermarkOptions {
        watermark_text: &watermark_text,
        long_edge_font_ratio: watermark_long_edge_font_ratio / 100.0,
        opacity: watermark_opacity / 100.0,
        rotation_degrees: watermark_rotation,
        horizontal_spacing_ratio: watermark_horizontal_spacing_ratio / 100.0,
        vertical_spacing_ratio: watermark_vertical_spacing_ratio / 100.0,
    };

    Imgs::add_text_watermark_to_images(
        temp_preview_dirs.input_dir.as_path(),
        temp_preview_dirs.output_dir.as_path(),
        &options,
    )
    .map_err(|err| err.to_string())?;

    fs::read(temp_preview_dirs.output_dir.join(relative_path)).map_err(|err| err.to_string())
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
    if !preview_path.starts_with(&root)
        || !preview_path.is_file()
        || !is_preview_image_path(&preview_path)
    {
        return Err("预览图片路径不合法".to_string());
    }

    Ok(preview_path)
}

struct TempPreviewDirs {
    input_dir: PathBuf,
    output_dir: PathBuf,
    root: PathBuf,
}

impl TempPreviewDirs {
    fn new() -> Result<Self, String> {
        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| err.to_string())?
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pdf-split-real-preview-{unique_suffix}"));
        let input_dir = root.join("input");
        let output_dir = root.join("output");
        fs::create_dir_all(&input_dir).map_err(|err| err.to_string())?;
        fs::create_dir_all(&output_dir).map_err(|err| err.to_string())?;

        Ok(Self {
            input_dir,
            output_dir,
            root,
        })
    }
}

impl Drop for TempPreviewDirs {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
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

#[cfg(test)]
mod tests {
    use super::{
        add_text_watermark, extract_embedded_images, generate_preview_image_bytes,
        list_previewable_images, run_batch_image_watermark, split_pdf_to_images,
    };
    use crate::models::{
        BatchImageWatermarkInput, BatchImageWatermarkPreviewInput, PdfTextWatermarkInput,
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
                watermark_long_edge_font_ratio: 2.8,
                watermark_opacity: 18.0,
                watermark_rotation: -35.0,
                watermark_horizontal_spacing_ratio: 18.0,
                watermark_vertical_spacing_ratio: 12.0,
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
                watermark_long_edge_font_ratio: 2.8,
                watermark_opacity: 0.0,
                watermark_rotation: -35.0,
                watermark_horizontal_spacing_ratio: 18.0,
                watermark_vertical_spacing_ratio: 12.0,
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
                watermark_long_edge_font_ratio: 2.8,
                watermark_opacity: 18.0,
                watermark_rotation: -35.0,
                watermark_horizontal_spacing_ratio: 101.0,
                watermark_vertical_spacing_ratio: 12.0,
            },
            |_| {},
        )
        .expect_err("too large spacing should fail");

        assert!(err.contains("横向间距"));
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
            watermark_long_edge_font_ratio: 2.8,
            watermark_opacity: 18.0,
            watermark_rotation: -35.0,
            watermark_horizontal_spacing_ratio: 18.0,
            watermark_vertical_spacing_ratio: 12.0,
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
}
