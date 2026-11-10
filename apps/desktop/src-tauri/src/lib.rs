use cs2_settings_core::{AssetSettingsOverride, GenerationReport, ScanResult, TextureSetOverride};
use std::path::PathBuf;
pub mod updates;

#[tauri::command]
async fn open_project_profile() -> Result<Option<String>, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_title("Import Project Profile")
        .add_filter("Project Profile", &["json"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    std::fs::read_to_string(file.path())
        .map(Some)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_project_profile(contents: String) -> Result<bool, String> {
    use std::io::Write;
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_title("Export Project Profile")
        .set_file_name("cs2-project-profile.json")
        .add_filter("Project Profile", &["json"])
        .save_file()
        .await
    else {
        return Ok(false);
    };
    let path = file.path();
    let mut temporary =
        tempfile::NamedTempFile::new_in(path.parent().ok_or("Invalid export path")?)
            .map_err(|error| error.to_string())?;
    temporary
        .write_all(contents.as_bytes())
        .map_err(|error| error.to_string())?;
    temporary.persist_noclobber(path).map_err(|error| {
        format!("Could not export the profile. Choose an unused filename: {error}")
    })?;
    Ok(true)
}

#[tauri::command]
async fn confirm_profile_replacement(window: tauri::WebviewWindow) -> bool {
    rfd::AsyncMessageDialog::new()
        .set_parent(&window)
        .set_title("Replace saved project choices?")
        .set_description("Importing this profile will replace the choices saved for the selected folder. Asset settings.json files will not change.")
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        .await == rfd::MessageDialogResult::Yes
}

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
    texture_options: Option<cs2_settings_core::TextureOptions>,
) -> Result<ScanResult, String> {
    cs2_settings_core::scan_export_folder_with_texture_options(
        &PathBuf::from(path),
        &texture_overrides,
        &asset_settings_overrides,
        &texture_options.unwrap_or_default(),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn generate_settings(
    path: String,
    replace_existing: bool,
    reviewed: Vec<serde_json::Value>,
    texture_overrides: Vec<TextureSetOverride>,
    asset_settings_overrides: Vec<AssetSettingsOverride>,
    texture_options: Option<cs2_settings_core::TextureOptions>,
) -> Result<GenerationReport, String> {
    let scan = scan_export_folder(
        path,
        texture_overrides,
        asset_settings_overrides,
        texture_options,
    )?;
    let current: Vec<_> = scan
        .assets
        .iter()
        .map(|asset| {
            serde_json::json!({
                "name": asset.name, "folder": asset.folder, "json": asset.settings.json,
                "canGenerate": asset.settings.can_generate,
                "existingFile": asset.settings.existing_file,
                "existingRevision": asset.settings.existing_revision,
            })
        })
        .collect();
    if current != reviewed {
        return Err("The export folder or settings changed since the preview. Rescan and review the changes before generating.".into());
    }
    cs2_settings_core::generate_from_scan(scan, replace_existing).map_err(|error| error.to_string())
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
            open_project_profile,
            export_project_profile,
            confirm_profile_replacement,
            scan_export_folder,
            generate_settings,
            updates::check_for_updates,
            updates::view_update_release,
            updates::install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running CS2 Settings Generator");
}
