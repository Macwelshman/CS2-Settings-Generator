use cs2_settings_core::{AssetSettingsOverride, GenerationReport, ScanResult, TextureSetOverride};
use std::path::PathBuf;
pub mod updates;

#[tauri::command]
fn choose_export_folder() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("Select Folder to Scan")
        .pick_folder()
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
fn scan_export_folder(
    path: String,
    texture_overrides: Vec<TextureSetOverride>,
    asset_settings_overrides: Vec<AssetSettingsOverride>,
) -> Result<ScanResult, String> {
    cs2_settings_core::scan_export_folder_with_all_overrides(
        &PathBuf::from(path),
        &texture_overrides,
        &asset_settings_overrides,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn generate_settings(
    path: String,
    replace_existing: bool,
    texture_overrides: Vec<TextureSetOverride>,
    asset_settings_overrides: Vec<AssetSettingsOverride>,
) -> Result<GenerationReport, String> {
    cs2_settings_core::generate_settings_files_with_all_overrides(
        &PathBuf::from(path),
        replace_existing,
        &texture_overrides,
        &asset_settings_overrides,
    )
    .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                use tauri::{
                    Emitter,
                    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
                };
                let check = MenuItem::with_id(
                    app,
                    "check-updates",
                    "Check for Updates…",
                    true,
                    None::<&str>,
                )?;
                let submenu = Submenu::with_items(
                    app,
                    "CS2 Settings Generator",
                    true,
                    &[
                        &PredefinedMenuItem::about(app, None, None)?,
                        &check,
                        &PredefinedMenuItem::separator(app)?,
                        &PredefinedMenuItem::quit(app, None)?,
                    ],
                )?;
                let menu = Menu::default(app.handle())?;
                menu.remove_at(0)?;
                menu.prepend(&submenu)?;
                app.set_menu(menu)?;
                app.on_menu_event(|app, event| {
                    if event.id().as_ref() == "check-updates" {
                        let _ = app.emit("check-for-updates", ());
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            choose_export_folder,
            scan_export_folder,
            generate_settings,
            updates::check_for_updates,
            updates::view_update_release,
            updates::install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running CS2 Settings Generator");
}
