use crate::model::{
    AssetType, Issue, IssueSeverity, SettingsPreview, SharedAssetEntry, TextureSet,
};
use crate::pathing::portable_relative_path;
use std::path::Path;

pub fn build_settings_preview(
    asset_name: &str,
    asset_folder: &Path,
    has_lod1: bool,
    has_lod2: bool,
    main_set: Option<&TextureSet>,
    lod2_set: Option<&TextureSet>,
    asset_type: AssetType,
    normal_opacity: Option<f32>,
    issues: &mut Vec<Issue>,
) -> SettingsPreview {
    let output_path = asset_folder.join("settings.json");
    let existing_file = output_path.is_file();
    let mut entries = Vec::new();

    if let Some(texture_set) = main_set {
        let is_local_exact_set =
            texture_set.folder == asset_folder && texture_set.name == asset_name;
        if !is_local_exact_set {
            append_entries(
                &mut entries,
                asset_name,
                None,
                asset_folder,
                texture_set,
                issues,
            );
        }
        if has_lod1 {
            append_entries(
                &mut entries,
                asset_name,
                Some("LOD1"),
                asset_folder,
                texture_set,
                issues,
            );
        }
    } else if has_lod1 {
        issues.push(Issue::error(
            "mainTextureSetUnresolved",
            "LOD1 must share the main textures, but the main texture set could not be resolved.",
            asset_folder,
        ));
    }

    if has_lod2 {
        if let Some(texture_set) = lod2_set {
            let local_exact = texture_set.folder == asset_folder && texture_set.name == asset_name;
            if !local_exact {
                append_entries(
                    &mut entries,
                    asset_name,
                    Some("LOD2"),
                    asset_folder,
                    texture_set,
                    issues,
                );
            }
        } else {
            issues.push(Issue::warning(
                "lod2TextureSetUnresolved",
                "LOD2 was found, but no local or shared LOD2 texture set could be resolved.",
                asset_folder,
            ));
        }
    }

    let json = render_settings_json(&entries, asset_type, normal_opacity);
    let can_generate = !issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Error);
    SettingsPreview {
        output_path,
        entries,
        json,
        existing_file,
        can_generate,
    }
}

fn append_entries(
    entries: &mut Vec<SharedAssetEntry>,
    asset_name: &str,
    target_lod: Option<&str>,
    asset_folder: &Path,
    texture_set: &TextureSet,
    issues: &mut Vec<Issue>,
) {
    for texture in &texture_set.files {
        let Some(shared_from) = portable_relative_path(asset_folder, &texture.path) else {
            issues.push(Issue::error(
                "relativePathUnavailable",
                format!(
                    "Could not create a portable relative path to {}.",
                    texture.path.display()
                ),
                &texture.path,
            ));
            continue;
        };
        let suffix = texture.kind.suffix();
        let shared_to = match target_lod {
            Some(lod) => format!("{asset_name}_{lod}_{suffix}.png"),
            None => format!("{asset_name}_{suffix}.png"),
        };
        entries.push(SharedAssetEntry {
            shared_to,
            shared_from,
        });
    }
}

fn render_settings_json(
    entries: &[SharedAssetEntry],
    asset_type: AssetType,
    normal_opacity: Option<f32>,
) -> String {
    let mut output = String::from("{\n");
    if asset_type == AssetType::Decal {
        output.push_str("  \"SurfacePostProcessor\": {\n");
        output.push_str("    \"materialTemplate\": \"DefaultDecal\"");
        if let Some(normal_opacity) = normal_opacity {
            let value = if normal_opacity.fract() == 0.0 {
                format!("{normal_opacity:.0}")
            } else {
                serde_json::to_string(&normal_opacity)
                    .expect("finite normal opacity serialisation cannot fail")
            };
            output.push_str(",\n    \"floatProperties\": {\n");
            output.push_str("      \"_NormalOpacity\": ");
            output.push_str(&value);
            output.push_str("\n    }\n");
        } else {
            output.push('\n');
        }
        output.push_str("  },\n");
    }
    output.push_str("  \"sharedAssets\": {");
    if entries.is_empty() {
        output.push_str("\n  }\n}\n");
        return output;
    }
    output.push('\n');
    for (index, entry) in entries.iter().enumerate() {
        let key =
            serde_json::to_string(&entry.shared_to).expect("string serialisation cannot fail");
        let value =
            serde_json::to_string(&entry.shared_from).expect("string serialisation cannot fail");
        output.push_str("    ");
        output.push_str(&key);
        output.push_str(": ");
        output.push_str(&value);
        if index + 1 != entries.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str("  }\n}\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_stable_pretty_json() {
        let json = render_settings_json(
            &[SharedAssetEntry {
                shared_to: "House_LOD1_BaseColor.png".into(),
                shared_from: "House_BaseColor.png".into(),
            }],
            AssetType::Standard,
            None,
        );
        assert_eq!(
            json,
            "{\n  \"sharedAssets\": {\n    \"House_LOD1_BaseColor.png\": \"House_BaseColor.png\"\n  }\n}\n"
        );
    }

    #[test]
    fn combines_decal_import_settings_with_shared_textures() {
        let json = render_settings_json(
            &[SharedAssetEntry {
                shared_to: "Decal_BaseColor.png".into(),
                shared_from: "../Shared/BaseColor.png".into(),
            }],
            AssetType::Decal,
            Some(0.0),
        );
        assert_eq!(
            json,
            "{\n  \"SurfacePostProcessor\": {\n    \"materialTemplate\": \"DefaultDecal\",\n    \"floatProperties\": {\n      \"_NormalOpacity\": 0\n    }\n  },\n  \"sharedAssets\": {\n    \"Decal_BaseColor.png\": \"../Shared/BaseColor.png\"\n  }\n}\n"
        );
    }
}
