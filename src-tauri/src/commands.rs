mod dialog;
mod image;
mod pdf;
mod shared;
#[cfg(test)]
mod tests;
mod video;

use tauri::WebviewWindow;

use crate::models::{
    BatchImageWatermarkInput, BatchImageWatermarkPreviewInput, BatchPdfTextWatermarkInput,
    BatchPdfWatermarkPreviewInput, BatchVideoWatermarkInput, ExtractImagesResult,
    InputDirectoryImageListResult, InputDirectoryPdfListResult, InputDirectoryVideoListResult,
    PdfTextWatermarkInput, PreviewImageBytesResult, SeriesRecutInput, SeriesRecutResult,
    SplitPdfResult, WatermarkPdfResult,
};

#[tauri::command]
pub fn select_pdf_file(window: WebviewWindow) -> Result<Option<String>, String> {
    dialog::select_pdf_file(window)
}

#[tauri::command]
pub fn select_output_dir(window: WebviewWindow) -> Result<Option<String>, String> {
    dialog::select_output_dir(window)
}

#[tauri::command]
pub fn split_pdf_to_images(
    input_path: String,
    output_dir: String,
    image_format: String,
) -> Result<SplitPdfResult, String> {
    pdf::split_pdf_to_images(input_path, output_dir, image_format)
}

#[tauri::command]
pub fn add_text_watermark(payload: PdfTextWatermarkInput) -> Result<WatermarkPdfResult, String> {
    pdf::add_text_watermark(payload)
}

#[tauri::command]
pub async fn add_text_watermark_to_pdfs(
    window: WebviewWindow,
    payload: BatchPdfTextWatermarkInput,
) -> Result<crate::models::BatchPdfWatermarkResult, String> {
    pdf::add_text_watermark_to_pdfs(window, payload).await
}

#[tauri::command]
pub fn list_input_directory_pdfs(input_dir: String) -> Result<InputDirectoryPdfListResult, String> {
    pdf::list_input_directory_pdfs(input_dir)
}

#[tauri::command]
pub async fn generate_input_directory_pdf_preview(
    payload: BatchPdfWatermarkPreviewInput,
) -> Result<PreviewImageBytesResult, String> {
    pdf::generate_input_directory_pdf_preview(payload).await
}

#[tauri::command]
pub fn extract_embedded_images(
    input_path: String,
    output_dir: String,
) -> Result<ExtractImagesResult, String> {
    pdf::extract_embedded_images(input_path, output_dir)
}

#[tauri::command]
pub async fn add_text_watermark_to_images(
    window: WebviewWindow,
    payload: BatchImageWatermarkInput,
) -> Result<crate::models::BatchImageWatermarkResult, String> {
    image::add_text_watermark_to_images(window, payload).await
}

#[tauri::command]
pub fn list_input_directory_images(
    input_dir: String,
) -> Result<InputDirectoryImageListResult, String> {
    image::list_input_directory_images(input_dir)
}

#[tauri::command]
pub async fn generate_input_directory_image_preview(
    payload: BatchImageWatermarkPreviewInput,
) -> Result<PreviewImageBytesResult, String> {
    image::generate_input_directory_image_preview(payload).await
}

#[tauri::command]
pub fn list_input_directory_videos(
    input_dir: String,
) -> Result<InputDirectoryVideoListResult, String> {
    video::list_input_directory_videos(input_dir)
}

#[tauri::command]
pub async fn generate_input_directory_video_preview(
    payload: BatchImageWatermarkPreviewInput,
) -> Result<PreviewImageBytesResult, String> {
    video::generate_input_directory_video_preview(payload).await
}

#[tauri::command]
pub async fn add_slanted_watermark_to_videos(
    window: WebviewWindow,
    payload: BatchVideoWatermarkInput,
) -> Result<crate::models::BatchVideoWatermarkResult, String> {
    video::add_slanted_watermark_to_videos(window, payload).await
}

#[tauri::command]
pub async fn video_recut_series(
    window: WebviewWindow,
    payload: SeriesRecutInput,
) -> Result<SeriesRecutResult, String> {
    video::video_recut_series(window, payload).await
}
