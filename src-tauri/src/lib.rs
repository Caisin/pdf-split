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
            commands::extract_embedded_images,
            commands::add_text_watermark_to_images
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
