use cs2_settings_core::{GenerationReport, ScanResult};
use std::path::PathBuf;

#[tauri::command]
fn choose_export_folder() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("Choose Export Folder")
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
fn scan_export_folder(path: String) -> Result<ScanResult, String> {
    cs2_settings_core::scan_export_folder(&PathBuf::from(path)).map_err(|error| error.to_string())
}

#[tauri::command]
fn generate_settings(path: String, replace_existing: bool) -> Result<GenerationReport, String> {
    cs2_settings_core::generate_settings_files(&PathBuf::from(path), replace_existing)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            choose_export_folder,
            scan_export_folder,
            generate_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running CS2 Settings Generator");
}
