use crate::fbx::{classify_fbx, inspect_fbx};
use crate::model::{
    AssetScan, AssetSettingsOverride, AssetType, FbxFile, FbxKind, Issue, ScanResult, TextureKind,
    TextureSet, TextureSetOverride, TextureTier,
};
use crate::settings::build_settings_preview;
use crate::texture::collect_texture_sets;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ScanError {
    message: String,
}

impl ScanError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ScanError {}

pub fn scan_export_folder(root: &Path) -> Result<ScanResult, ScanError> {
    scan_export_folder_with_all_overrides(root, &[], &[])
}

pub fn scan_export_folder_with_overrides(
    root: &Path,
    texture_overrides: &[TextureSetOverride],
) -> Result<ScanResult, ScanError> {
    scan_export_folder_with_all_overrides(root, texture_overrides, &[])
}

pub fn scan_export_folder_with_all_overrides(
    root: &Path,
    texture_overrides: &[TextureSetOverride],
    asset_settings_overrides: &[AssetSettingsOverride],
) -> Result<ScanResult, ScanError> {
    let root = fs::canonicalize(root).map_err(|error| {
        ScanError::new(format!(
            "Could not open export folder {}: {error}",
            root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(ScanError::new(format!(
            "The selected export path is not a folder: {}",
            root.display()
        )));
    }

    let mut fbx_paths = Vec::new();
    let mut png_paths = Vec::new();
    let mut global_issues = Vec::new();

    for entry in WalkDir::new(&root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                global_issues.push(Issue::warning(
                    "folderEntryUnreadable",
                    format!("A folder entry could not be read: {error}"),
                    &root,
                ));
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        match entry
            .path()
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .as_deref()
        {
            Some("fbx") => fbx_paths.push(entry.path().to_path_buf()),
            Some("png") => png_paths.push(entry.path().to_path_buf()),
            _ => {}
        }
    }
    fbx_paths.sort();
    png_paths.sort();

    let (mut texture_sets, texture_issues) = collect_texture_sets(&png_paths);
    global_issues.extend(texture_issues);

    let mut grouped: BTreeMap<(PathBuf, String), Vec<(PathBuf, FbxKind)>> = BTreeMap::new();
    for path in fbx_paths {
        let Some(stem) = path.file_stem().map(|value| value.to_string_lossy()) else {
            continue;
        };
        let (asset_name, kind) = classify_fbx(&stem);
        let folder = path.parent().unwrap_or(&root).to_path_buf();
        grouped
            .entry((folder, asset_name.to_owned()))
            .or_default()
            .push((path, kind));
    }

    let main_assets = grouped
        .iter()
        .filter(|(_, files)| files.iter().any(|(_, kind)| *kind == FbxKind::Main))
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();

    let asset_folders = main_assets
        .iter()
        .map(|(folder, _)| folder.clone())
        .collect::<BTreeSet<_>>();
    texture_sets = retain_importable_texture_sets(texture_sets, &asset_folders, &mut global_issues);

    for ((folder, asset_name), files) in &grouped {
        if !files.iter().any(|(_, kind)| *kind == FbxKind::Main) {
            global_issues.push(Issue::warning(
                "orphanFbxVariant",
                format!("FBX variants for “{asset_name}” were found without a matching main FBX."),
                folder,
            ));
        }
    }

    let mut assets = Vec::new();
    for (folder, asset_name) in main_assets {
        let variants = grouped
            .remove(&(folder.clone(), asset_name.clone()))
            .unwrap_or_default();
        let mut issues = Vec::new();
        let mut files = Vec::<FbxFile>::new();
        for (path, kind) in variants {
            let (file, file_issues) = inspect_fbx(&path, kind);
            files.push(file);
            issues.extend(file_issues);
        }
        files.sort_by_key(|file| (file.kind, file.path.clone()));

        let asset_settings_override = asset_settings_overrides
            .iter()
            .find(|asset_override| asset_override.asset_folder == folder);
        let asset_type = asset_settings_override
            .map(|asset_override| asset_override.asset_type)
            .unwrap_or(AssetType::Standard);
        let normal_opacity =
            asset_settings_override.and_then(|asset_override| asset_override.normal_opacity);

        if let Some(value) = normal_opacity
            && !(0.0..=1.0).contains(&value)
        {
            issues.push(Issue::error(
                "normalOpacityOutOfRange",
                "Decal normal opacity must be between 0 and 1.",
                &folder,
            ));
        }

        let has_lod1 = files.iter().any(|file| file.kind.is_lod1());
        let has_lod2 = files.iter().any(|file| file.kind.is_lod2());
        if asset_type == AssetType::Standard && !has_lod1 {
            issues.push(Issue::warning(
                "lod1Missing",
                "No LOD1 FBX was found. CS2 building assets require an LOD1 mesh.",
                &folder,
            ));
        }
        let main_material = files
            .iter()
            .find(|file| file.kind == FbxKind::Main)
            .and_then(|file| file.material_names.first().cloned());

        let main_texture_set = resolve_main_texture_set(
            &asset_name,
            &folder,
            main_material.as_deref(),
            &texture_sets,
            texture_overrides
                .iter()
                .find(|texture_override| texture_override.asset_folder == folder),
            &mut issues,
        );
        let lod2_texture_set = resolve_lod2_texture_set(
            &asset_name,
            &folder,
            main_texture_set.as_ref(),
            &texture_sets,
            has_lod2,
            &mut issues,
        );

        if asset_type == AssetType::Decal {
            validate_decal_texture_set(main_texture_set.as_ref(), &folder, &mut issues);
        }

        let settings = build_settings_preview(
            &asset_name,
            &folder,
            has_lod1,
            has_lod2,
            main_texture_set.as_ref(),
            lod2_texture_set.as_ref(),
            asset_type,
            normal_opacity,
            &mut issues,
        );

        assets.push(AssetScan {
            name: asset_name,
            folder,
            files,
            main_texture_set,
            lod2_texture_set,
            asset_type,
            normal_opacity,
            settings,
            issues,
        });
    }
    assets.sort_by(|left, right| left.folder.cmp(&right.folder));

    Ok(ScanResult {
        root,
        assets,
        texture_sets,
        global_issues,
    })
}

fn retain_importable_texture_sets(
    texture_sets: Vec<TextureSet>,
    asset_folders: &BTreeSet<PathBuf>,
    issues: &mut Vec<Issue>,
) -> Vec<TextureSet> {
    texture_sets
        .into_iter()
        .filter(|texture_set| {
            if asset_folders.contains(&texture_set.folder) {
                return true;
            }
            issues.push(Issue::warning(
                "textureSetOutsideAssetFolder",
                format!(
                    "Texture set “{}” is not inside an importable asset folder and cannot be used by CS2.",
                    texture_set.name
                ),
                &texture_set.folder,
            ));
            false
        })
        .collect()
}

fn validate_decal_texture_set(
    texture_set: Option<&TextureSet>,
    asset_folder: &Path,
    issues: &mut Vec<Issue>,
) {
    let Some(texture_set) = texture_set else {
        issues.push(Issue::error(
            "decalTextureSetUnresolved",
            "Decals require a BaseColor, MaskMap, and Normal texture set.",
            asset_folder,
        ));
        return;
    };

    for (kind, label) in [
        (TextureKind::BaseColor, "BaseColor"),
        (TextureKind::MaskMap, "MaskMap"),
        (TextureKind::Normal, "Normal"),
    ] {
        if !texture_set.files.iter().any(|file| file.kind == kind) {
            issues.push(Issue::error(
                "decalRequiredTextureMissing",
                format!(
                    "Decal texture set “{}” is missing {label}.",
                    texture_set.name
                ),
                asset_folder,
            ));
        }
    }
}

fn resolve_main_texture_set(
    asset_name: &str,
    asset_folder: &Path,
    material_name: Option<&str>,
    texture_sets: &[TextureSet],
    texture_override: Option<&TextureSetOverride>,
    issues: &mut Vec<Issue>,
) -> Option<TextureSet> {
    if let Some(texture_override) = texture_override {
        if let Some(texture_set) = texture_sets.iter().find(|texture_set| {
            texture_set.tier == TextureTier::Main
                && texture_set.folder == texture_override.texture_set_folder
                && texture_set.name == texture_override.texture_set_name
        }) {
            return Some(texture_set.clone());
        }
        issues.push(Issue::warning(
            "mainTextureOverrideUnavailable",
            format!(
                "The selected texture set “{}” is no longer available; automatic resolution was used instead.",
                texture_override.texture_set_name
            ),
            asset_folder,
        ));
    }

    if let Some(material_name) = material_name {
        let material_base = material_name.strip_suffix("_Mtl").unwrap_or(material_name);
        let matches = matching_sets(texture_sets, material_base, TextureTier::Main);
        if matches.len() == 1 {
            return Some(matches[0].clone());
        }
        if matches.len() > 1 {
            issues.push(Issue::error(
                "mainTextureSetAmbiguous",
                format!(
                    "Material “{material_name}” matches multiple texture sets; choose the intended set."
                ),
                asset_folder,
            ));
            return None;
        }
    }

    issues.push(Issue::error(
        "mainTextureSetUnresolved",
        match material_name {
            Some(material) => format!(
                "No texture set matching FBX material “{material}” was found in an asset folder. Add the matching textures or explicitly select a texture set."
            ),
            None => format!(
                "“{asset_name}” has no readable main FBX material name. A texture set cannot be matched automatically."
            ),
        },
        asset_folder,
    ));
    None
}

fn resolve_lod2_texture_set(
    asset_name: &str,
    asset_folder: &Path,
    main_set: Option<&TextureSet>,
    texture_sets: &[TextureSet],
    has_lod2: bool,
    issues: &mut Vec<Issue>,
) -> Option<TextureSet> {
    if !has_lod2 {
        return None;
    }

    let local = matching_sets(texture_sets, asset_name, TextureTier::Lod2)
        .into_iter()
        .filter(|set| set.folder == asset_folder)
        .collect::<Vec<_>>();
    if local.len() == 1 {
        return Some(local[0].clone());
    }

    if let Some(main_set) = main_set {
        let matching_provider = matching_sets(texture_sets, &main_set.name, TextureTier::Lod2);
        if matching_provider.len() == 1 {
            return Some(matching_provider[0].clone());
        }
    }

    let all_lod2 = texture_sets
        .iter()
        .filter(|set| set.tier == TextureTier::Lod2)
        .cloned()
        .collect::<Vec<_>>();
    if all_lod2.len() == 1 {
        return Some(all_lod2[0].clone());
    }
    if all_lod2.len() > 1 {
        issues.push(Issue::warning(
            "lod2TextureSetAmbiguous",
            "Multiple LOD2 texture sets are available; choose the intended shared set.",
            asset_folder,
        ));
    }
    None
}

fn matching_sets(texture_sets: &[TextureSet], name: &str, tier: TextureTier) -> Vec<TextureSet> {
    texture_sets
        .iter()
        .filter(|set| set.name == name && set.tier == tier)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_materials_resolve_local_shared_and_missing_sets() {
        let provider = PathBuf::from("/export/House 1");
        let mut set = texture_set("House 1", &provider);
        set.files.push(crate::model::TextureFile {
            path: provider.join("House 1_BaseColor.png"),
            kind: TextureKind::BaseColor,
            width: Some(1024),
            height: Some(1024),
        });
        let sets = vec![set];
        for (name, material, expected) in [
            ("House 1", "House 1", true),
            ("House 2", "House 1", true),
            ("House 3", "House 3", false),
        ] {
            let folder = PathBuf::from("/export").join(name);
            let mut issues = Vec::new();
            let resolved =
                resolve_main_texture_set(name, &folder, Some(material), &sets, None, &mut issues);
            assert_eq!(resolved.is_some(), expected);
            let preview = build_settings_preview(
                name,
                &folder,
                true,
                false,
                resolved.as_ref(),
                None,
                AssetType::Standard,
                None,
                &mut issues,
            );
            assert_eq!(preview.can_generate, expected);
            if name == "House 2" {
                assert!(preview.json.contains("House 2_BaseColor.png"));
                assert!(preview.json.contains("House 2_LOD1_BaseColor.png"));
                assert!(preview.json.contains("../House 1/House 1_BaseColor.png"));
            }
        }
    }

    #[test]
    fn explicit_override_selects_a_different_shared_texture_set() {
        let asset_folder = PathBuf::from("/export/SDNH Ambulance Sign");
        let sign_folder = PathBuf::from("/export/SDNH Sign");
        let texture_sets = vec![texture_set("SDNH Sign", &sign_folder)];
        let texture_override = TextureSetOverride {
            asset_folder: asset_folder.clone(),
            texture_set_folder: sign_folder,
            texture_set_name: "SDNH Sign".into(),
        };
        let mut issues = Vec::new();

        let resolved = resolve_main_texture_set(
            "SDNH Ambulance Sign",
            &asset_folder,
            Some("SDNH Signs"),
            &texture_sets,
            Some(&texture_override),
            &mut issues,
        )
        .expect("manual override should resolve");

        assert_eq!(resolved.name, "SDNH Sign");
        assert!(issues.is_empty());
    }

    #[test]
    fn material_match_wins_over_asset_named_local_textures() {
        let crematorium_folder = PathBuf::from("/export/San Diego Crematorium");
        let hospital_folder = PathBuf::from("/export/San Diego Naval Hospital");
        let texture_sets = vec![
            texture_set("San Diego Crematorium", &crematorium_folder),
            texture_set("San Diego Naval Hospital", &hospital_folder),
        ];
        let mut issues = Vec::new();

        let resolved = resolve_main_texture_set(
            "San Diego Crematorium",
            &crematorium_folder,
            Some("San Diego Naval Hospital"),
            &texture_sets,
            None,
            &mut issues,
        )
        .expect("material texture set should resolve");

        assert_eq!(resolved.name, "San Diego Naval Hospital");
        assert!(issues.is_empty());
    }

    #[test]
    fn unrelated_local_texture_set_does_not_override_material_match() {
        let asset_folder = PathBuf::from("/export/House A");
        let other_folder = PathBuf::from("/export/House B");
        let texture_sets = vec![
            texture_set("Custom Brick", &asset_folder),
            texture_set("House A Material", &other_folder),
        ];
        let mut issues = Vec::new();

        let resolved = resolve_main_texture_set(
            "House A",
            &asset_folder,
            Some("House A Material"),
            &texture_sets,
            None,
            &mut issues,
        )
        .expect("the material name should match the remote provider");

        assert_eq!(resolved.name, "House A Material");
        assert_eq!(resolved.folder, other_folder);
        assert!(issues.is_empty());
    }

    #[test]
    fn sole_unrelated_texture_set_does_not_resolve_missing_material_textures() {
        let asset_folder = PathBuf::from("/export/House A");
        let provider_folder = PathBuf::from("/export/Texture Provider");
        let texture_sets = vec![texture_set("Shared Brick", &provider_folder)];
        let mut issues = Vec::new();

        let resolved = resolve_main_texture_set(
            "House A",
            &asset_folder,
            Some("House A Material"),
            &texture_sets,
            None,
            &mut issues,
        );

        assert!(resolved.is_none());
        assert_eq!(issues[0].code, "mainTextureSetUnresolved");
        assert_eq!(issues[0].severity, crate::model::IssueSeverity::Error);
    }

    #[test]
    fn multiple_importable_texture_sets_require_an_explicit_choice() {
        let asset_folder = PathBuf::from("/export/House A");
        let provider_folder = PathBuf::from("/export/Texture Provider");
        let texture_sets = vec![
            texture_set("Shared Brick", &provider_folder),
            texture_set("Shared Stone", &provider_folder),
        ];
        let mut issues = Vec::new();

        let resolved = resolve_main_texture_set(
            "House A",
            &asset_folder,
            Some("House A Material"),
            &texture_sets,
            None,
            &mut issues,
        );

        assert!(resolved.is_none());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "mainTextureSetUnresolved");
    }

    #[test]
    fn texture_sets_outside_asset_folders_are_not_available() {
        let export_root = PathBuf::from("/export");
        let provider_folder = PathBuf::from("/export/Texture Provider");
        let asset_folders = BTreeSet::from([provider_folder.clone()]);
        let texture_sets = vec![
            texture_set("Loose Root Textures", &export_root),
            texture_set("Imported Textures", &provider_folder),
        ];
        let mut issues = Vec::new();

        let retained = retain_importable_texture_sets(texture_sets, &asset_folders, &mut issues);

        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].name, "Imported Textures");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "textureSetOutsideAssetFolder");
    }

    #[test]
    fn decal_requires_base_color_mask_map_and_normal() {
        let folder = PathBuf::from("/export/Decal");
        let mut set = texture_set("Decal", &folder);
        set.files.push(crate::model::TextureFile {
            path: folder.join("Decal_BaseColor.png"),
            kind: TextureKind::BaseColor,
            width: Some(512),
            height: Some(512),
        });
        let mut issues = Vec::new();

        validate_decal_texture_set(Some(&set), &folder, &mut issues);

        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.code == "decalRequiredTextureMissing")
                .count(),
            2
        );
        assert!(issues.iter().any(|issue| issue.message.contains("MaskMap")));
        assert!(issues.iter().any(|issue| issue.message.contains("Normal")));
    }

    fn texture_set(name: &str, folder: &Path) -> TextureSet {
        TextureSet {
            name: name.into(),
            tier: TextureTier::Main,
            folder: folder.into(),
            files: Vec::new(),
        }
    }
}
