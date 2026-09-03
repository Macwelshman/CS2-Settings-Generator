use crate::model::{GenerationAction, GenerationItem, GenerationReport};
use crate::{
    AssetSettingsOverride, ScanError, TextureSetOverride, scan_export_folder,
    scan_export_folder_with_all_overrides, scan_export_folder_with_overrides,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn generate_settings_files(
    root: &Path,
    replace_existing: bool,
) -> Result<GenerationReport, ScanError> {
    let scan = scan_export_folder(root)?;
    generate_from_scan(scan, replace_existing)
}

pub fn generate_settings_files_with_overrides(
    root: &Path,
    replace_existing: bool,
    texture_overrides: &[TextureSetOverride],
) -> Result<GenerationReport, ScanError> {
    let scan = scan_export_folder_with_overrides(root, texture_overrides)?;
    generate_from_scan(scan, replace_existing)
}

pub fn generate_settings_files_with_all_overrides(
    root: &Path,
    replace_existing: bool,
    texture_overrides: &[TextureSetOverride],
    asset_settings_overrides: &[AssetSettingsOverride],
) -> Result<GenerationReport, ScanError> {
    let scan =
        scan_export_folder_with_all_overrides(root, texture_overrides, asset_settings_overrides)?;
    generate_from_scan(scan, replace_existing)
}

fn generate_from_scan(
    scan: crate::model::ScanResult,
    replace_existing: bool,
) -> Result<GenerationReport, ScanError> {
    let mut items = Vec::new();

    for asset in scan.assets {
        let output_path = asset.settings.output_path.clone();
        if !asset.settings.can_generate {
            items.push(GenerationItem {
                asset_name: asset.name,
                output_path,
                action: GenerationAction::SkippedInvalid,
                message: "Settings were not written because this asset has unresolved errors."
                    .into(),
            });
            continue;
        }
        if asset.settings.existing_file && !replace_existing {
            items.push(GenerationItem {
                asset_name: asset.name,
                output_path,
                action: GenerationAction::SkippedExisting,
                message: "An existing settings.json was preserved.".into(),
            });
            continue;
        }

        let action = if asset.settings.existing_file {
            GenerationAction::Replaced
        } else {
            GenerationAction::Generated
        };
        match write_atomically(
            &output_path,
            asset.settings.json.as_bytes(),
            replace_existing,
        ) {
            Ok(()) => items.push(GenerationItem {
                asset_name: asset.name,
                output_path,
                action,
                message: match action {
                    GenerationAction::Replaced => "Existing settings.json was replaced.".into(),
                    _ => "Settings file was generated.".into(),
                },
            }),
            Err(error) => items.push(GenerationItem {
                asset_name: asset.name,
                output_path,
                action: GenerationAction::Failed,
                message: format!("Could not write settings.json: {error}"),
            }),
        }
    }

    Ok(GenerationReport {
        root: scan.root,
        items,
    })
}

fn write_atomically(path: &Path, contents: &[u8], replace_existing: bool) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let process_id = std::process::id();
    let temp_path = unique_sidecar(parent, &format!(".settings-{process_id}"), "tmp");

    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(contents)?;
        file.sync_all()?;

        if !path.exists() {
            return fs::rename(&temp_path, path);
        }
        if !replace_existing {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "settings.json already exists",
            ));
        }

        let backup_path = unique_sidecar(parent, &format!(".settings-{process_id}"), "backup");
        fs::rename(path, &backup_path)?;
        match fs::rename(&temp_path, path) {
            Ok(()) => {
                fs::remove_file(backup_path)?;
                Ok(())
            }
            Err(error) => {
                let _ = fs::rename(&backup_path, path);
                Err(error)
            }
        }
    })();

    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

fn unique_sidecar(parent: &Path, base: &str, extension: &str) -> PathBuf {
    for index in 0_u32.. {
        let suffix = if index == 0 {
            String::new()
        } else {
            format!("-{index}")
        };
        let candidate = parent.join(format!("{base}{suffix}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("u32 sidecar namespace exhausted")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn preserves_existing_file_without_replace_permission() {
        let folder = temp_folder("preserve");
        fs::create_dir_all(&folder).unwrap();
        let output = folder.join("settings.json");
        fs::write(&output, b"original").unwrap();

        let error = write_atomically(&output, b"replacement", false).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&output).unwrap(), b"original");

        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn replaces_existing_file_when_explicitly_allowed() {
        let folder = temp_folder("replace");
        fs::create_dir_all(&folder).unwrap();
        let output = folder.join("settings.json");
        fs::write(&output, b"original").unwrap();

        write_atomically(&output, b"replacement", true).unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"replacement");

        fs::remove_dir_all(folder).unwrap();
    }

    fn temp_folder(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cs2-settings-{label}-{unique}"))
    }
}
