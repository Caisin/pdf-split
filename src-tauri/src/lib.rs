mod commands;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::select_pdf_file,
            commands::select_output_dir,
            commands::split_pdf_to_images,
            commands::add_text_watermark,
            commands::add_text_watermark_to_pdfs,
            commands::list_input_directory_pdfs,
            commands::generate_input_directory_pdf_preview,
            commands::extract_embedded_images,
            commands::add_text_watermark_to_images,
            commands::list_input_directory_images,
            commands::generate_input_directory_image_preview,
            commands::list_input_directory_videos,
            commands::generate_input_directory_video_preview,
            commands::add_slanted_watermark_to_videos,
            commands::video_recut_series
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
