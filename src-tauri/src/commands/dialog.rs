use tauri::WebviewWindow;
use tauri_plugin_dialog::DialogExt;

use super::shared::dialog_path_to_string;

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

pub fn select_output_dir(window: WebviewWindow) -> Result<Option<String>, String> {
    let folder = window.dialog().file().blocking_pick_folder();

    match folder {
        Some(folder) => Ok(Some(dialog_path_to_string(folder)?)),
        None => Ok(None),
    }
}
